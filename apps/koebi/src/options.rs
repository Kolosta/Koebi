//! Command-line options (hand-parsed: no argument-parsing dependency).

use std::fmt;

use koebi_gpu::PresentMode;

/// Parsed command line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// Render this many frames continuously, then exit (tests, benchmarks).
    pub frames: Option<u64>,
    /// Presentation pacing.
    pub present_mode: PresentMode,
    /// Print usage and exit.
    pub help: bool,
    /// Print the version and exit.
    pub version: bool,
}

/// Invalid command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// An argument Koebi does not know.
    UnknownArgument(String),
    /// An option that needs a value got none.
    MissingValue(&'static str),
    /// An option got an unusable value.
    InvalidValue {
        /// The option.
        option: &'static str,
        /// The rejected value.
        value: String,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownArgument(arg) => write!(f, "unknown argument {arg:?}"),
            Self::MissingValue(option) => write!(f, "{option} needs a value"),
            Self::InvalidValue { option, value } => {
                write!(f, "invalid value {value:?} for {option}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl Options {
    /// Usage text printed by `--help`.
    pub const USAGE: &'static str = "\
Usage: koebi [OPTIONS]

Options:
  --frames <N>             Render N frames continuously, then exit (0 on success)
  --present-mode <MODE>    fifo (vsync, default), mailbox or immediate
  -h, --help               Print this help
  -V, --version            Print the version

Environment:
  KOEBI_LOG=<level>        off, error, warn, info (default), debug, trace
  KOEBI_VALIDATION=<mode>  0, 1 or required (Vulkan validation layers)
  KOEBI_ADAPTER=<name>     Use the first GPU whose name contains <name>";

    /// Parses arguments, excluding the program name.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseError`] for unknown options or invalid values.
    pub fn parse<I, S>(args: I) -> Result<Self, ParseError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_ref() {
                "-h" | "--help" => options.help = true,
                "-V" | "--version" => options.version = true,
                "--frames" => {
                    let value = args.next().ok_or(ParseError::MissingValue("--frames"))?;
                    let frames =
                        value
                            .as_ref()
                            .parse()
                            .ok()
                            .filter(|n| *n > 0)
                            .ok_or_else(|| ParseError::InvalidValue {
                                option: "--frames",
                                value: value.as_ref().into(),
                            })?;
                    options.frames = Some(frames);
                }
                "--present-mode" => {
                    let value = args
                        .next()
                        .ok_or(ParseError::MissingValue("--present-mode"))?;
                    options.present_mode = match value.as_ref() {
                        "fifo" => PresentMode::Fifo,
                        "mailbox" => PresentMode::Mailbox,
                        "immediate" => PresentMode::Immediate,
                        other => {
                            return Err(ParseError::InvalidValue {
                                option: "--present-mode",
                                value: other.into(),
                            });
                        }
                    };
                }
                // Platform launchers add arguments of their own (macOS `-psn_*`, IDE helpers).
                other if other.starts_with("-psn_") => {}
                other => return Err(ParseError::UnknownArgument(other.into())),
            }
        }
        Ok(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_gives_defaults() {
        assert_eq!(Options::parse::<_, &str>([]), Ok(Options::default()));
    }

    #[test]
    fn frames_and_present_mode_are_parsed() {
        let options = Options::parse(["--frames", "120", "--present-mode", "mailbox"]).unwrap();
        assert_eq!(options.frames, Some(120));
        assert_eq!(options.present_mode, PresentMode::Mailbox);
    }

    #[test]
    fn invalid_input_is_reported() {
        assert_eq!(
            Options::parse(["--frames"]),
            Err(ParseError::MissingValue("--frames"))
        );
        assert!(matches!(
            Options::parse(["--frames", "0"]),
            Err(ParseError::InvalidValue { .. })
        ));
        assert!(matches!(
            Options::parse(["--frames", "x"]),
            Err(ParseError::InvalidValue { .. })
        ));
        assert_eq!(
            Options::parse(["--fast"]),
            Err(ParseError::UnknownArgument("--fast".into()))
        );
    }

    #[test]
    fn help_and_version_flags() {
        let options = Options::parse(["-h", "--version"]).unwrap();
        assert!(options.help && options.version);
        assert!(Options::USAGE.contains("--frames"));
    }
}
