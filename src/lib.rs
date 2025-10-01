#![forbid(unsafe_code)]
//! Astreinte — bibliothèque de planification d'astreintes locale (sans BD).
//!
//! - Stockage fichiers (JSON/CSV).
//! - Rotation round-robin.
//! - Détection de conflits, swaps sûrs.
//! - Tout en UTC ; parsing RFC3339 ; affichage local en dehors de la lib.

pub mod io;
pub mod model;
pub mod notification;
pub mod scheduler;
pub mod storage;
pub mod template;

pub use model::{Person, PersonId, Role, Roster, Shift, ShiftId, VacationPeriod};
pub use notification::{prepare_reminder, Reminder, ReminderRenderer, TextReminder};
pub use scheduler::{AssignOptions, Conflict, ConflictKind, Scheduler};
pub use storage::{JsonStorage, Storage};
pub use template::{
    export_roster_to_path, export_template_json, generate_roster, load_template_from_file, Rules,
    Slot, Template, TemplateInfo, TemplateStore,
};
