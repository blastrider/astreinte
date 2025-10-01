use super::{util, AssignOptions, Conflict, ConflictKind, Scheduler};
use crate::model::Shift;

pub(super) fn detect_conflicts(scheduler: &Scheduler, opts: AssignOptions) -> Vec<Conflict> {
    let mut out = Vec::new();

    for person in scheduler.roster.people.iter() {
        let mut shifts: Vec<&Shift> = scheduler
            .roster
            .shifts
            .iter()
            .filter(|s| s.assigned.as_ref() == Some(&person.id))
            .collect();
        shifts.sort_by_key(|s| s.start);

        for (idx, a) in shifts.iter().enumerate() {
            for b in shifts.iter().skip(idx + 1) {
                if util::overlaps(a.start, a.end, b.start, b.end) {
                    out.push(Conflict {
                        person: person.id.clone(),
                        shift_a: a.id.clone(),
                        shift_b: b.id.clone(),
                        kind: ConflictKind::Overlap,
                    });
                }

                let rest_h = (b.start - a.end).num_hours();
                if rest_h < i64::from(opts.min_rest_hours) {
                    out.push(Conflict {
                        person: person.id.clone(),
                        shift_a: a.id.clone(),
                        shift_b: b.id.clone(),
                        kind: ConflictKind::RestViolation,
                    });
                }
            }
        }
    }

    out
}
