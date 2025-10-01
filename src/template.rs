use crate::io;
use crate::model::{Roster, Shift};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Description complète d'un template de rotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub rotation_cycle_days: u16,
    #[serde(default)]
    pub slots: Vec<Slot>,
    #[serde(default)]
    pub rules: Option<Rules>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

impl Template {
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            bail!("template id cannot be empty");
        }
        if self.name.trim().is_empty() {
            bail!("template name cannot be empty");
        }
        if self.rotation_cycle_days == 0 {
            bail!("rotation_cycle_days must be > 0");
        }
        if self.slots.is_empty() {
            bail!("template must contain at least one slot");
        }
        for slot in &self.slots {
            slot.validate()?;
        }
        validate_slot_overlaps(&self.slots)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub role: String,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub days: Vec<u8>,
    #[serde(default)]
    pub priority: u8,
}

impl Slot {
    fn validate(&self) -> Result<()> {
        if self.role.trim().is_empty() {
            bail!("slot role cannot be empty");
        }
        if self.days.is_empty() {
            bail!("slot must define at least one day");
        }
        if self.start_time == self.end_time {
            bail!("slot start_time and end_time cannot be equal");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rules {
    #[serde(default)]
    pub min_rest_hours: Option<u16>,
    #[serde(default)]
    pub max_consecutive_days: Option<u8>,
    #[serde(default)]
    pub allow_weekend_swap: bool,
}

#[derive(Debug, Clone)]
pub struct TemplateInfo {
    pub template: Template,
    pub path: PathBuf,
    pub modified: Option<DateTime<Utc>>,
}

/// Gestion simple des templates persistés sur disque.
#[derive(Debug, Clone)]
pub struct TemplateStore {
    base_dir: PathBuf,
}

impl TemplateStore {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            base_dir: dir.as_ref().to_path_buf(),
        }
    }

    fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.base_dir)
            .with_context(|| format!("creating template directory {}", self.base_dir.display()))
    }

    pub fn save(&self, template: &Template) -> Result<PathBuf> {
        template.validate()?;
        self.ensure_dir()?;
        let path = self.base_dir.join(format!("{}.json", template.id));
        let json = serde_json::to_string_pretty(template)?;
        fs::write(&path, json).with_context(|| format!("writing template {}", path.display()))?;
        Ok(path)
    }

    pub fn load(&self, id: &str) -> Result<Template> {
        let path = self.base_dir.join(format!("{}.json", id));
        let data =
            fs::read(&path).with_context(|| format!("reading template {}", path.display()))?;
        let template: Template = serde_json::from_slice(&data)
            .with_context(|| format!("parsing template {}", path.display()))?;
        template.validate()?;
        Ok(template)
    }

    pub fn list(&self) -> Result<Vec<TemplateInfo>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }
        let mut infos = Vec::new();
        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let data = fs::read(&path)?;
            let template: Template = match serde_json::from_slice(&data) {
                Ok(t) => t,
                Err(err) => {
                    eprintln!(
                        "Warning: could not parse template {}: {err}",
                        path.display()
                    );
                    continue;
                }
            };
            let modified = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .ok()
                .map(DateTime::<Utc>::from);
            infos.push(TemplateInfo {
                template,
                path,
                modified,
            });
        }
        infos.sort_by(|a, b| a.template.id.cmp(&b.template.id));
        Ok(infos)
    }
}

/// Génère un roster à partir d'un template et d'une période.
pub fn generate_roster(
    template: &Template,
    start: NaiveDate,
    end: NaiveDate,
    _rules: Option<Rules>,
) -> Result<Roster> {
    if end < start {
        bail!("end date must be after start date");
    }

    let mut roster = Roster::default();
    let mut current = start;

    while current <= end {
        let cycle_day = days_between(start, current) % i64::from(template.rotation_cycle_days);
        let weekday = current.weekday().number_from_monday() as u8;

        for slot in &template.slots {
            if !slot_matches_day(slot, weekday, cycle_day, template.rotation_cycle_days) {
                continue;
            }
            let (start_dt, end_dt) = build_datetimes(current, slot.start_time, slot.end_time);
            let mut shift = Shift::new(
                format!("{} {}", slot.role, current),
                start_dt,
                end_dt,
                Some(crate::model::Role::Custom(slot.role.clone())),
            )
            .map_err(anyhow::Error::msg)?;
            shift.assigned = None;
            roster.shifts.push(shift);
        }
        current = current.succ_opt().context("date overflow")?;
    }

    roster.shifts.sort_by_key(|s| s.start);

    Ok(roster)
}

pub fn export_template_json<P: AsRef<Path>>(path: P, template: &Template) -> Result<()> {
    let json = serde_json::to_string_pretty(template)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_template_from_file<P: AsRef<Path>>(path: P) -> Result<Template> {
    let data = fs::read(&path)?;
    let template: Template = serde_json::from_slice(&data)?;
    template.validate()?;
    Ok(template)
}

pub fn export_roster_to_path<P: AsRef<Path>>(path: P, roster: &Roster) -> Result<()> {
    io::export_roster_json(path, roster)
}

fn days_between(start: NaiveDate, current: NaiveDate) -> i64 {
    current.signed_duration_since(start).num_days()
}

fn slot_matches_day(slot: &Slot, weekday: u8, cycle_day: i64, cycle_len: u16) -> bool {
    slot.days.iter().any(|d| {
        if *d <= 7 {
            *d == weekday
        } else {
            let rel = cycle_day.rem_euclid(i64::from(cycle_len)) + 1;
            rel as u8 == *d
        }
    })
}

fn build_datetimes(
    date: NaiveDate,
    start_time: NaiveTime,
    end_time: NaiveTime,
) -> (DateTime<Utc>, DateTime<Utc>) {
    let start_dt = Utc.from_utc_datetime(&NaiveDateTime::new(date, start_time));
    let mut end_date = date;
    if end_time <= start_time {
        end_date = end_date.succ_opt().unwrap();
    }
    let end_dt = Utc.from_utc_datetime(&NaiveDateTime::new(end_date, end_time));
    (start_dt, end_dt)
}

fn validate_slot_overlaps(slots: &[Slot]) -> Result<()> {
    for (i, slot_a) in slots.iter().enumerate() {
        for slot_b in slots.iter().skip(i + 1) {
            if slot_a.role != slot_b.role {
                continue;
            }
            if slot_overlap(slot_a, slot_b) {
                bail!(
                    "template contains overlapping slots for role {} and {}",
                    slot_a.role,
                    slot_b.role
                );
            }
        }
    }
    Ok(())
}

fn slot_overlap(a: &Slot, b: &Slot) -> bool {
    let shared_days = a.days.iter().any(|da| b.days.contains(da));
    if !shared_days {
        return false;
    }
    let (a_start, a_end) = slot_bounds_seconds(a.start_time, a.end_time);
    let (b_start, b_end) = slot_bounds_seconds(b.start_time, b.end_time);
    !(a_end <= b_start || b_end <= a_start)
}

fn slot_bounds_seconds(start: NaiveTime, end: NaiveTime) -> (i32, i32) {
    let start_secs = start.num_seconds_from_midnight() as i32;
    let mut end_secs = end.num_seconds_from_midnight() as i32;
    if end <= start {
        end_secs += 24 * 60 * 60;
    }
    (start_secs, end_secs)
}
