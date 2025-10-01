use super::AssignOptions;
use crate::model::{Shift, ShiftId, VacationPeriod};
use chrono::{DateTime, Duration, Utc};

pub(super) fn overlaps(
    a_start: DateTime<Utc>,
    a_end: DateTime<Utc>,
    b_start: DateTime<Utc>,
    b_end: DateTime<Utc>,
) -> bool {
    a_start < b_end && b_start < a_end
}

pub(super) fn vacation_blocks_shift(
    vac: &VacationPeriod,
    shift: &Shift,
    opts: AssignOptions,
) -> bool {
    let buffer = Duration::hours(i64::from(opts.min_rest_hours));
    let vac_start = vac.start - buffer;
    let vac_end = vac.end + buffer;
    shift.start < vac_end && vac_start < shift.end
}

pub(super) fn find_shift_index(shifts: &[Shift], shift_id: &ShiftId) -> Option<usize> {
    shifts.iter().position(|s| &s.id == shift_id)
}
