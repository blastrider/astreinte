#![forbid(unsafe_code)]
use anyhow::{bail, Result};
use astreinte::{
    io,
    model::{Person, ShiftId},
    notification::{prepare_reminder, TextReminder},
    scheduler::{AssignOptions, ConflictKind, Scheduler},
    storage::{JsonStorage, Storage},
};
use chrono::Utc;
use clap::{Parser, Subcommand};
#[cfg(feature = "logging")]
use tracing_subscriber::{fmt::Subscriber, EnvFilter};

/// CLI minimaliste d'astreinte (sans base de données)
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// Active les logs (feature `logging`)
    #[arg(long, global = true)]
    log: bool,

    /// Fichier JSON de roster
    #[arg(long, global = true, default_value = "roster.json")]
    roster: String,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Créer un shift
    CreateShift {
        #[arg(long)]
        name: String,
        /// RFC3339 UTC
        #[arg(long)]
        start: String,
        /// RFC3339 UTC
        #[arg(long)]
        end: String,
    },

    /// Importer des personnes depuis un CSV
    ImportPeople {
        #[arg(long)]
        csv: String,
    },

    /// Importer des shifts depuis un CSV
    ImportShifts {
        #[arg(long)]
        csv: String,
    },

    /// Assigner en round-robin
    Assign {
        /// liste "handle1,handle2,..."
        #[arg(long)]
        people: Option<String>,
        #[arg(long, default_value_t = 11)]
        min_rest_hours: u32,
        #[arg(long, default_value_t = 3)]
        max_consecutive_shifts: u32,
    },

    /// Lister et optionnellement exporter
    List {
        #[arg(long)]
        out_json: Option<String>,
        #[arg(long)]
        out_csv: Option<String>,
    },

    /// Échanger l'assignation d'un shift entre deux personnes
    Swap {
        #[arg(long)]
        shift_id: String,
        #[arg(long)]
        person: String,
        #[arg(long)]
        with: String,
    },

    /// Couvrir la fin d'un shift à partir d'une date donnée
    Cover {
        #[arg(long)]
        shift_id: String,
        /// Point de reprise (RFC3339 UTC) à l'intérieur du shift
        #[arg(long)]
        from: String,
        #[arg(long)]
        with: String,
        #[arg(long, default_value_t = 11)]
        min_rest_hours: u32,
        #[arg(long, default_value_t = 3)]
        max_consecutive_shifts: u32,
    },

    /// Vérifier les conflits
    Check {
        #[arg(long, default_value_t = 11)]
        min_rest_hours: u32,
        #[arg(long, default_value_t = 3)]
        max_consecutive_shifts: u32,
        /// Export CSV des conflits (optionnel)
        #[arg(long)]
        report: Option<String>,
    },

    /// Générer un rappel texte pour un membre d'astreinte
    Notify {
        #[arg(long)]
        handle: String,
        #[arg(long, default_value_t = 2)]
        days_before: i64,
        /// Fichier de sortie (texte brut)
        #[arg(long)]
        out: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    #[cfg(feature = "logging")]
    if cli.log {
        let _ = Subscriber::builder()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
    }

    let storage = JsonStorage::open(&cli.roster)?;
    let mut scheduler = match storage.load() {
        Ok(r) => {
            let mut s = Scheduler::new();
            *s.roster_mut() = r;
            s
        }
        Err(_) => Scheduler::new(),
    };

    let code = match cli.cmd {
        Commands::CreateShift { name, start, end } => {
            let start = start.parse()?;
            let end = end.parse()?;
            scheduler.create_shift(&name, start, end)?;
            storage.save(scheduler.roster())?;
            0
        }
        Commands::ImportPeople { csv } => {
            let people = io::import_people_csv(csv)?;
            scheduler.add_people(people);
            storage.save(scheduler.roster())?;
            0
        }
        Commands::ImportShifts { csv } => {
            let shifts = io::import_shifts_csv(csv)?;
            scheduler.roster_mut().shifts.extend(shifts);
            storage.save(scheduler.roster())?;
            0
        }
        Commands::Assign {
            people,
            min_rest_hours,
            max_consecutive_shifts,
        } => {
            let opts = AssignOptions {
                min_rest_hours,
                max_consecutive_shifts,
            };
            let mut persons: Vec<Person> = if let Some(list) = people {
                let set: Vec<String> = list
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let mut out = Vec::new();
                for h in set {
                    if let Some(p) = scheduler.roster().people.iter().find(|p| p.handle == h) {
                        out.push(p.clone());
                    }
                }
                out
            } else {
                scheduler.roster().people.clone()
            };
            persons.retain(|p| !p.on_vacation);
            if persons.is_empty() {
                bail!("aucune personne disponible (vacances ou indisponibilités)");
            }
            scheduler.assign_rotative(&persons, opts)?;
            storage.save(scheduler.roster())?;
            0
        }
        Commands::List { out_json, out_csv } => {
            if let Some(path) = out_json {
                io::export_roster_json(path, scheduler.roster())?;
            }
            if let Some(path) = out_csv {
                io::export_shifts_csv(path, scheduler.roster())?;
            }
            // impression compacte
            for s in &scheduler.roster().shifts {
                let assigned = s
                    .assigned
                    .as_ref()
                    .and_then(|pid| scheduler.roster().people.iter().find(|p| p.id == *pid))
                    .map(|p| p.handle.as_str())
                    .unwrap_or("-");
                println!(
                    "{} | {} → {} | {}",
                    s.id.as_str(),
                    s.start.to_rfc3339(),
                    s.end.to_rfc3339(),
                    assigned
                );
            }
            0
        }
        Commands::Swap {
            shift_id,
            person,
            with,
        } => {
            let sid = ShiftId::new(shift_id);
            let pa = scheduler
                .roster()
                .find_person_by_handle(&person)
                .map(|p| p.id.clone())
                .ok_or_else(|| anyhow::anyhow!("unknown person: {}", person))?;
            let pb = scheduler
                .roster()
                .find_person_by_handle(&with)
                .map(|p| p.id.clone())
                .ok_or_else(|| anyhow::anyhow!("unknown person: {}", with))?;
            scheduler.swap(&sid, &pa, &pb, AssignOptions::default())?;
            storage.save(scheduler.roster())?;
            0
        }
        Commands::Cover {
            shift_id,
            from,
            with,
            min_rest_hours,
            max_consecutive_shifts,
        } => {
            let sid = ShiftId::new(shift_id);
            let at = from.parse()?;
            let cover_id = scheduler
                .roster()
                .find_person_by_handle(&with)
                .map(|p| p.id.clone())
                .ok_or_else(|| anyhow::anyhow!("unknown person: {}", with))?;
            let opts = AssignOptions {
                min_rest_hours,
                max_consecutive_shifts,
            };
            scheduler.cover_shift(&sid, at, &cover_id, opts)?;
            storage.save(scheduler.roster())?;
            0
        }
        Commands::Check {
            min_rest_hours,
            max_consecutive_shifts,
            report,
        } => {
            let opts = AssignOptions {
                min_rest_hours,
                max_consecutive_shifts,
            };
            let conflicts = scheduler.detect_conflicts(opts);
            if conflicts.is_empty() {
                println!("OK: no conflicts");
                0
            } else {
                eprintln!("Found {} conflict(s)", conflicts.len());
                if let Some(path) = report {
                    // CSV simple
                    let mut w = csv::Writer::from_path(path)?;
                    w.write_record(["person_id", "shift_a", "shift_b", "kind"])?;
                    for c in &conflicts {
                        w.write_record([
                            c.person.as_str(),
                            c.shift_a.as_str(),
                            c.shift_b.as_str(),
                            match c.kind {
                                ConflictKind::Overlap => "overlap",
                                ConflictKind::DoubleAssignment => "double",
                                ConflictKind::RestViolation => "rest",
                            },
                        ])?;
                    }
                    w.flush()?;
                }
                // Code 2 = WARNING/INCOMPLETE
                2
            }
        }
        Commands::Notify {
            handle,
            days_before,
            out,
        } => {
            let renderer = TextReminder;
            let reminder = prepare_reminder(
                scheduler.roster(),
                &handle,
                days_before,
                Utc::now(),
                &renderer,
            )?;
            std::fs::write(&out, reminder.content)?;
            println!(
                "Reminder generated for {} (shift {}) at {}",
                reminder.person_handle,
                reminder.shift_id,
                reminder.notice_at.to_rfc3339()
            );
            0
        }
    };

    std::process::exit(code);
}
