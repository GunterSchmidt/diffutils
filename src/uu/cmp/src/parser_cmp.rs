// This file is part of the uutils diffutils package.
//
// For the full copyright and license information, please view the LICENSE-*
// files that was distributed with this source code.

//! This is the parser for the cmp utility.
//!
//! It uses the parsed data clap provides and fills the [params] for cmp.
//! It contains the allowed options, specific parsing logic and parsing error messages.
//!
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use uucore::display::Quotable;
use uucore::parser::parse_size::{ParseSizeError, Parser};
use uudiff::{error::UError, translate};

/// For option --bytes, set to u64, so large size limits can
/// be expressed, like Exabyte. \
/// This could be set to u128 with small modifications,
/// but AFAIK file sizes (metadata) can not exceed u64.
/// This is also limiting the compare function to u64::MAX
/// as this is the default value.
pub type BytesLimitU64 = u64;
/// For option --ignore initial, should not be changed.
pub type SkipU64 = u64;

/// Units up eo Exabyte (EiB) following GNU documentation: \
/// <https://www.gnu.org/software/diffutils/manual/html_node/cmp-Options.html>.
// "kB" | "KB" => 1_000,
// "k" | "K" | "KiB" | "kiB" => 1_024,
// "MB" => 1_000_000,
// "m" | "M" | "MiB" => 1_048_576,
// "GB" => 1_000_000_000,
// "g" | "G" | "GiB" => 1_073_741_824,
// "TB" => 1_000_000_000_000,
// "t" | "T" | "TiB" => 1_099_511_627_776,
// "PB" => 1_000_000_000_000_000,
// "p" | "P" | "PiB" => 1_125_899_906_842_624,
// "EB" => 1_000_000_000_000_000_000,
// "e" | "E" | "EiB" => 1_152_921_504_606_846_976,
const ALLOWED_UNITS: [&str; 26] = [
    "kB", "KB", "k", "K", "KiB", "kiB", "MB", "m", "M", "MiB", "GB", "g", "G", "GiB", "TB", "t",
    "T", "TiB", "PB", "p", "P", "PiB", "EB", "e", "E", "EiB",
];

// Allowed utility arguments (options)
pub mod options {

    /// Generic option for files and other undefined operands
    pub const FILE: &str = "file";
    ///   -n, --bytes=LIMIT          compare at most LIMIT bytes
    pub const BYTES_LIMIT: &str = "bytes";
    ///   -i, --ignore-initial=SKIP         skip first SKIP bytes of both inputs
    ///   -i, --ignore-initial=SKIP1:SKIP2  skip first SKIP1 bytes of FILE1 and
    pub const IGNORE_INITIAL: &str = "ignore-initial";
    // pub const IGNORE_INITIAL: &str = "SKIP[:SKIP2]";
    ///   -b, --print-bytes          print differing bytes
    pub const PRINT_BYTES: &str = "print-bytes";
    ///   -s, --quiet, --silent      suppress all normal output
    pub const QUIET: &str = "quiet";
    pub const SILENT: &str = "silent";
    ///   -l, --verbose              output byte numbers and differing byte values
    pub const VERBOSE: &str = "verbose";
}

/// Holds the given command line arguments except "--version" and "--help".
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Params {
    /// path or "-" for stdin
    pub from: OsString,
    pub to: OsString,
    /// -n, --bytes=LIMIT          compare at most LIMIT bytes
    /// cmp from diffutils has a limit of i64::MAX (9_223_372_036_854_775_807)
    pub bytes_limit: Option<BytesLimitU64>,
    /// -i, --ignore-initial=SKIP         skip first SKIP bytes of both inputs
    pub skip_bytes_from: Option<SkipU64>,
    /// -i, --ignore-initial=SKIP1:SKIP2  skip first SKIP1 bytes of FILE1 and
    pub skip_bytes_to: Option<SkipU64>,
    /// -b, --print-bytes          print differing bytes
    pub print_bytes: bool,
    /// -s, --quiet, --silent      suppress all normal output \
    /// Do not set directly, use set_silent().
    pub silent: bool,
    /// -l, --verbose              output byte numbers and differing byte values \
    /// Do not set directly, use set_verbose().
    pub verbose: bool,
}

