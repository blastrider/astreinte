#![forbid(unsafe_code)]
use astreinte::{AssignOptions, Person, Scheduler, VacationPeriod};
use chrono::{TimeZone, Utc};

#[test]
fn create_and_assign_basic() {
    let mut s = Scheduler::new();
    let a = Person::new("alice", "Alice");
    let b = Person::new("bob", "Bob");
    s.add_people(vec![a.clone(), b.clone()]);

    let t0 = Utc.with_ymd_and_hms(2025, 10, 1, 8, 0, 0).unwrap();
    let t1 = Utc.with_ymd_and_hms(2025, 10, 1, 20, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025, 10, 2, 8, 0, 0).unwrap();
    let t3 = Utc.with_ymd_and_hms(2025, 10, 2, 20, 0, 0).unwrap();

    s.create_shift("nuit1", t0, t1).unwrap();
    s.create_shift("nuit2", t2, t3).unwrap();

    s.assign_rotative(&[a, b], AssignOptions::default())
        .unwrap();
    let roster = s.roster();
    assert_eq!(roster.shifts.len(), 2);
    assert!(roster.shifts[0].assigned.is_some());
    assert!(roster.shifts[1].assigned.is_some());
}

#[test]
fn detect_overlap_conflict() {
    let mut s = Scheduler::new();
    let a = Person::new("alice", "Alice");
    s.add_people(vec![a.clone()]);

    let t0 = Utc.with_ymd_and_hms(2025, 10, 1, 8, 0, 0).unwrap();
    let t1 = Utc.with_ymd_and_hms(2025, 10, 1, 12, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025, 10, 1, 10, 0, 0).unwrap();
    let t3 = Utc.with_ymd_and_hms(2025, 10, 1, 14, 0, 0).unwrap();

    let id1 = s.create_shift("A", t0, t1).unwrap();
    let id2 = s.create_shift("B", t2, t3).unwrap();

    // assigne manuellement
    {
        let r = s.roster_mut();
        r.shifts
            .iter_mut()
            .find(|sh| sh.id == id1)
            .unwrap()
            .assigned = Some(a.id.clone());
        r.shifts
            .iter_mut()
            .find(|sh| sh.id == id2)
            .unwrap()
            .assigned = Some(a.id.clone());
    }

    let conflicts = s.detect_conflicts(AssignOptions::default());
    assert!(!conflicts.is_empty());
}

#[test]
fn round_robin_skips_people_on_vacation() {
    let mut scheduler = Scheduler::new();
    let alice = Person::new("alice", "Alice");
    let mut bob = Person::new("bob", "Bob");

    let vac_start = Utc.with_ymd_and_hms(2025, 10, 2, 0, 0, 0).unwrap();
    let vac_end = Utc.with_ymd_and_hms(2025, 10, 3, 0, 0, 0).unwrap();
    bob.vacations = vec![VacationPeriod::new(vac_start, vac_end).unwrap()];

    scheduler.add_people(vec![alice.clone(), bob.clone()]);

    let t0 = Utc.with_ymd_and_hms(2025, 10, 1, 8, 0, 0).unwrap();
    let t1 = Utc.with_ymd_and_hms(2025, 10, 1, 20, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025, 10, 2, 8, 0, 0).unwrap();
    let t3 = Utc.with_ymd_and_hms(2025, 10, 2, 20, 0, 0).unwrap();

    scheduler.create_shift("day1", t0, t1).unwrap();
    scheduler.create_shift("day2", t2, t3).unwrap();

    scheduler
        .assign_rotative(&[alice.clone(), bob.clone()], AssignOptions::default())
        .unwrap();

    let roster = scheduler.roster();
    let assignees: Vec<_> = roster
        .shifts
        .iter()
        .map(|shift| shift.assigned.as_ref().map(|id| id.as_str()))
        .collect();

    assert_eq!(assignees[0], Some(alice.id.as_str()));
    assert_eq!(assignees[1], Some(alice.id.as_str()));
}

#[test]
fn cover_shift_splits_and_assigns() {
    let mut scheduler = Scheduler::new();
    let alice = Person::new("alice", "Alice");
    let bob = Person::new("bob", "Bob");

    scheduler.add_people(vec![alice.clone(), bob.clone()]);

    let start = Utc.with_ymd_and_hms(2025, 12, 26, 8, 0, 0).unwrap();
    let mid = Utc.with_ymd_and_hms(2025, 12, 29, 8, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 1, 2, 8, 0, 0).unwrap();

    let shift_id = scheduler.create_shift("holiday", start, end).unwrap();

    // initial assignment to Alice
    scheduler
        .roster_mut()
        .find_shift_mut(&shift_id)
        .unwrap()
        .assigned = Some(alice.id.clone());

    let opts = AssignOptions::default();
    let new_id = scheduler
        .cover_shift(&shift_id, mid, &bob.id, opts)
        .expect("cover should succeed");

    let roster = scheduler.roster();
    let first = roster.shifts.iter().find(|s| s.id == shift_id).unwrap();
    assert_eq!(first.end, mid);
    assert_eq!(first.assigned.as_ref(), Some(&alice.id));

    let second = roster.shifts.iter().find(|s| s.id == new_id).unwrap();
    assert_eq!(second.start, mid);
    assert_eq!(second.end, end);
    assert_eq!(second.assigned.as_ref(), Some(&bob.id));
}

#[test]
fn cover_rejects_vacation_overlap() {
    let mut scheduler = Scheduler::new();
    let alice = Person::new("alice", "Alice");
    let mut bob = Person::new("bob", "Bob");

    scheduler.add_people(vec![alice.clone(), bob.clone()]);

    let start = Utc.with_ymd_and_hms(2025, 12, 26, 8, 0, 0).unwrap();
    let mid = Utc.with_ymd_and_hms(2025, 12, 29, 8, 0, 0).unwrap();
    let end = Utc.with_ymd_and_hms(2026, 1, 2, 8, 0, 0).unwrap();

    let vac_start = Utc.with_ymd_and_hms(2025, 12, 29, 0, 0, 0).unwrap();
    let vac_end = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    bob.vacations = vec![VacationPeriod::new(vac_start, vac_end).unwrap()];
    scheduler
        .roster_mut()
        .find_person_mut_by_id(&bob.id)
        .unwrap()
        .vacations = bob.vacations.clone();

    let shift_id = scheduler.create_shift("holiday", start, end).unwrap();
    scheduler
        .roster_mut()
        .find_shift_mut(&shift_id)
        .unwrap()
        .assigned = Some(alice.id.clone());

    let result = scheduler.cover_shift(&shift_id, mid, &bob.id, AssignOptions::default());
    assert!(result.is_err());
}
