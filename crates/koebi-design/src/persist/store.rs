//! [`Store`]: the design system's state on disk.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::error::{Error, Result};
use crate::persist::format::{RestoreReport, encode, restore};
use crate::system::DesignSystem;

const FILE: &str = "design.kds";
const BACKUP: &str = "design.kds.bak";
const TEMPORARY: &str = "design.kds.tmp";

/// The state of a [`DesignSystem`] in a directory, saved atomically with a backup.
///
/// - [`Self::load`] restores the saved state, falling back to the backup when the file is
///   damaged.
/// - [`Self::save`] writes a new file only when the design system changed since the last save
///   or load: it writes a temporary file, flushes it to the disk, keeps the previous file as a
///   backup and renames the new file into place, so a crash never leaves a half-written file.
/// - [`Self::save_if_due`] saves once the state has stayed unsaved for the store's delay: call
///   it every frame while a slider drags, and the disk sees one write per pause, not one per
///   frame. Call [`Self::save`] when the application goes to the background or quits.
#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    delay: Duration,
    saved_revision: Option<u64>,
    dirty_since: Option<Instant>,
}

impl Store {
    /// A store in `dir` (created on first save), with a delay of half a second.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            delay: Duration::from_millis(500),
            saved_revision: None,
            dirty_since: None,
        }
    }

    /// The same store with another delay for [`Self::save_if_due`].
    #[must_use]
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// The file holding the state.
    pub fn path(&self) -> PathBuf {
        self.dir.join(FILE)
    }

    /// `true` when `design` changed since the last save or load.
    pub fn is_dirty(&self, design: &DesignSystem) -> bool {
        self.saved_revision != Some(design.revision())
    }

    /// Restores the saved state into `design`, normally freshly created. Returns `None` when
    /// nothing was saved yet.
    ///
    /// # Errors
    ///
    /// When neither the file nor its backup can be read and restored; `design` is unchanged.
    pub fn load(&mut self, design: &mut DesignSystem) -> Result<Option<RestoreReport>> {
        let main = read(&self.dir.join(FILE))?;
        let backup = || read(&self.dir.join(BACKUP));
        let report = if let Some(bytes) = main {
            match restore(design, &bytes) {
                Ok(report) => report,
                Err(error @ (Error::Corrupt(_) | Error::UnsupportedVersion(_))) => {
                    let Some(bytes) = backup()? else {
                        return Err(error);
                    };
                    let mut report = restore(design, &bytes)?;
                    report.skipped.insert(
                        0,
                        format!("{FILE} was damaged ({error}): restored the backup"),
                    );
                    report
                }
                Err(error) => return Err(error),
            }
        } else {
            // A crash between the two renames of a save leaves only the backup.
            let Some(bytes) = backup()? else {
                self.saved_revision = Some(design.revision());
                return Ok(None);
            };
            restore(design, &bytes)?
        };
        self.saved_revision = Some(design.revision());
        self.dirty_since = None;
        Ok(Some(report))
    }

    /// Saves the state of `design` if it changed since the last save or load. Returns whether
    /// a file was written.
    ///
    /// # Errors
    ///
    /// When the directory or a file cannot be written; the previous file stays in place.
    pub fn save(&mut self, design: &DesignSystem) -> Result<bool> {
        if !self.is_dirty(design) {
            return Ok(false);
        }
        let bytes = encode(design);
        fs::create_dir_all(&self.dir).map_err(|e| io_error("create", &self.dir, &e))?;
        let temporary = self.dir.join(TEMPORARY);
        let mut file =
            fs::File::create(&temporary).map_err(|e| io_error("create", &temporary, &e))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| io_error("write", &temporary, &e))?;
        drop(file);
        let main = self.dir.join(FILE);
        if main.exists() {
            let backup = self.dir.join(BACKUP);
            fs::rename(&main, &backup).map_err(|e| io_error("rename", &main, &e))?;
        }
        fs::rename(&temporary, &main).map_err(|e| io_error("rename", &temporary, &e))?;
        self.saved_revision = Some(design.revision());
        self.dirty_since = None;
        Ok(true)
    }

    /// Saves when the state has stayed unsaved for the store's delay. Returns whether a file
    /// was written.
    ///
    /// # Errors
    ///
    /// The errors of [`Self::save`].
    pub fn save_if_due(&mut self, design: &DesignSystem, now: Instant) -> Result<bool> {
        if !self.is_dirty(design) {
            self.dirty_since = None;
            return Ok(false);
        }
        let since = *self.dirty_since.get_or_insert(now);
        if now.saturating_duration_since(since) < self.delay {
            return Ok(false);
        }
        self.save(design)
    }
}

fn read(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error("read", path, &error)),
    }
}

fn io_error(operation: &'static str, path: &Path, error: &std::io::Error) -> Error {
    Error::Io {
        operation,
        path: path.display().to_string(),
        message: error.to_string(),
    }
}
