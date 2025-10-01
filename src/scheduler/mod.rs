mod assignment;
mod conflicts;
mod mutate;
mod types;
mod util;

pub use types::{AssignOptions, Conflict, ConflictKind, SchedError};

use crate::model::{Person, PersonId, Roster, Shift, ShiftId};
use chrono::{DateTime, Utc};

/// Scheduler : encapsule un Roster en cours de construction
#[derive(Debug, Default)]
pub struct Scheduler {
    roster: Roster,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            roster: Roster::default(),
        }
    }

    pub fn roster(&self) -> &Roster {
        &self.roster
    }
    pub fn roster_mut(&mut self) -> &mut Roster {
        &mut self.roster
    }

    pub fn add_people(&mut self, people: Vec<Person>) {
        self.roster.people.extend(people);
    }

    /// Crée un shift à partir de timestamps UTC
    pub fn create_shift(
        &mut self,
        name: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<ShiftId, SchedError> {
        if end <= start {
            return Err(SchedError::InvalidTimeRange);
        }
        let s = Shift::new(name.to_string(), start, end, None)
            .map_err(|_| SchedError::InvalidTimeRange)?;
        let id = s.id.clone();
        self.roster.shifts.push(s);
        Ok(id)
    }

    pub fn assign_rotative(
        &mut self,
        people: &[Person],
        opts: AssignOptions,
    ) -> Result<(), SchedError> {
        assignment::assign_rotative(self, people, opts)
    }

    pub fn detect_conflicts(&self, opts: AssignOptions) -> Vec<Conflict> {
        conflicts::detect_conflicts(self, opts)
    }

    pub fn swap(
        &mut self,
        shift_id: &ShiftId,
        a: &PersonId,
        b: &PersonId,
        opts: AssignOptions,
    ) -> Result<(), SchedError> {
        mutate::swap(self, shift_id, a, b, opts)
    }

    pub fn cover_shift(
        &mut self,
        shift_id: &ShiftId,
        from: DateTime<Utc>,
        person: &PersonId,
        opts: AssignOptions,
    ) -> Result<ShiftId, SchedError> {
        mutate::cover_shift(self, shift_id, from, person, opts)
    }
}
