use crate::model::Roster;
use anyhow::Context;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub trait Storage {
    /// Charge un roster depuis un support.
    fn load(&self) -> anyhow::Result<Roster>;
    /// Sauvegarde de manière atomique.
    fn save(&self, roster: &Roster) -> anyhow::Result<()>;
}

pub struct JsonStorage {
    path: PathBuf,
}

impl JsonStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        Ok(Self { path: path.as_ref().to_path_buf() })
    }
}

impl Storage for JsonStorage {
    fn load(&self) -> anyhow::Result<Roster> {
        let data = fs::read(&self.path).with_context(|| format!("reading {}", self.path.display()))?;
        let roster: Roster = serde_json::from_slice(&data).with_context(|| "parsing roster.json")?;
        Ok(roster)
    }

    fn save(&self, roster: &Roster) -> anyhow::Result<()> {
        let json = serde_json::to_vec_pretty(roster)?;
        let mut tmp = NamedTempFile::new_in(
            self.path.parent().unwrap_or_else(|| Path::new(".")))
            .with_context(|| "creating temp file")?;
        tmp.write_all(&json)?;
        tmp.flush()?;
        tmp.as_file().sync_all()?;
        tmp.persist(&self.path).with_context(|| "atomic rename")?;
        Ok(())
    }
}
