use super::{util, AssignOptions, ConflictKind, SchedError, Scheduler};
use crate::model::{PersonId, Shift, ShiftId};
use chrono::{DateTime, Utc};

pub(super) fn swap(
    scheduler: &mut Scheduler,
    shift_id: &ShiftId,
    a: &PersonId,
    b: &PersonId,
    opts: AssignOptions,
) -> Result<(), SchedError> {
    let Some(pos) = util::find_shift_index(&scheduler.roster.shifts, shift_id) else {
        return Err(SchedError::UnknownShift(shift_id.as_str().to_string()));
    };

    let (target, prev) = {
        let shift = &scheduler.roster.shifts[pos];
        let target = if shift.assigned.as_ref() == Some(a) {
            b.clone()
        } else if shift.assigned.as_ref() == Some(b) {
            a.clone()
        } else {
            return Err(SchedError::SwapInvalid(
                "shift not assigned to either person",
            ));
        };
        (target, shift.assigned.clone())
    };

    if let Some(person) = scheduler.roster.find_person_by_id(&target) {
        if person.on_vacation {
            return Err(SchedError::SwapInvalid("target person on vacation"));
        }
        if person
            .vacations
            .iter()
            .any(|vac| util::vacation_blocks_shift(vac, &scheduler.roster.shifts[pos], opts))
        {
            return Err(SchedError::SwapInvalid("target person on vacation range"));
        }
    }

    scheduler.roster.shifts[pos].assigned = Some(target.clone());

    let conflicts = scheduler.detect_conflicts(opts);
    let severe = conflicts
        .iter()
        .any(|c| &c.person == &target && c.kind == ConflictKind::Overlap);
    if severe {
        scheduler.roster.shifts[pos].assigned = prev;
        return Err(SchedError::SwapInvalid("introduces overlap"));
    }
    Ok(())
}

pub(super) fn cover_shift(
    scheduler: &mut Scheduler,
    shift_id: &ShiftId,
    from: DateTime<Utc>,
    person: &PersonId,
    opts: AssignOptions,
) -> Result<ShiftId, SchedError> {
    let Some(pos) = util::find_shift_index(&scheduler.roster.shifts, shift_id) else {
        return Err(SchedError::UnknownShift(shift_id.as_str().to_string()));
    };

    let cover = scheduler
        .roster
        .find_person_by_id(person)
        .ok_or_else(|| SchedError::UnknownPerson(person.as_str().to_string()))?;
    if cover.on_vacation {
        return Err(SchedError::CoverInvalid("person on vacation"));
    }

    let original = scheduler.roster.shifts[pos].clone();
    if from <= original.start {
        return Err(SchedError::CoverInvalid("cover point before shift start"));
    }
    if from >= original.end {
        return Err(SchedError::CoverInvalid("cover point after shift end"));
    }

    let mut new_segment = Shift {
        id: ShiftId::random(),
        name: original.name.clone(),
        start: from,
        end: original.end,
        role: original.role.clone(),
        assigned: None,
    };

    if cover
        .vacations
        .iter()
        .any(|vac| util::vacation_blocks_shift(vac, &new_segment, opts))
    {
        return Err(SchedError::CoverInvalid("person vacation conflicts"));
    }

    if !scheduler.person_ok_for_shift(person, &new_segment, opts, None) {
        return Err(SchedError::CoverInvalid("assignment constraints violated"));
    }

    scheduler.roster.shifts[pos].end = from;

    new_segment.assigned = Some(person.clone());
    let new_id = new_segment.id.clone();
    scheduler.roster.shifts.insert(pos + 1, new_segment);

    Ok(new_id)
}