impl Params {
    /// Sets the --bytes limit and returns the input as number.
    ///
    /// bytes - unparsed number string, e.g. '50KiB'
    pub fn set_bytes_limit(&mut self, num_unit: &str) -> Result<BytesLimitU64, ParseCmpError> {
        let num = Self::parse_num_bytes(num_unit).map_err(|e| {
            ParseCmpError::ParseSizeError(options::BYTES_LIMIT, num_unit.to_string(), e)
        })?;

        self.bytes_limit = Some(num);
        Ok(num)
    }

    pub fn set_print_bytes(&mut self, value: bool) -> Result<(), ParseCmpError> {
        // Should actually raise an error if --silent is set, but GNU cmp does not do that.
        if value && self.silent {
            return Err(ParseCmpError::OptionsIncompatible(
                options::PRINT_BYTES,
                options::SILENT,
            ));
        }
        self.print_bytes = value;

        Ok(())
    }

    /// Sets the ignore initial bytes for both files.
    ///
    /// Accepts digits[unit][:digits[unit]] \
    /// Sets the 2nd file to the value of the 1st file if no second parameter is given. \
    pub fn set_skip_bytes(&mut self, bytes: &str) -> Result<(), ParseCmpError> {
        // empty string is not checked

        // Split at ':' if present
        let (skip_1, skip_2) = match bytes.split_once(':') {
            Some((s1, s2)) => (s1, s2),
            None => {
                // set file_to to same value as file_from
                (bytes, bytes)
            }
        };

        self.set_skip_bytes_file_no(skip_1, 1)?;
        self.set_skip_bytes_file_no(skip_2, 2)?;

        Ok(())
    }

    /// Sets the [Self::skip_bytes_from] or [Self::skip_bytes_to] value.
    ///
    /// GNU cmp always uses the higher number in case of conflicting definitions
    /// with --ignore-initial and operand
    fn set_skip_bytes_file_no(
        &mut self,
        bytes_num_unit: &str,
        file_no: i32,
    ) -> Result<SkipU64, ParseCmpError> {
        let skip = match Self::parse_num_bytes(bytes_num_unit) {
            Ok(r) => r,
            Err(e) => {
                return Err(ParseCmpError::ParseSizeError(
                    options::IGNORE_INITIAL,
                    bytes_num_unit.to_string(),
                    e,
                ));
            }
        };
        match file_no {
            // use higher value
            1 => {
                self.skip_bytes_from = match self.skip_bytes_from {
                    Some(v) => Some(skip.max(v)),
                    None => Some(skip),
                }
            }
            2 => {
                self.skip_bytes_to = match self.skip_bytes_to {
                    Some(v) => Some(skip.max(v)),
                    None => Some(skip),
                }
            }
            _ => panic!("logic error"),
        }

        Ok(skip)
    }

    pub fn set_verbose(&mut self, value: bool) -> Result<(), ParseCmpError> {
        if value && self.silent {
            return Err(ParseCmpError::OptionsIncompatible(
                options::VERBOSE,
                options::SILENT,
            ));
        }
        self.verbose = value;
        Ok(())
    }

    /// Parse a SIZE string into a number of bytes.
    /// A size string comprises an integer and an optional unit.
    /// The unit may be k, K, m, M, g, G, t, T, P, E, Z, Y (powers of 1024), or b which is 1.
    /// Default is K.
    fn parse_num_bytes(input: &str) -> Result<SkipU64, ParseSizeError> {
        let size = Parser::default()
            .with_allow_list(&ALLOWED_UNITS)
            // .with_default_unit("K")
            // .with_b_byte_count(true)
            .parse(input.trim())?;

        SkipU64::try_from(size).map_err(|_| {
            // ParseSizeError::SizeTooBig(translate!("sort-error-buffer-size-too-big", "size" => size))
            ParseSizeError::SizeTooBig(input.to_string())
        })
    }
}

