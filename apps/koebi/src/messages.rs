//! The messages the Info editor shows: the application's log records from `info` up, kept as
//! they are logged (from any thread) and collected by the interface between frames.

use std::sync::Mutex;

use koebi_core::log::{self, Level, Record};

/// Most messages kept: older ones go first.
const LIMIT: usize = 500;

/// One log record, as the Info editor shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Message {
    /// Unique and increasing: the row's key.
    pub(crate) id: u64,
    pub(crate) level: Level,
    pub(crate) text: String,
}

/// Records logged since the interface last collected them.
static PENDING: Mutex<Vec<(Level, String)>> = Mutex::new(Vec::new());

/// Keeps every record from `info` up for the Info editor, and passes every record on to the
/// platform's sink.
fn sink(record: &Record<'_>) {
    if record.level() <= Level::Info
        && let Ok(mut pending) = PENDING.lock()
        && pending.len() < LIMIT
    {
        pending.push((record.level(), record.args().to_string()));
    }
    log::platform_sink(record);
}

/// Installs the sink that keeps messages. Call it before the first record; a sink installed
/// earlier (a test harness) stays.
pub(crate) fn install() {
    if log::set_sink(sink).is_err() {
        koebi_core::debug!("a log sink is already installed: the Info editor stays empty");
    }
}

/// The messages of the application, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Messages {
    list: Vec<Message>,
    next: u64,
}

impl Messages {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The messages, oldest first.
    pub(crate) fn list(&self) -> &[Message] {
        &self.list
    }

    /// Adds a message.
    pub(crate) fn push(&mut self, level: Level, text: String) {
        self.list.push(Message {
            id: self.next,
            level,
            text,
        });
        self.next += 1;
        if self.list.len() > LIMIT {
            self.list.drain(..self.list.len() - LIMIT);
        }
    }

    /// Takes the records logged since the last call. Returns whether there were any.
    pub(crate) fn collect(&mut self) -> bool {
        let Ok(mut pending) = PENDING.lock() else {
            return false;
        };
        if pending.is_empty() {
            return false;
        }
        let records = core::mem::take(&mut *pending);
        drop(pending);
        for (level, text) in records {
            self.push(level, text);
        }
        true
    }

    /// Removes the messages `remove` accepts.
    pub(crate) fn remove(&mut self, mut remove: impl FnMut(u64) -> bool) {
        self.list.retain(|message| !remove(message.id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_keep_the_latest_ones_with_increasing_keys() {
        let mut messages = Messages::new();
        for index in 0..LIMIT + 2 {
            messages.push(Level::Info, index.to_string());
        }
        assert_eq!(messages.list().len(), LIMIT);
        assert_eq!(messages.list()[0].text, "2");
        assert_eq!(messages.list()[0].id, 2);
        messages.remove(|id| id % 2 == 0);
        assert!(messages.list().iter().all(|message| message.id % 2 == 1));
    }
}
