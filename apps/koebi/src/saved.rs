//! Text files in the data directory, written atomically with a backup: the workspace and the
//! shortcut overrides.
//!
//! A save writes a temporary file, flushes it to the disk, keeps the previous file as a backup
//! and renames the new file into place, so that a crash never leaves a half-written file. A
//! load reads the file, or its backup when the file is missing (a crash between the two
//! renames) or unreadable.

use std::fs;
use std::io::{self, Write as _};
use std::path::PathBuf;

/// One saved text file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SavedFile {
    dir: PathBuf,
    name: &'static str,
}

impl SavedFile {
    /// The file `name` in `dir`.
    pub(crate) fn new(dir: impl Into<PathBuf>, name: &'static str) -> Self {
        Self {
            dir: dir.into(),
            name,
        }
    }

    /// The file's path.
    pub(crate) fn path(&self) -> PathBuf {
        self.dir.join(self.name)
    }

    fn backup(&self) -> PathBuf {
        self.dir.join(format!("{}.bak", self.name))
    }

    /// The file's text, then its backup's, as far as they exist: try the first, and the second
    /// when the first cannot be used.
    pub(crate) fn read(&self) -> io::Result<Vec<String>> {
        let mut texts = Vec::new();
        for path in [self.path(), self.backup()] {
            match fs::read_to_string(&path) {
                Ok(text) => texts.push(text),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(texts)
    }

    /// Writes `text`, keeping the previous file as the backup.
    pub(crate) fn write(&self, text: &str) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let temporary = self.dir.join(format!("{}.tmp", self.name));
        let mut file = fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        let main = self.path();
        if main.exists() {
            fs::rename(&main, self.backup())?;
        }
        fs::rename(&temporary, &main)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directory(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("koebi-saved-{name}-{}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        dir
    }

    #[test]
    fn saves_keep_the_previous_file_as_a_backup() {
        let dir = directory("backup");
        let file = SavedFile::new(&dir, "state.txt");
        assert_eq!(file.read().unwrap(), Vec::<String>::new());
        file.write("one").unwrap();
        file.write("two").unwrap();
        assert_eq!(file.read().unwrap(), ["two", "one"]);
        // Without the main file (a crash between the renames), the backup remains.
        fs::remove_file(file.path()).unwrap();
        assert_eq!(file.read().unwrap(), ["one"]);
        fs::remove_dir_all(&dir).ok();
    }
}
