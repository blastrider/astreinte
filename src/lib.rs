#![forbid(unsafe_code)]
//! Astreinte — bibliothèque de planification d'astreintes locale (sans BD).
//!
//! - Stockage fichiers (JSON/CSV).
//! - Rotation round-robin.
//! - Détection de conflits, swaps sûrs.
//! - Tout en UTC ; parsing RFC3339 ; affichage local en dehors de la lib.

pub mod model;
pub mod scheduler;
pub mod storage;
pub mod io;

pub use model::{Person, PersonId, Role, Shift, ShiftId, VacationPeriod, Roster};
pub use scheduler::{AssignOptions, Conflict, ConflictKind, Scheduler};
pub use storage::{Storage, JsonStorage};
