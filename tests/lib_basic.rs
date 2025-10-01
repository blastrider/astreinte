#![forbid(unsafe_code)]
use astreinte::{Scheduler, scheduler::AssignOptions, model::Person};
use chrono::{TimeZone, Utc};

#[test]
fn create_and_assign_basic() {
    let mut s = Scheduler::new();
    let a = Person::new("alice", "Alice");
    let b = Person::new("bob", "Bob");
    s.add_people(vec![a.clone(), b.clone()]);

    let t0 = Utc.with_ymd_and_hms(2025,10,1,8,0,0).unwrap();
    let t1 = Utc.with_ymd_and_hms(2025,10,1,20,0,0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025,10,2,8,0,0).unwrap();
    let t3 = Utc.with_ymd_and_hms(2025,10,2,20,0,0).unwrap();

    s.create_shift("nuit1", t0, t1).unwrap();
    s.create_shift("nuit2", t2, t3).unwrap();

    s.assign_rotative(&[a,b], AssignOptions::default()).unwrap();
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

    let t0 = Utc.with_ymd_and_hms(2025,10,1,8,0,0).unwrap();
    let t1 = Utc.with_ymd_and_hms(2025,10,1,12,0,0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025,10,1,10,0,0).unwrap();
    let t3 = Utc.with_ymd_and_hms(2025,10,1,14,0,0).unwrap();

    let id1 = s.create_shift("A", t0, t1).unwrap();
    let id2 = s.create_shift("B", t2, t3).unwrap();

    // assigne manuellement
    {
        let r = s.roster_mut();
        r.shifts.iter_mut().find(|sh| sh.id == id1).unwrap().assigned = Some(a.id.clone());
        r.shifts.iter_mut().find(|sh| sh.id == id2).unwrap().assigned = Some(a.id.clone());
    }

    let conflicts = s.detect_conflicts(AssignOptions::default());
    assert!(!conflicts.is_empty());
}
