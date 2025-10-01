use crate::model::{PersonId, ShiftId};
use thiserror::Error;

/// Options d'assignation
#[derive(Debug, Clone, Copy)]
pub struct AssignOptions {
    pub min_rest_hours: u32,
    pub max_consecutive_shifts: u32,
}

impl Default for AssignOptions {
    fn default() -> Self {
        Self {
            min_rest_hours: 11,
            max_consecutive_shifts: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    Overlap,
    DoubleAssignment,
    RestViolation,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub person: PersonId,
    pub shift_a: ShiftId,
    pub shift_b: ShiftId,
    pub kind: ConflictKind,
}

#[derive(Error, Debug)]
pub enum SchedError {
    #[error("invalid time range: end must be after start")]
    InvalidTimeRange,
    #[error("unknown person handle: {0}")]
    UnknownPerson(String),
    #[error("unknown shift: {0}")]
    UnknownShift(String),
    #[error("swap invalid: {0}")]
    SwapInvalid(&'static str),
    #[error("cover invalid: {0}")]
    CoverInvalid(&'static str),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
