//! Command-line options (hand-parsed: no argument-parsing dependency).

use std::fmt;
use std::path::PathBuf;

use koebi_design::ColorVision;
use koebi_gpu::PresentMode;

use crate::design_options::COLOR_VISION_MODES;

/// Parsed command line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Options {
    /// Render this many frames continuously, then exit (tests, benchmarks).
    pub frames: Option<u64>,
    /// Presentation pacing.
    pub present_mode: PresentMode,
    /// Directory of the saved design state; the platform's preferences directory if `None`.
    pub data_dir: Option<PathBuf>,
    /// Theme to activate at startup, by key.
    pub theme: Option<String>,
    /// Global overrides to set at startup: token path and value, in command-line order.
    pub overrides: Vec<(String, String)>,
    /// Color vision setting to apply at startup.
    pub color_vision: Option<ColorVision>,
    /// Design Tokens files to import as themes, in command-line order; the last one becomes
    /// the active theme unless `theme` names another.
    pub imports: Vec<PathBuf>,
    /// File to write the active theme to, once the other options are applied.
    pub export: Option<PathBuf>,
    /// File to write every token to, with its base definition.
    pub export_schema: Option<PathBuf>,
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
  --data-dir <DIR>         Keep the design state in DIR (default: the user's preferences)
  --theme <KEY>            Activate a theme: dark, light, muted-green, high-contrast
  --override <PATH=VALUE>  Override a token in every theme, for instance
                           semantic.radius.control=6,
                           component.button.background.color.default=#ff8800 or
                           semantic.accent.color.700={primitive.color.green.700}
  --color-vision <MODE>    normal, simulate-protan, simulate-deutan, simulate-tritan,
                           simulate-achromatopsia, correct-protan, correct-deutan or
                           correct-tritan
  --import <FILE>          Import a theme from a Design Tokens (JSON) file and activate it
  --export <FILE>          Write the active theme to FILE in the Design Tokens format
  --export-schema <FILE>   Write every token, with its base definition, to FILE
  -h, --help               Print this help
  -V, --version            Print the version

Design changes made with these options are saved as soon as the application starts.

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
                "--data-dir" => {
                    let value = args.next().ok_or(ParseError::MissingValue("--data-dir"))?;
                    options.data_dir = Some(PathBuf::from(value.as_ref()));
                }
                "--theme" => {
                    let value = args.next().ok_or(ParseError::MissingValue("--theme"))?;
                    options.theme = Some(value.as_ref().to_owned());
                }
                "--override" => {
                    let value = args.next().ok_or(ParseError::MissingValue("--override"))?;
                    let (path, definition) = value
                        .as_ref()
                        .split_once('=')
                        .filter(|(path, definition)| !path.is_empty() && !definition.is_empty())
                        .ok_or_else(|| ParseError::InvalidValue {
                            option: "--override",
                            value: value.as_ref().into(),
                        })?;
                    options
                        .overrides
                        .push((path.trim().to_owned(), definition.trim().to_owned()));
                }
                "--import" => {
                    let value = args.next().ok_or(ParseError::MissingValue("--import"))?;
                    options.imports.push(PathBuf::from(value.as_ref()));
                }
                "--export" => {
                    let value = args.next().ok_or(ParseError::MissingValue("--export"))?;
                    options.export = Some(PathBuf::from(value.as_ref()));
                }
                "--export-schema" => {
                    let value = args
                        .next()
                        .ok_or(ParseError::MissingValue("--export-schema"))?;
                    options.export_schema = Some(PathBuf::from(value.as_ref()));
                }
                "--color-vision" => {
                    let value = args
                        .next()
                        .ok_or(ParseError::MissingValue("--color-vision"))?;
                    let (_, vision) = COLOR_VISION_MODES
                        .iter()
                        .find(|(name, _)| *name == value.as_ref())
                        .ok_or_else(|| ParseError::InvalidValue {
                            option: "--color-vision",
                            value: value.as_ref().into(),
                        })?;
                    options.color_vision = Some(*vision);
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
    fn design_options_are_parsed() {
        let options = Options::parse([
            "--data-dir",
            "state",
            "--theme",
            "light",
            "--override",
            "semantic.radius.control = 6",
            "--override",
            "component.button.background.color.default=#ff8800",
            "--color-vision",
            "simulate-deutan",
        ])
        .unwrap();
        assert_eq!(options.data_dir, Some(PathBuf::from("state")));
        assert_eq!(options.theme.as_deref(), Some("light"));
        assert_eq!(
            options.overrides,
            [
                ("semantic.radius.control".to_owned(), "6".to_owned()),
                (
                    "component.button.background.color.default".to_owned(),
                    "#ff8800".to_owned()
                ),
            ]
        );
        assert!(matches!(
            options.color_vision,
            Some(ColorVision::Simulate { .. })
        ));
        let files = Options::parse([
            "--import",
            "a.json",
            "--import",
            "b.json",
            "--export",
            "out.json",
            "--export-schema",
            "schema.json",
        ])
        .unwrap();
        assert_eq!(
            files.imports,
            [PathBuf::from("a.json"), PathBuf::from("b.json")]
        );
        assert_eq!(files.export, Some(PathBuf::from("out.json")));
        assert_eq!(files.export_schema, Some(PathBuf::from("schema.json")));
        for invalid in [["--override", "no-value"], ["--color-vision", "sepia"]] {
            assert!(matches!(
                Options::parse(invalid),
                Err(ParseError::InvalidValue { .. })
            ));
        }
    }

    #[test]
    fn help_and_version_flags() {
        let options = Options::parse(["-h", "--version"]).unwrap();
        assert!(options.help && options.version);
        assert!(Options::USAGE.contains("--frames"));
    }
}
