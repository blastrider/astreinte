use crate::model::{Person, Roster, Shift, VacationPeriod};
use anyhow::{bail, Context};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use csv::{ReaderBuilder, WriterBuilder};
use std::fs;
use std::path::Path;

/// Import de personnes depuis CSV: header `handle,display_name[,on_vacation][,vacations]`
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
        if let Some(ranges) = rec.get(3) {
            let ranges = ranges.trim();
            if !ranges.is_empty() {
                person.vacations = parse_vacations(ranges)
                    .with_context(|| format!("invalid vacations value for handle {handle}"))?;
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

fn parse_vacations(raw: &str) -> anyhow::Result<Vec<VacationPeriod>> {
    raw.split(';')
        .filter(|chunk| !chunk.trim().is_empty())
        .map(|chunk| parse_vacation_chunk(chunk.trim()))
        .collect()
}

fn parse_vacation_chunk(chunk: &str) -> anyhow::Result<VacationPeriod> {
    if let Some((start_raw, end_raw)) = chunk.split_once('/').or_else(|| chunk.split_once("..")) {
        let (start, _) = parse_point(start_raw.trim())?;
        let (mut end, end_was_date) = parse_point(end_raw.trim())?;
        if end_was_date {
            end += Duration::days(1);
        }
        VacationPeriod::new(start, end).map_err(anyhow::Error::msg)
    } else {
        let (start, _) = parse_point(chunk)?;
        let end = start + Duration::days(1);
        VacationPeriod::new(start, end).map_err(anyhow::Error::msg)
    }
}

fn parse_point(raw: &str) -> anyhow::Result<(DateTime<Utc>, bool)> {
    if let Ok(dt) = raw.parse::<DateTime<Utc>>() {
        return Ok((dt, false));
    }
    let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .with_context(|| format!("invalid date/datetime: {raw}"))?;
    let datetime = date
        .and_hms_opt(0, 0, 0)
        .context("invalid midnight conversion")?;
    Ok((Utc.from_utc_datetime(&datetime), true))
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
    w.write_record(["id", "name", "start", "end", "assigned_handle"])?;
    for s in &roster.shifts {
        let assigned = s
            .assigned
            .as_ref()
            .and_then(|pid| roster.people.iter().find(|p| p.id == *pid))
            .map(|p| p.handle.as_str())
            .unwrap_or("");
        let start = s.start.to_rfc3339();
        let end = s.end.to_rfc3339();
        w.write_record([
            s.id.as_str(),
            s.name.as_str(),
            start.as_str(),
            end.as_str(),
            assigned,
        ])?;
    }
    w.flush()?;
    Ok(())
}
