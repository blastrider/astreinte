use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifiant fort pour Person
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PersonId(String);

impl PersonId {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(s.as_ref().to_owned())
    }
    pub fn random() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Personne (membre d'astreinte)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: PersonId,
    pub handle: String,
    pub display_name: String,
    #[serde(default)]
    pub on_vacation: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vacations: Vec<VacationPeriod>,
}

impl Person {
    pub fn new<H: Into<String>, D: Into<String>>(handle: H, display_name: D) -> Self {
        Self {
            id: PersonId::random(),
            handle: handle.into(),
            display_name: display_name.into(),
            on_vacation: false,
            vacations: Vec::new(),
        }
    }
}

/// Période de congés d'une personne (intervalle UTC [start, end)).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VacationPeriod {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl VacationPeriod {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Self, String> {
        if end <= start {
            return Err("vacation end must be after start".to_string());
        }
        Ok(Self { start, end })
    }
}

/// Rôle éventuel (pour extensions post-MVP)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Primary,
    Secondary,
    Custom(String),
}

/// Identifiant fort pour Shift
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShiftId(String);

impl ShiftId {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(s.as_ref().to_owned())
    }
    pub fn random() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Créneau d'astreinte (UTC)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shift {
    pub id: ShiftId,
    pub name: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub role: Option<Role>,
    pub assigned: Option<PersonId>,
}

impl Shift {
    /// Crée un shift en validant que `end > start`.
    pub fn new(
        name: String,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        role: Option<Role>,
    ) -> Result<Self, String> {
        if end <= start {
            return Err("end must be strictly after start".to_string());
        }
        Ok(Self {
            id: ShiftId::random(),
            name,
            start,
            end,
            role,
            assigned: None,
        })
    }

    /// Durée en minutes.
    pub fn duration_minutes(&self) -> i64 {
        (self.end - self.start).num_minutes()
    }
}

/// Roster complet
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Roster {
    pub people: Vec<Person>,
    pub shifts: Vec<Shift>,
}

impl Roster {
    pub fn find_person_by_handle<'a>(&'a self, handle: &str) -> Option<&'a Person> {
        self.people.iter().find(|p| p.handle == handle)
    }
    pub fn find_person_by_id<'a>(&'a self, id: &PersonId) -> Option<&'a Person> {
        self.people.iter().find(|p| &p.id == id)
    }
    pub fn find_person_mut_by_id(&mut self, id: &PersonId) -> Option<&mut Person> {
        self.people.iter_mut().find(|p| &p.id == id)
    }
    pub fn find_shift_mut(&mut self, id: &ShiftId) -> Option<&mut Shift> {
        self.shifts.iter_mut().find(|s| &s.id == id)
    }
}
