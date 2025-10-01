use crate::model::{Person, Roster, Shift};
use anyhow::{bail, Context};
use chrono::{DateTime, Utc};
use csv::{ReaderBuilder, WriterBuilder};
use std::fs;
use std::path::Path;

/// Import de personnes depuis CSV: header `handle,display_name`
pub fn import_people_csv<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<Person>> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_path(path)?;
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        let handle = rec.get(0).context("missing handle")?.trim();
        let display = rec.get(1).context("missing display_name")?.trim();
        if handle.is_empty() || display.is_empty() {
            bail!("invalid people row (empty)");
        }
        let mut person = Person::new(handle.to_string(), display.to_string());
        if let Some(flag) = rec.get(2) {
            let flag = flag.trim();
            if !flag.is_empty() {
                person.on_vacation = parse_bool(flag)
                    .with_context(|| format!("invalid on_vacation value for handle {handle}"))?;
            }
        }
        out.push(person);
    }
    Ok(out)
}

fn parse_bool(s: &str) -> anyhow::Result<bool> {
    match s.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "y" | "oui" => Ok(true),
        "false" | "0" | "no" | "n" | "non" => Ok(false),
        _ => bail!("expected boolean"),
    }
}

/// Import de shifts: header `name,start,end` (RFC3339 UTC)
pub fn import_shifts_csv<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<Shift>> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_path(path)?;
    let mut out = Vec::new();
    for rec in rdr.records() {
        let rec = rec?;
        let name = rec.get(0).context("missing name")?.trim().to_string();
        let start = rec.get(1).context("missing start")?.trim();
        let end = rec.get(2).context("missing end")?.trim();
        let start: DateTime<Utc> = start.parse().context("start RFC3339")?;
        let end: DateTime<Utc> = end.parse().context("end RFC3339")?;
        let s = Shift::new(name, start, end, None).map_err(anyhow::Error::msg)?;
        out.push(s);
    }
    Ok(out)
}

/// Export JSON du roster (jolie mise en forme)
pub fn export_roster_json<P: AsRef<Path>>(path: P, roster: &Roster) -> anyhow::Result<()> {
    let s = serde_json::to_string_pretty(roster)?;
    fs::write(path, s)?;
    Ok(())
}

/// Export CSV des shifts: header `id,name,start,end,assigned_handle`
pub fn export_shifts_csv<P: AsRef<Path>>(path: P, roster: &Roster) -> anyhow::Result<()> {
    let mut w = WriterBuilder::new().has_headers(true).from_path(path)?;
    w.write_record(&["id", "name", "start", "end", "assigned_handle"])?;
    for s in &roster.shifts {
        let assigned = s.assigned.as_ref()
            .and_then(|pid| roster.people.iter().find(|p| p.id == *pid))
            .map(|p| p.handle.as_str()).unwrap_or("");
        w.write_record(&[
            s.id.as_str(), &s.name, &s.start.to_rfc3339(), &s.end.to_rfc3339(), assigned
        ])?;
    }
    w.flush()?;
    Ok(())
}
