use crate::model::{Person, PersonId, Roster, Shift, ShiftId};
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Options d'assignation
#[derive(Debug, Clone, Copy)]
pub struct AssignOptions {
    pub min_rest_hours: u32,
    pub max_consecutive_shifts: u32,
}

impl Default for AssignOptions {
    fn default() -> Self {
        Self { min_rest_hours: 11, max_consecutive_shifts: 3 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictKind {
    Overlap,               // chevauchement pour la même personne
    DoubleAssignment,      // deux shifts différents assignés au même moment
    RestViolation,         // repos minimal non respecté
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
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Scheduler : encapsule un Roster en cours de construction
#[derive(Debug, Default)]
pub struct Scheduler {
    roster: Roster,
}

impl Scheduler {
    pub fn new() -> Self { Self { roster: Roster::default() } }

    pub fn roster(&self) -> &Roster { &self.roster }
    pub fn roster_mut(&mut self) -> &mut Roster { &mut self.roster }

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

    /// Assigne en round-robin les shifts triés chronologiquement
    pub fn assign_rotative(&mut self, people: &[Person], opts: AssignOptions) -> Result<(), SchedError> {
        if people.is_empty() { return Ok(()); }
        // Indexer les personnes par ordre fourni
        let mut idx = 0usize;

        self.roster.shifts.sort_by_key(|s| s.start);

        for shift_index in 0..self.roster.shifts.len() {
            // clone pour évaluation sans emprunt mutable simultané
            let candidate = self.roster.shifts[shift_index].clone();
            let n = people.len();
            let mut tries = 0usize;
            let mut chosen: Option<PersonId> = None;

            while tries < n {
                let p = &people[idx % n];
                if p.on_vacation {
                    idx = (idx + 1) % n;
                    tries += 1;
                    continue;
                }
                if self.person_ok_for_shift(&p.id, &candidate, opts, Some(shift_index)) {
                    chosen = Some(p.id.clone());
                    idx = (idx + 1) % n;
                    break;
                }
                idx = (idx + 1) % n;
                tries += 1;
            }

            if let Some(person) = chosen {
                self.roster.shifts[shift_index].assigned = Some(person);
            }
            // si aucune personne ne convient : leave unassigned (warning au check)
        }
        Ok(())
    }

    /// Vérifie si `person` peut prendre `shift` selon les contraintes.
    fn person_ok_for_shift(
        &self,
        person: &PersonId,
        shift: &Shift,
        opts: AssignOptions,
        exclude_shift_index: Option<usize>,
    ) -> bool {
        let mut prev_end: Option<DateTime<Utc>> = None;
        let mut consec = 0u32;

        // Parcourt les shifts déjà assignés à cette personne en ordre chronologique
        let mut assigned: Vec<&Shift> = self.roster.shifts.iter()
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
            // chevauchement
            if overlaps(s.start, s.end, shift.start, shift.end) {
                return false;
            }
            // repos minimal
            if s.end <= shift.start {
                prev_end = Some(prev_end.map(|pe| pe.max(s.end)).unwrap_or(s.end));
            }
            // consecutive (adjacents ou quasi)
            let gap = (shift.start - s.end).num_minutes();
            if gap <= 30 && gap >= -30 { // fenêtre tolérance 30min
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
        }

        true
    }

    /// Détecte les conflits sur le roster courant.
    pub fn detect_conflicts(&self, opts: AssignOptions) -> Vec<Conflict> {
        let mut out = Vec::new();

        // par personne, regarde toutes les paires (O(k^2) sur ses shifts)
        for p in self.roster.people.iter() {
            let mut ps: Vec<&Shift> = self.roster.shifts.iter()
                .filter(|s| s.assigned.as_ref() == Some(&p.id))
                .collect();
            ps.sort_by_key(|s| s.start);

            for i in 0..ps.len() {
                for j in i+1..ps.len() {
                    let a = ps[i]; let b = ps[j];
                    if overlaps(a.start, a.end, b.start, b.end) {
                        out.push(Conflict {
                            person: p.id.clone(),
                            shift_a: a.id.clone(),
                            shift_b: b.id.clone(),
                            kind: ConflictKind::Overlap,
                        });
                    }
                    // repos minimal (b démarre trop tôt après a)
                    let rest_h = (b.start - a.end).num_hours();
                    if rest_h < i64::from(opts.min_rest_hours) {
                        out.push(Conflict {
                            person: p.id.clone(),
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

    /// Échange l'assignation d'un shift entre deux personnes (idempotent).
    pub fn swap(&mut self, shift_id: &ShiftId, a: &PersonId, b: &PersonId, opts: AssignOptions) -> Result<(), SchedError> {
        let pos = self.roster.shifts.iter().position(|s| &s.id == shift_id)
            .ok_or_else(|| SchedError::UnknownShift(shift_id.as_str().to_string()))?;

        let (target, prev) = {
            let s = &self.roster.shifts[pos];
            let target = if s.assigned.as_ref() == Some(a) {
                b.clone()
            } else if s.assigned.as_ref() == Some(b) {
                a.clone()
            } else {
                return Err(SchedError::SwapInvalid("shift not assigned to either person"));
            };
            (target, s.assigned.clone())
        };

        if let Some(person) = self.roster.find_person_by_id(&target) {
            if person.on_vacation {
                return Err(SchedError::SwapInvalid("target person on vacation"));
            }
        }

        self.roster.shifts[pos].assigned = Some(target.clone());

        // Valide qu'on n'introduit pas de conflit sévère
        let conflicts = self.detect_conflicts(opts);
        let severe = conflicts.iter().any(|c| &c.person == &target && c.kind == ConflictKind::Overlap);
        if severe {
            self.roster.shifts[pos].assigned = prev; // rollback
            return Err(SchedError::SwapInvalid("introduces overlap"));
        }
        Ok(())
    }
}

fn overlaps(a_start: DateTime<Utc>, a_end: DateTime<Utc>, b_start: DateTime<Utc>, b_end: DateTime<Utc>) -> bool {
    a_start < b_end && b_start < a_end
}