/// Converts clap args to params.
impl TryFrom<clap::ArgMatches> for Params {
    type Error = ParseCmpError;

    fn try_from(matches: clap::ArgMatches) -> Result<Self, Self::Error> {
        // dbg!(&matches);

        let mut params = Self {
            silent: matches.get_flag(options::SILENT) || matches.get_flag(options::QUIET),
            ..Default::default()
        };
        params.set_verbose(matches.get_flag(options::VERBOSE))?;
        params.set_print_bytes(matches.get_flag(options::PRINT_BYTES))?;

        // has bytes-limit?
        if let Some(byte_str) = matches
            .get_many::<String>(options::BYTES_LIMIT)
            .and_then(|mut iter| iter.next())
        {
            params.set_bytes_limit(byte_str)?;
        }

        // has ignore-initial?
        if let Some(skip_str) = matches
            .get_many::<String>(options::IGNORE_INITIAL)
            .and_then(|mut iter| iter.next())
        {
            // dbg!(&skip_str);
            params.set_skip_bytes(skip_str)?;
        }

        // get files
        let files: Vec<OsString> = match matches.get_many::<OsString>(options::FILE) {
            Some(v) => v.cloned().collect(),
            None => return Err(ParseCmpError::NoOperands(uucore::util_name().to_string())),
        };
        // dbg!(&files);

        match files.len() {
            0 => return Err(ParseCmpError::NoOperands(uucore::util_name().to_string())),
            // If only file_1 is set, then file_2 defaults to '-', so it reads from StandardInput.
            1 => {
                params.from.clone_from(&files[0]);
                params.to = "-".into();
            }
            2..=4 => {
                params.from.clone_from(&files[0]);
                params.to.clone_from(&files[1]);
                // ignore if ignore-initial is already set by option
                if files.len() > 2 {
                    params.set_skip_bytes_file_no(&files[2].to_string_lossy(), 1)?;
                    if let Some(skip) = files.get(3) {
                        params.set_skip_bytes_file_no(&skip.to_string_lossy(), 2)?;
                    }
                }
            }
            _ => {
                return Err(ParseCmpError::ExtraOperand(files[4].clone()));
            }
        }

        // Do as GNU cmp, and completely disable printing if we are
        // outputting to /dev/null.
        #[cfg(not(target_os = "windows"))]
        if is_stdout_dev_null() {
            params.silent = true;
            params.verbose = false;
            params.print_bytes = false;
        }

        // dbg!(&params);
        Ok(params)
    }
}

#[cfg(not(target_os = "windows"))]
fn is_stdout_dev_null() -> bool {
    use std::{
        fs, io,
        os::{fd::AsRawFd, unix::fs::MetadataExt},
    };

    let Ok(dev_null) = fs::metadata("/dev/null") else {
        return false;
    };

    let stdout_fd = io::stdout().lock().as_raw_fd();

    // SAFETY: we have exclusive access to stdout right now.
    let stdout_file = unsafe {
        use std::os::fd::FromRawFd;
        fs::File::from_raw_fd(stdout_fd)
    };
    let Ok(stdout) = stdout_file.metadata() else {
        return false;
    };

    let is_dev_null = stdout.dev() == dev_null.dev() && stdout.ino() == dev_null.ino();

    // Don't let File close the fd. It's unfortunate that File doesn't have a leak_fd().
    std::mem::forget(stdout_file);

    is_dev_null
}

