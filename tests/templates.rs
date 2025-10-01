#![forbid(unsafe_code)]
use astreinte::{generate_roster, Rules, Slot, Template, TemplateStore};
use chrono::{NaiveDate, NaiveTime};
use tempfile::tempdir;

#[test]
fn save_and_load_template_roundtrip() {
    let dir = tempdir().unwrap();
    let store = TemplateStore::new(dir.path());
    let template = sample_template();
    store.save(&template).unwrap();

    let loaded = store.load(&template.id).unwrap();
    assert_eq!(loaded.id, template.id);
    assert_eq!(loaded.slots.len(), template.slots.len());
}

#[test]
fn generate_roster_from_template() {
    let template = sample_template();
    let start = NaiveDate::from_ymd_opt(2025, 10, 24).unwrap(); // Friday
    let end = NaiveDate::from_ymd_opt(2025, 10, 28).unwrap(); // Tuesday

    let roster = generate_roster(&template, start, end, template.rules.clone()).unwrap();
    assert!(!roster.shifts.is_empty());

    // Expect two shifts per applicable day (oncall + backup on weekend days)
    let weekend_shifts: Vec<_> = roster
        .shifts
        .iter()
        .filter(|s| s.name.contains("oncall") || s.name.contains("backup"))
        .collect();
    assert_eq!(weekend_shifts.len(), 4);

    // Ensure no overlapping shifts share the same timestamp ordering issue
    for window in roster.shifts.windows(2) {
        if let [a, b] = window {
            assert!(a.start <= b.start);
        }
    }
}

fn sample_template() -> Template {
    Template {
        id: "weekend-2p".into(),
        name: "Week-end 2 personnes".into(),
        description: Some("Rotation week-end".into()),
        rotation_cycle_days: 14,
        slots: vec![
            Slot {
                role: "oncall".into(),
                start_time: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                days: vec![6, 7],
                priority: 0,
            },
            Slot {
                role: "backup".into(),
                start_time: NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
                end_time: NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                days: vec![6, 7],
                priority: 1,
            },
        ],
        rules: Some(Rules {
            min_rest_hours: Some(12),
            max_consecutive_days: Some(3),
            allow_weekend_swap: true,
        }),
        metadata: None,
    }
}
