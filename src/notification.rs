use crate::model::{Person, Roster, Shift};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};

/// Représente un rappel généré pour une personne.
#[derive(Debug, Clone)]
pub struct Reminder {
    pub person_handle: String,
    pub shift_id: String,
    pub notice_at: DateTime<Utc>,
    pub content: String,
}

/// Permet de customiser le rendu du message (texte, SMS, etc.).
pub trait ReminderRenderer {
    fn render(&self, person: &Person, shift: &Shift, notice_at: DateTime<Utc>) -> String;
}

/// Gabarit texte simple destiné à un futur mail/SMS.
#[derive(Debug, Default, Clone, Copy)]
pub struct TextReminder;

impl ReminderRenderer for TextReminder {
    fn render(&self, person: &Person, shift: &Shift, notice_at: DateTime<Utc>) -> String {
        format!(
            "Bonjour {name},\n\nTu es d'astreinte pour le créneau \"{shift}\" du {start} au {end}.\nCe message est généré le {notice}.\n\nMerci de te préparer et de vérifier ton matériel.\n",
            name = person.display_name,
            shift = shift.name,
            start = shift.start.to_rfc3339(),
            end = shift.end.to_rfc3339(),
            notice = notice_at.to_rfc3339()
        )
    }
}

/// Prépare un rappel pour la prochaine astreinte d'une personne.
pub fn prepare_reminder(
    roster: &Roster,
    handle: &str,
    days_before: i64,
    now: DateTime<Utc>,
    renderer: &dyn ReminderRenderer,
) -> Result<Reminder> {
    if days_before < 0 {
        bail!("days_before must be positive");
    }

    let person = roster
        .find_person_by_handle(handle)
        .with_context(|| format!("unknown person handle: {handle}"))?;

    let mut upcoming: Vec<&Shift> = roster
        .shifts
        .iter()
        .filter(|shift| shift.assigned.as_ref() == Some(&person.id) && shift.start >= now)
        .collect();

    if upcoming.is_empty() {
        bail!("no upcoming shift found for handle {handle}");
    }

    upcoming.sort_by_key(|shift| shift.start);
    let shift = upcoming[0];

    let notice_at = shift.start - Duration::days(days_before);

    let content = renderer.render(person, shift, notice_at);
    Ok(Reminder {
        person_handle: person.handle.clone(),
        shift_id: shift.id.as_str().to_string(),
        notice_at,
        content,
    })
}