/// Contains all parser errors and their text messages.
///
/// All errors can be output easily using the normal Display functionality.
/// To format the error message for the typical diffutils output, use [format_error_text].
#[derive(Debug, PartialEq)]
pub enum ParseCmpError {
    /// (Option, value, error)
    ParseSizeError(&'static str, String, ParseSizeError),

    /// Having more operands than the four allowed (file_1, file_2, ign_1, ign_2)
    ///
    /// Params: (wrong operand)
    ExtraOperand(OsString),

    /// No args for the cmp utility given.
    /// Requires at least one file (other will then be standard input).
    ///
    /// Params: (executable name)
    // TODO test stdin for windows
    NoOperands(String),

    /// Two options cannot be used together, e.g. cmp --silent and --verbose (output).
    OptionsIncompatible(&'static str, &'static str),
    // Error message for options available in GNU, but not yet here
    // NotYetImplemented(String),
}

impl std::error::Error for ParseCmpError {}

impl UError for ParseCmpError {
    fn code(&self) -> i32 {
        2
    }

    fn usage(&self) -> bool {
        // TODO should not returns full path on try --help message
        // Try '/home/gunnar/SynologyDrive/Development/diffutils_fork/target/debug/cmp --help' for more information.
        true
    }
}

impl std::fmt::Display for ParseCmpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::ParseSizeError(option, value, e) => match e {
                ParseSizeError::InvalidSuffix(_) => {
                    translate!(
                        "cmp-error-invalid-value-unit",
                        "option" => option,
                        "value" => value
                    )
                }
                ParseSizeError::ParseFailure(_) => {
                    translate!(
                        "cmp-error-invalid-value",
                        "option" => option,
                        "value" => value
                    )
                }
                ParseSizeError::SizeTooBig(_) => {
                    dbg!(translate!(
                        "cmp-error-invalid-value-overflow",
                        "option" => option,
                        "value" => value
                    ));
                    translate!(
                        "cmp-error-invalid-value-overflow",
                        "option" => option,
                        "value" => value
                    )
                }
                ParseSizeError::PhysicalMem(_value) => e.to_string(),
            },

            Self::ExtraOperand(extra_operand) => {
                translate!("base-common-extra-operand", "operand" => extra_operand.quote())
            }
            Self::NoOperands(_exe_name) => {
                translate!("cmp-error-missing-operands", "util_name" => uucore::util_name())
            }
            Self::OptionsIncompatible(option_1, option_2) => translate!(
                "cmp-error-incompatible-options",
                "opt1" => option_1,
                "opt2" => option_2,
            ),
            // Self::NotYetImplemented(s) => {
            //     translate!("cmp-error-not-yet-implemented", "option" => s)
            // }
        };
        write!(f, "{msg}")
    }
}

pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(uucore::format_usage(&translate!("cmp-usage")))
        .about(translate!("cmp-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::BYTES_LIMIT)
                .long("bytes")
                .short('n')
                .value_name("LIMIT")
                .action(ArgAction::Append)
                .help(translate!("cmp-help-bytes-limit")),
        )
        .arg(
            Arg::new(options::IGNORE_INITIAL)
                .long("ignore-initial")
                .short('i')
                .value_name("SKIP[:SKIP2]")
                .action(ArgAction::Append)
                .help(translate!("cmp-help-ignore-initial")),
        )
        .arg(
            Arg::new(options::PRINT_BYTES)
                .long("print-bytes")
                .short('b')
                .action(ArgAction::SetTrue)
                .help(translate!("cmp-help-print-bytes")),
        )
        .arg(
            Arg::new(options::QUIET)
                .long("quiet")
                .action(ArgAction::SetTrue)
                .help(translate!("cmp-help-quiet")),
        )
        .arg(
            Arg::new(options::SILENT)
                .long("silent")
                .short('s')
                .action(ArgAction::SetTrue)
                .help(translate!("cmp-help-silent")),
        )
        .arg(
            Arg::new(options::VERBOSE)
                .long("verbose")
                .short('l')
                .action(ArgAction::SetTrue)
                .help(translate!("cmp-help-verbose")),
        )
}
