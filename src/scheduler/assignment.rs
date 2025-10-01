use super::{types::SchedError, util, AssignOptions, Scheduler};
use crate::model::{Person, PersonId, Shift};
use chrono::{DateTime, Utc};

pub(super) fn assign_rotative(
    scheduler: &mut Scheduler,
    people: &[Person],
    opts: AssignOptions,
) -> Result<(), SchedError> {
    if people.is_empty() {
        return Ok(());
    }

    scheduler.roster.shifts.sort_by_key(|s| s.start);
    let total = people.len();
    let mut cursor = 0usize;

    for shift_index in 0..scheduler.roster.shifts.len() {
        let candidate = scheduler.roster.shifts[shift_index].clone();

        let chosen = (0..total).find_map(|_| {
            let person = &people[cursor];
            cursor = (cursor + 1) % total;

            if person.on_vacation {
                return None;
            }

            if scheduler.person_ok_for_shift(&person.id, &candidate, opts, Some(shift_index)) {
                return Some(person.id.clone());
            }

            None
        });

        if let Some(person_id) = chosen {
            scheduler.roster.shifts[shift_index].assigned = Some(person_id);
        }
    }

    Ok(())
}

impl Scheduler {
    pub(super) fn person_ok_for_shift(
        &self,
        person: &PersonId,
        shift: &Shift,
        opts: AssignOptions,
        exclude_shift_index: Option<usize>,
    ) -> bool {
        let mut prev_end: Option<DateTime<Utc>> = None;
        let mut consec = 0u32;

        let mut assigned: Vec<&Shift> = self
            .roster
            .shifts
            .iter()
            .enumerate()
            .filter(|(idx, s)| {
                let same_person = s.assigned.as_ref() == Some(person);
                let excluded = exclude_shift_index.map(|i| i == *idx).unwrap_or(false);
                same_person && !excluded
            })
            .map(|(_, s)| s)
            .collect();
        assigned.sort_by_key(|s| s.start);

        for s in assigned {
            if util::overlaps(s.start, s.end, shift.start, shift.end) {
                return false;
            }
            if s.end <= shift.start {
                prev_end = Some(prev_end.map_or(s.end, |pe| pe.max(s.end)));
            }
            let gap = (shift.start - s.end).num_minutes();
            if gap.abs() <= 30 {
                consec += 1;
            }
        }

        if let Some(end) = prev_end {
            let rest_h = (shift.start - end).num_hours();
            if rest_h < i64::from(opts.min_rest_hours) {
                return false;
            }
        }

        if consec >= opts.max_consecutive_shifts {
            return false;
        }

        if let Some(p) = self.roster.find_person_by_id(person) {
            if p.on_vacation {
                return false;
            }
            if p.vacations
                .iter()
                .any(|vac| util::vacation_blocks_shift(vac, shift, opts))
            {
                return false;
            }
        }

        true
    }
}
