// This file is part of the uutils diffutils package.
//
// For the full copyright and license information, please view the LICENSE-*
// files that was distributed with this source code.

// spell-checker:ignore GFMT GTYPE LFMT LTYPE TABSIZE

//! This is the parser for the cmp utility.
//!
//! It uses the parsed data clap provides and fills the [Params] for cmp.
//! It contains the allowed options, specific parsing logic and parsing error messages.
//!
use clap::{Arg, ArgAction, Command};
use std::ffi::OsString;
use std::fmt::Display;
use std::path::PathBuf;
use uucore::display::Quotable;
use uucore::parser::parse_size::ParseSizeError;
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

// Allowed utility arguments (options)
mod options {
    /// Generic option for files and other undefined operands
    pub const FILE: &str = "file";
    ///   -q, --brief                   report only when files differ
    pub const BRIEF: &str = "brief";
    ///       --color[=WHEN]       color output; WHEN is 'never', 'always', or 'auto';
    pub const COLOR: &str = "color";
    ///   -c, -C NUM, --context[=NUM]   output NUM (default 3) lines of copied context
    pub const CONTEXT_LINES: &str = "context";
    /// -C requires different handling
    pub const CONTEXT_LINES_UPPER: &str = "context_upper";
    ///   -e, --ed                      output an ed script
    pub const ED: &str = "ed";
    ///   -x, --exclude=PAT               exclude files that match PAT
    pub const EXCLUDE: &str = "exclude";
    ///   -X, --exclude-from=FILE         exclude files that match any pattern in FILE
    pub const EXCLUDE_FROM: &str = "exclude-from";
    ///   -t, --expand-tabs             expand tabs to spaces in output
    pub const EXPAND_TABS: &str = "expand-tabs";
    ///       --from-file=FILE1           compare FILE1 to all operands;
    pub const FROM_FILE: &str = "from-file";
    ///       --GTYPE-group-format=GFMT   format GTYPE input groups with GFMT
    pub const GTYPE_GROUP_FORMAT: &str = "gtype-group-format";
    ///       --horizon-lines=NUM  keep NUM lines of the common prefix and suffix
    pub const HORIZON_LINES: &str = "horizon-lines";
    ///   -D, --ifdef=NAME                output merged file with '#ifdef NAME' diffs
    pub const IFDEF: &str = "ifdef";
    ///   -w, --ignore-all-space          ignore all white space
    pub const IGNORE_ALL_SPACE: &str = "ignore-all-space";
    ///   -B, --ignore-blank-lines        ignore changes where lines are all blank
    pub const IGNORE_BLANK_LINES: &str = "ignore-blank-lines";
    ///   -i, --ignore-case               ignore case differences in file contents
    pub const IGNORE_CASE: &str = "ignore-case";
    ///       --ignore-file-name-case     ignore case when comparing file names
    pub const IGNORE_FILE_NAME_CASE: &str = "ignore-file-name-case";
    ///   -I, --ignore-matching-lines=RE  ignore changes where all lines match RE
    pub const IGNORE_MATCHING_LINES: &str = "ignore-matching-lines";
    ///   -b, --ignore-space-change       ignore changes in the amount of white space
    pub const IGNORE_SPACE_CHANGE: &str = "ignore-space-change";
    ///   -E, --ignore-tab-expansion      ignore changes due to tab expansion
    pub const IGNORE_TAB_EXPANSION: &str = "ignore-tab-expansion";
    ///   -Z, --ignore-trailing-space     ignore white space at line end
    pub const IGNORE_TRAILING_SPACE: &str = "ignore-trailing-space";
    ///   -T, --initial-tab             make tabs line up by prepending a tab
    pub const INITIAL_TAB: &str = "initial-tab";
    ///       --label LABEL             use LABEL instead of file name and timestamp
    pub const LABEL: &str = "label";
    ///       --left-column             output only the left column of common lines
    pub const LEFT_COLUMN: &str = "left-column";
    ///       --line-format=LFMT          format all input lines with LFMT
    pub const LINE_FORMAT: &str = "line-format";
    ///       --LTYPE-line-format=LFMT    format LTYPE input lines with LFMT
    pub const LTYPE_LINE_FORMAT: &str = "ltype-line-format";
    ///   -d, --minimal            try hard to find a smaller set of changes
    pub const MINIMAL: &str = "minimal";
    ///   -N, --new-file                  treat absent files as empty
    pub const NEW_FILE: &str = "new-file";
    ///       --no-dereference            don't follow symbolic links
    pub const NO_DEREFERENCE: &str = "no-dereference";
    ///       --no-ignore-file-name-case  consider case when comparing file names
    pub const NO_IGNORE_FILE_NAME_CASE: &str = "no-ignore-file-name-case";
    ///       --normal                  output a normal diff (the default)
    pub const NORMAL: &str = "normal";
    ///   -l, --paginate                pass output through 'pr' to paginate it
    pub const PAGINATE: &str = "paginate";
    ///       --palette=PALETTE    the colors to use when --color is active; PALETTE is
    pub const PALETTE: &str = "palette";
    ///   -n, --rcs                     output an RCS format diff
    pub const RCS: &str = "rcs";
    ///   -r, --recursive                 recursively compare any subdirectories found
    pub const RECURSIVE: &str = "recursive";
    ///   -s, --report-identical-files  report when two files are the same
    pub const REPORT_IDENTICAL_FILES: &str = "report-identical-files";
    ///   -p, --show-c-function         show which C function each change is in
    pub const SHOW_C_FUNCTION: &str = "show-c-function";
    ///   -F, --show-function-line=RE   show the most recent line matching RE
    pub const SHOW_FUNCTION_LINE: &str = "show-function-line";
    ///   -y, --side-by-side            output in two columns
    pub const SIDE_BY_SIDE: &str = "side-by-side";
    ///       --speed-large-files  assume large files and many scattered small changes
    pub const SPEED_LARGE_FILES: &str = "speed-large-files";
    ///   -S, --starting-file=FILE        start with FILE when comparing directories
    pub const STARTING_FILE: &str = "starting-file";
    ///       --strip-trailing-cr         strip trailing carriage return on input
    pub const STRIP_TRAILING_CR: &str = "strip-trailing-cr";
    ///       --suppress-blank-empty    suppress space or tab before empty output lines
    pub const SUPPRESS_BLANK_EMPTY: &str = "suppress-blank-empty";
    ///       --suppress-common-lines   do not output common lines
    pub const SUPPRESS_COMMON_LINES: &str = "suppress-common-lines";
    ///       --tabsize=NUM             tab stops every NUM (default 8) print columns
    pub const TABSIZE: &str = "tabsize";
    ///   -a, --text                      treat all files as text
    pub const TEXT: &str = "text";
    ///       --to-file=FILE2             compare all operands to FILE2;
    pub const TO_FILE: &str = "to-file";
    ///       --unidirectional-new-file   treat absent first files as empty
    pub const UNIDIRECTIONAL_NEW_FILE: &str = "unidirectional-new-file";
    ///   -u, -U NUM, --unified[=NUM]   output NUM (default 3) lines of unified context
    pub const UNIFIED_LINES: &str = "unified";
    ///   -U  requires different handling
    pub const UNIFIED_LINES_UPPER: &str = "unified_upper";
    ///   -W, --width=NUM               output at most NUM (default 130) print columns
    pub const WIDTH: &str = "width";
}

/// Output format
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Format {
    #[default]
    /// Default output
    Normal,
    Unified,
    Context,
    /// output in ed editor format
    Ed,
    /// output in two columns
    SideBySide,
}

impl From<&str> for Format {
    fn from(option: &str) -> Self {
        match option {
            options::NORMAL => Self::Normal,
            options::UNIFIED_LINES => Self::Unified,
            options::CONTEXT_LINES => Self::Context,
            options::ED => Self::Ed,
            options::SIDE_BY_SIDE => Self::SideBySide,
            _ => todo!("option '{option}' missing in match"),
        }
    }
}

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let opt = match self {
            Self::Normal => options::NORMAL,
            Self::Unified => options::UNIFIED_LINES,
            Self::Context => options::CONTEXT_LINES,
            Self::Ed => options::ED,
            Self::SideBySide => options::SIDE_BY_SIDE,
        };
        write!(f, "{opt}")
    }
}

/// Holds the given command line arguments except "--version" and "--help".
#[derive(Debug, Clone, PartialEq)]
pub struct Params {
    /// path or "-" for stdin
    pub from: OsString,
    pub to: OsString,
    /// report only when files differ
    pub brief: bool,
    /// color output; WHEN is 'never', 'always', or 'auto';
    pub color: Option<String>,
    /// output NUM (default 3) lines of copied context
    pub n_output_lines: usize,
    /// exclude files that match PAT
    pub exclude: Option<String>,
    /// exclude files that match any pattern in FILE
    pub exclude_from: Option<String>,
    /// expand tabs to spaces in output
    pub expand_tabs: bool,
    /// output format
    pub format_out: Format,
    /// compare FILE1 to all operands;
    pub from_file: Option<String>,
    /// format GTYPE input groups with GFMT
    pub gtype_group_format: Option<String>,
    /// keep NUM lines of the common prefix and suffix
    pub horizon_lines: Option<String>,
    /// output merged file with '#ifdef NAME' diffs
    pub ifdef: Option<String>,
    /// ignore all white space
    pub ignore_all_space: bool,
    /// ignore changes where lines are all blank
    pub ignore_blank_lines: bool,
    /// ignore case differences in file contents
    pub ignore_case: bool,
    /// ignore case when comparing file names
    pub ignore_file_name_case: bool,
    /// ignore changes where all lines match RE
    pub ignore_matching_lines: Option<String>,
    /// ignore changes in the amount of white space
    pub ignore_space_change: bool,
    /// ignore changes due to tab expansion
    pub ignore_tab_expansion: bool,
    /// ignore white space at line end
    pub ignore_trailing_space: bool,
    /// make tabs line up by prepending a tab
    pub initial_tab: bool,
    /// LABEL             use LABEL instead of file name and timestamp
    pub label: bool,
    /// output only the left column of common lines
    pub left_column: bool,
    /// format all input lines with LFMT
    pub line_format: Option<String>,
    /// format LTYPE input lines with LFMT
    pub ltype_line_format: Option<String>,
    /// try hard to find a smaller set of changes
    pub minimal: bool,
    /// treat absent files as empty
    pub new_file: bool,
    /// don't follow symbolic links
    pub no_dereference: bool,
    /// consider case when comparing file names
    pub no_ignore_file_name_case: bool,
    /// pass output through 'pr' to paginate it
    pub paginate: bool,
    /// the colors to use when --color is active; PALETTE is
    pub palette: Option<String>,
    /// output an RCS format diff
    pub rcs: bool,
    /// recursively compare any subdirectories found
    pub recursive: bool,
    /// report when two files are the same
    pub report_identical_files: bool,
    /// show which C function each change is in
    pub show_c_function: bool,
    /// show the most recent line matching RE
    pub show_function_line: Option<String>,
    /// assume large files and many scattered small changes
    pub speed_large_files: bool,
    /// start with FILE when comparing directories
    pub starting_file: Option<String>,
    /// strip trailing carriage return on input
    pub strip_trailing_cr: bool,
    /// suppress space or tab before empty output lines
    pub suppress_blank_empty: bool,
    /// do not output common lines
    pub suppress_common_lines: bool,
    /// tab stops every NUM (default 8) print columns
    pub tabsize: usize,
    /// treat all files as text
    pub text: bool,
    /// compare all operands to FILE2;
    pub to_file: Option<String>,
    /// treat absent first files as empty
    pub unidirectional_new_file: bool,
    // /// output NUM (default 3) lines of unified context
    // pub n_unified_lines: usize,
    /// output at most NUM (default 130) print columns
    pub width: usize,
}

#[allow(clippy::default_trait_access)]
impl Default for Params {
    fn default() -> Self {
        Self {
            from: Default::default(),
            to: Default::default(),
            brief: Default::default(),
            color: Default::default(),
            n_output_lines: 3,
            exclude: Default::default(),
            exclude_from: Default::default(),
            expand_tabs: Default::default(),
            format_out: Default::default(),
            from_file: Default::default(),
            gtype_group_format: Default::default(),
            horizon_lines: Default::default(),
            ifdef: Default::default(),
            ignore_all_space: Default::default(),
            ignore_blank_lines: Default::default(),
            ignore_case: Default::default(),
            ignore_file_name_case: Default::default(),
            ignore_matching_lines: Default::default(),
            ignore_space_change: Default::default(),
            ignore_tab_expansion: Default::default(),
            ignore_trailing_space: Default::default(),
            initial_tab: Default::default(),
            label: Default::default(),
            left_column: Default::default(),
            line_format: Default::default(),
            ltype_line_format: Default::default(),
            minimal: Default::default(),
            new_file: Default::default(),
            no_dereference: Default::default(),
            no_ignore_file_name_case: Default::default(),
            paginate: Default::default(),
            palette: Default::default(),
            rcs: Default::default(),
            recursive: Default::default(),
            report_identical_files: Default::default(),
            show_c_function: Default::default(),
            show_function_line: Default::default(),
            speed_large_files: Default::default(),
            starting_file: Default::default(),
            strip_trailing_cr: Default::default(),
            suppress_blank_empty: Default::default(),
            suppress_common_lines: Default::default(),
            tabsize: 8,
            text: Default::default(),
            to_file: Default::default(),
            unidirectional_new_file: Default::default(),
            // n_unified_lines: 3,
            width: 130,
        }
    }
}

impl Params {
    pub fn from_as_string_lossy(&self) -> String {
        self.from.to_string_lossy().to_string()
    }

    pub fn to_as_string_lossy(&self) -> String {
        self.to.to_string_lossy().to_string()
    }

    //     /// Sets the --bytes limit and returns the input as number.
    //     ///
    //     /// bytes - unparsed number string, e.g. '50KiB'
    //     pub fn set_bytes_limit(&mut self, num_unit: &str) -> Result<BytesLimitU64, ParseCmpError> {
    //         let num = Self::parse_num_bytes(num_unit).map_err(|e| {
    //             ParseCmpError::ParseSizeError(options::BYTES_LIMIT, num_unit.to_string(), e)
    //         })?;
    //
    //         self.bytes_limit = Some(num);
    //         Ok(num)
    //     }
    //
    //     pub fn set_print_bytes(&mut self, value: bool) -> Result<(), ParseCmpError> {
    //         // Should actually raise an error if --silent is set, but GNU cmp does not do that.
    //         if value && self.silent {
    //             return Err(ParseCmpError::OptionsIncompatible(
    //                 options::PRINT_BYTES,
    //                 options::SILENT,
    //             ));
    //         }
    //         self.print_bytes = value;
    //
    //         Ok(())
    //     }
    //
    //     /// Sets the ignore initial bytes for both files.
    //     ///
    //     /// Accepts digits[unit][:digits[unit]] \
    //     /// Sets the 2nd file to the value of the 1st file if no second parameter is given. \
    //     pub fn set_skip_bytes(&mut self, bytes: &str) -> Result<(), ParseCmpError> {
    //         // empty string is not checked
    //
    //         // Split at ':' if present
    //         let (skip_1, skip_2) = match bytes.split_once(':') {
    //             Some((s1, s2)) => (s1, s2),
    //             None => {
    //                 // set file_to to same value as file_from
    //                 (bytes, bytes)
    //             }
    //         };
    //
    //         self.set_skip_bytes_file_no(skip_1, 1)?;
    //         self.set_skip_bytes_file_no(skip_2, 2)?;
    //
    //         Ok(())
    //     }
    //
    //     /// Sets the [Self::skip_bytes_from] or [Self::skip_bytes_to] value.
    //     ///
    //     /// GNU cmp always uses the higher number in case of conflicting definitions
    //     /// with --ignore-initial and operand
    //     fn set_skip_bytes_file_no(
    //         &mut self,
    //         bytes_num_unit: &str,
    //         file_no: i32,
    //     ) -> Result<SkipU64, ParseCmpError> {
    //         let skip = match Self::parse_num_bytes(bytes_num_unit) {
    //             Ok(r) => r,
    //             Err(e) => {
    //                 return Err(ParseCmpError::ParseSizeError(
    //                     options::IGNORE_INITIAL,
    //                     bytes_num_unit.to_string(),
    //                     e,
    //                 ));
    //             }
    //         };
    //         match file_no {
    //             // use higher value
    //             1 => {
    //                 self.skip_bytes_from = match self.skip_bytes_from {
    //                     Some(v) => Some(skip.max(v)),
    //                     None => Some(skip),
    //                 }
    //             }
    //             2 => {
    //                 self.skip_bytes_to = match self.skip_bytes_to {
    //                     Some(v) => Some(skip.max(v)),
    //                     None => Some(skip),
    //                 }
    //             }
    //             _ => panic!("logic error"),
    //         }
    //
    //         Ok(skip)
    //     }

    pub fn set_format(
        format: &mut Option<Format>,
        option: &str,
        value: bool,
    ) -> Result<(), ParseDiffError> {
        if value {
            let new = option.into();
            match format {
                Some(f) => {
                    return Err(ParseDiffError::ConflictingOutputStyle(*f, new));
                }
                None => *format = Some(new),
            }
        }
        Ok(())
    }

    pub fn set_context_lines(
        format_out: &mut Option<Format>,
        params: &mut Self,
        context: &str,
    ) -> Result<(), ParseDiffError> {
        Self::set_format(format_out, options::CONTEXT_LINES, true)?;
        params.format_out = Format::Context;
        match context.parse::<usize>() {
            Ok(context_size) => {
                params.n_output_lines = context_size;
            }
            Err(_) => {
                // empty stays on default
                if !context.is_empty() {
                    return Err(ParseDiffError::InvalidContextLength(context.to_string()));
                }
            }
        }
        Ok(())
    }

    pub fn set_unified_lines(
        format_out: &mut Option<Format>,
        params: &mut Self,
        unified: &str,
    ) -> Result<(), ParseDiffError> {
        Self::set_format(format_out, options::UNIFIED_LINES, true)?;
        params.format_out = Format::Unified;
        match unified.parse::<usize>() {
            Ok(unified_size) => {
                params.n_output_lines = unified_size;
            }
            Err(_) => {
                // empty stays on default
                if !unified.is_empty() {
                    return Err(ParseDiffError::InvalidUnifiedLength(unified.to_string()));
                }
            }
        }
        Ok(())
    }

    //     /// Parse a SIZE string into a number of bytes.
    //     /// A size string comprises an integer and an optional unit.
    //     /// The unit may be k, K, m, M, g, G, t, T, P, E, Z, Y (powers of 1024), or b which is 1.
    //     /// Default is K.
    //     fn parse_num_bytes(input: &str) -> Result<SkipU64, ParseSizeError> {
    //         let size = Parser::default()
    //             .with_allow_list(&ALLOWED_UNITS)
    //             // .with_default_unit("K")
    //             // .with_b_byte_count(true)
    //             .parse(input.trim())?;
    //
    //         SkipU64::try_from(size).map_err(|_| {
    //             // ParseSizeError::SizeTooBig(translate!("sort-error-buffer-size-too-big", "size" => size))
    //             ParseSizeError::SizeTooBig(input.to_string())
    //         })
    //     }
}

/// Converts clap args to Params.
impl TryFrom<clap::ArgMatches> for Params {
    type Error = ParseDiffError;

    fn try_from(matches: clap::ArgMatches) -> Result<Self, Self::Error> {
        // dbg!(&matches);

        let mut params = Self {
            brief: matches.get_flag(options::BRIEF),
            expand_tabs: matches.get_flag(options::EXPAND_TABS),
            ignore_all_space: matches.get_flag(options::IGNORE_ALL_SPACE),
            ignore_blank_lines: matches.get_flag(options::IGNORE_BLANK_LINES),
            ignore_case: matches.get_flag(options::IGNORE_CASE),
            ignore_file_name_case: matches.get_flag(options::IGNORE_FILE_NAME_CASE),
            ignore_space_change: matches.get_flag(options::IGNORE_SPACE_CHANGE),
            ignore_tab_expansion: matches.get_flag(options::IGNORE_TAB_EXPANSION),
            ignore_trailing_space: matches.get_flag(options::IGNORE_TRAILING_SPACE),
            initial_tab: matches.get_flag(options::INITIAL_TAB),
            label: matches.get_flag(options::LABEL),
            left_column: matches.get_flag(options::LEFT_COLUMN),
            minimal: matches.get_flag(options::MINIMAL),
            new_file: matches.get_flag(options::NEW_FILE),
            no_dereference: matches.get_flag(options::NO_DEREFERENCE),
            no_ignore_file_name_case: matches.get_flag(options::NO_IGNORE_FILE_NAME_CASE),
            paginate: matches.get_flag(options::PAGINATE),
            rcs: matches.get_flag(options::RCS),
            recursive: matches.get_flag(options::RECURSIVE),
            report_identical_files: matches.get_flag(options::REPORT_IDENTICAL_FILES),
            show_c_function: matches.get_flag(options::SHOW_C_FUNCTION),
            speed_large_files: matches.get_flag(options::SPEED_LARGE_FILES),
            strip_trailing_cr: matches.get_flag(options::STRIP_TRAILING_CR),
            suppress_blank_empty: matches.get_flag(options::SUPPRESS_BLANK_EMPTY),
            suppress_common_lines: matches.get_flag(options::SUPPRESS_COMMON_LINES),
            text: matches.get_flag(options::TEXT),
            unidirectional_new_file: matches.get_flag(options::UNIDIRECTIONAL_NEW_FILE),
            ..Default::default()
        };

        // set output format
        let mut format_out = if matches.get_flag(options::NORMAL) {
            Some(Format::Normal)
        } else {
            None
        };
        Self::set_format(&mut format_out, options::ED, matches.get_flag(options::ED))?;
        Self::set_format(
            &mut format_out,
            options::SIDE_BY_SIDE,
            matches.get_flag(options::SIDE_BY_SIDE),
        )?;

        // has color?
        if let Some(color) = matches.get_one::<String>(options::COLOR) {
            params.color = Some(color.clone());
        }

        // has context?
        if let Some(context) = matches.get_one::<String>(options::CONTEXT_LINES) {
            Self::set_context_lines(&mut format_out, &mut params, context)?;
        }
        if let Some(context) = matches.get_one::<String>(options::CONTEXT_LINES_UPPER) {
            Self::set_context_lines(&mut format_out, &mut params, context)?;
        }

        // has exclude?
        if let Some(exclude) = matches.get_one::<String>(options::EXCLUDE) {
            params.exclude = Some(exclude.clone());
        }

        // has exclude_from?
        if let Some(exclude_from) = matches.get_one::<String>(options::EXCLUDE_FROM) {
            params.exclude_from = Some(exclude_from.clone());
        }

        // has from_file?
        if let Some(from_file) = matches.get_one::<String>(options::FROM_FILE) {
            params.from_file = Some(from_file.clone());
        }

        // has gtype_group_format?
        if let Some(gtype_group_format) = matches.get_one::<String>(options::GTYPE_GROUP_FORMAT) {
            params.gtype_group_format = Some(gtype_group_format.clone());
        }

        // has horizon_lines?
        if let Some(horizon_lines) = matches.get_one::<String>(options::HORIZON_LINES) {
            params.horizon_lines = Some(horizon_lines.clone());
        }

        // has ifdef?
        if let Some(ifdef) = matches.get_one::<String>(options::IFDEF) {
            params.ifdef = Some(ifdef.clone());
        }

        // has ignore_matching_lines?
        if let Some(ignore_matching_lines) =
            matches.get_one::<String>(options::IGNORE_MATCHING_LINES)
        {
            params.ignore_matching_lines = Some(ignore_matching_lines.clone());
        }

        // has line_format?
        if let Some(line_format) = matches.get_one::<String>(options::LINE_FORMAT) {
            params.line_format = Some(line_format.clone());
        }

        // has ltype_line_format?
        if let Some(ltype_line_format) = matches.get_one::<String>(options::LTYPE_LINE_FORMAT) {
            params.ltype_line_format = Some(ltype_line_format.clone());
        }

        // has palette?
        if let Some(palette) = matches.get_one::<String>(options::PALETTE) {
            params.palette = Some(palette.clone());
        }

        // has show_function_line?
        if let Some(show_function_line) = matches.get_one::<String>(options::SHOW_FUNCTION_LINE) {
            params.show_function_line = Some(show_function_line.clone());
        }

        // has starting_file?
        if let Some(starting_file) = matches.get_one::<String>(options::STARTING_FILE) {
            params.starting_file = Some(starting_file.clone());
        }

        // has tabsize?
        if let Some(tabsize) = matches.get_one::<String>(options::TABSIZE) {
            // params.tabsize = Some(tabsize.clone());
            params.tabsize = tabsize
                .parse::<usize>()
                // TODO error message
                .map_err(|_op| ParseDiffError::MissingOperands)?;
        }

        // has to_file?
        if let Some(to_file) = matches.get_one::<String>(options::TO_FILE) {
            params.to_file = Some(to_file.clone());
        }

        // has unified?
        if let Some(unified) = matches.get_one::<String>(options::UNIFIED_LINES) {
            Self::set_unified_lines(&mut format_out, &mut params, unified)?;
        }
        if let Some(unified) = matches.get_one::<String>(options::UNIFIED_LINES_UPPER) {
            Self::set_unified_lines(&mut format_out, &mut params, unified)?;
        }

        // has width?
        if let Some(width) = matches.get_one::<String>(options::WIDTH) {
            // params.width = Some(width.clone());
            // match width.parse::<usize>() {
            //     Ok(width) => {
            //         params.width = width;
            //         // next_param_consumed = true;
            //     }
            //     // Err(_) => return Err(format!("invalid context length '{context}'")),
            //     // TODO error
            //     Err(_) => return Err(ParseCmpError::NoOperands("exe".to_string())),
            // }
            params.width = width
                .parse::<usize>()
                .map_err(|_op| ParseDiffError::MissingOperands)?;
        }

        if let Some(format) = format_out {
            params.format_out = format;
        }

        // get files
        let files: Vec<OsString> = match matches.get_many::<OsString>(options::FILE) {
            Some(v) => v.cloned().collect(),
            None => {
                return Err(ParseDiffError::MissingOperands);
            }
        };
        // dbg!(&files);

        match files.len() {
            0 => return Err(ParseDiffError::MissingOperands),
            // If only file_1 is set, then file_2 defaults to '-', so it reads from StandardInput.
            1 => {
                return Err(ParseDiffError::MissingOperands);
            }
            2 => {
                // diff DIRECTORY FILE => diff DIRECTORY/FILE FILE
                // diff FILE DIRECTORY => diff FILE DIRECTORY/FILE
                let mut from_path = PathBuf::from(&files[0]);
                let mut to_path = PathBuf::from(&files[1]);

                if from_path.is_dir() && to_path.is_file() {
                    from_path.push(to_path.file_name().unwrap());
                } else if from_path.is_file() && to_path.is_dir() {
                    to_path.push(from_path.file_name().unwrap());
                }
                params.from = from_path.into_os_string();
                params.to = to_path.into_os_string();
            }
            _ => {
                // dbg!(&files);
                return Err(ParseDiffError::ExtraOperand(files[2].clone()));
            }
        }

        // not yet implemented error; delete when implemented
        if matches.get_one::<String>(options::COLOR).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::COLOR));
        }
        if matches.get_one::<String>(options::EXCLUDE).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::EXCLUDE));
        }
        if matches.get_one::<String>(options::EXCLUDE_FROM).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::EXCLUDE_FROM));
        }
        if matches.get_one::<String>(options::FROM_FILE).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::FROM_FILE));
        }
        if matches
            .get_one::<String>(options::GTYPE_GROUP_FORMAT)
            .is_some()
        {
            return Err(ParseDiffError::NotYetImplemented(
                options::GTYPE_GROUP_FORMAT,
            ));
        }
        if matches.get_one::<String>(options::HORIZON_LINES).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::HORIZON_LINES));
        }
        if matches.get_one::<String>(options::IFDEF).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::IFDEF));
        }
        if matches.get_flag(options::IGNORE_ALL_SPACE) {
            return Err(ParseDiffError::NotYetImplemented(options::IGNORE_ALL_SPACE));
        }
        if matches.get_flag(options::IGNORE_BLANK_LINES) {
            return Err(ParseDiffError::NotYetImplemented(
                options::IGNORE_BLANK_LINES,
            ));
        }
        if matches.get_flag(options::IGNORE_CASE) {
            return Err(ParseDiffError::NotYetImplemented(options::IGNORE_CASE));
        }
        if matches.get_flag(options::IGNORE_FILE_NAME_CASE) {
            return Err(ParseDiffError::NotYetImplemented(
                options::IGNORE_FILE_NAME_CASE,
            ));
        }
        if matches
            .get_one::<String>(options::IGNORE_MATCHING_LINES)
            .is_some()
        {
            return Err(ParseDiffError::NotYetImplemented(
                options::IGNORE_MATCHING_LINES,
            ));
        }
        if matches.get_flag(options::IGNORE_SPACE_CHANGE) {
            return Err(ParseDiffError::NotYetImplemented(
                options::IGNORE_SPACE_CHANGE,
            ));
        }
        if matches.get_flag(options::IGNORE_TAB_EXPANSION) {
            return Err(ParseDiffError::NotYetImplemented(
                options::IGNORE_TAB_EXPANSION,
            ));
        }
        if matches.get_flag(options::IGNORE_TRAILING_SPACE) {
            return Err(ParseDiffError::NotYetImplemented(
                options::IGNORE_TRAILING_SPACE,
            ));
        }
        if matches.get_flag(options::INITIAL_TAB) {
            return Err(ParseDiffError::NotYetImplemented(options::INITIAL_TAB));
        }
        if matches.get_flag(options::LABEL) {
            return Err(ParseDiffError::NotYetImplemented(options::LABEL));
        }
        if matches.get_flag(options::LEFT_COLUMN) {
            return Err(ParseDiffError::NotYetImplemented(options::LEFT_COLUMN));
        }
        if matches.get_one::<String>(options::LINE_FORMAT).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::LINE_FORMAT));
        }
        if matches
            .get_one::<String>(options::LTYPE_LINE_FORMAT)
            .is_some()
        {
            return Err(ParseDiffError::NotYetImplemented(
                options::LTYPE_LINE_FORMAT,
            ));
        }
        if matches.get_flag(options::MINIMAL) {
            return Err(ParseDiffError::NotYetImplemented(options::MINIMAL));
        }
        if matches.get_flag(options::NEW_FILE) {
            return Err(ParseDiffError::NotYetImplemented(options::NEW_FILE));
        }
        if matches.get_flag(options::NO_DEREFERENCE) {
            return Err(ParseDiffError::NotYetImplemented(options::NO_DEREFERENCE));
        }
        if matches.get_flag(options::NO_IGNORE_FILE_NAME_CASE) {
            return Err(ParseDiffError::NotYetImplemented(
                options::NO_IGNORE_FILE_NAME_CASE,
            ));
        }
        if matches.get_flag(options::PAGINATE) {
            return Err(ParseDiffError::NotYetImplemented(options::PAGINATE));
        }
        if matches.get_one::<String>(options::PALETTE).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::PALETTE));
        }
        if matches.get_flag(options::RCS) {
            return Err(ParseDiffError::NotYetImplemented(options::RCS));
        }
        if matches.get_flag(options::RECURSIVE) {
            return Err(ParseDiffError::NotYetImplemented(options::RECURSIVE));
        }
        if matches.get_flag(options::SHOW_C_FUNCTION) {
            return Err(ParseDiffError::NotYetImplemented(options::SHOW_C_FUNCTION));
        }
        if matches
            .get_one::<String>(options::SHOW_FUNCTION_LINE)
            .is_some()
        {
            return Err(ParseDiffError::NotYetImplemented(
                options::SHOW_FUNCTION_LINE,
            ));
        }
        if matches.get_flag(options::SPEED_LARGE_FILES) {
            return Err(ParseDiffError::NotYetImplemented(
                options::SPEED_LARGE_FILES,
            ));
        }
        if matches.get_one::<String>(options::STARTING_FILE).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::STARTING_FILE));
        }
        if matches.get_flag(options::STRIP_TRAILING_CR) {
            return Err(ParseDiffError::NotYetImplemented(
                options::STRIP_TRAILING_CR,
            ));
        }
        if matches.get_flag(options::SUPPRESS_BLANK_EMPTY) {
            return Err(ParseDiffError::NotYetImplemented(
                options::SUPPRESS_BLANK_EMPTY,
            ));
        }
        if matches.get_flag(options::SUPPRESS_COMMON_LINES) {
            return Err(ParseDiffError::NotYetImplemented(
                options::SUPPRESS_COMMON_LINES,
            ));
        }
        if matches.get_flag(options::TEXT) {
            return Err(ParseDiffError::NotYetImplemented(options::TEXT));
        }
        if matches.get_one::<String>(options::TO_FILE).is_some() {
            return Err(ParseDiffError::NotYetImplemented(options::TO_FILE));
        }
        if matches.get_flag(options::UNIDIRECTIONAL_NEW_FILE) {
            return Err(ParseDiffError::NotYetImplemented(
                options::UNIDIRECTIONAL_NEW_FILE,
            ));
        }

        // dbg!(&params);
        Ok(params)
    }
}

// #[cfg(not(target_os = "windows"))]
// fn is_stdout_dev_null() -> bool {
//     use std::{
//         fs, io,
//         os::{fd::AsRawFd, unix::fs::MetadataExt},
//     };
//
//     let Ok(dev_null) = fs::metadata("/dev/null") else {
//         return false;
//     };
//
//     let stdout_fd = io::stdout().lock().as_raw_fd();
//
//     // SAFETY: we have exclusive access to stdout right now.
//     let stdout_file = unsafe {
//         use std::os::fd::FromRawFd;
//         fs::File::from_raw_fd(stdout_fd)
//     };
//     let Ok(stdout) = stdout_file.metadata() else {
//         return false;
//     };
//
//     let is_dev_null = stdout.dev() == dev_null.dev() && stdout.ino() == dev_null.ino();
//
//     // Don't let File close the fd. It's unfortunate that File doesn't have a leak_fd().
//     std::mem::forget(stdout_file);
//
//     is_dev_null
// }

/// Contains all parser errors and their text messages.
///
/// All errors can be output easily using the normal Display functionality.
/// To format the error message for the typical diffutils output, use [format_error_text].
#[derive(Debug, PartialEq)]
pub enum ParseDiffError {
    /// (Option, value, error)
    ParseSizeError(&'static str, String, ParseSizeError),

    /// (Format options)
    ConflictingOutputStyle(Format, Format),

    /// Having more operands than the four allowed (file_1, file_2, ign_1, ign_2)
    ///
    /// Params: (wrong operand)
    ExtraOperand(OsString),

    InvalidContextLength(String),
    InvalidUnifiedLength(String),

    /// No args for the diff utility given.
    /// Requires at least two files.
    MissingOperands,

    /// Two options cannot be used together, e.g. cmp --silent and --verbose (output).
    OptionsIncompatible(&'static str, &'static str),

    /// Error message for options available in GNU, but not yet here
    NotYetImplemented(&'static str),
}

impl std::error::Error for ParseDiffError {}

impl UError for ParseDiffError {
    fn code(&self) -> i32 {
        2
    }

    fn usage(&self) -> bool {
        // TODO should not returns full path on try --help message
        // Try '/home/gunnar/SynologyDrive/Development/diffutils_fork/target/debug/cmp --help' for more information.
        true
    }
}

impl std::fmt::Display for ParseDiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::ParseSizeError(option, value, e) => match e {
                ParseSizeError::InvalidSuffix(_) => {
                    translate!(
                        "diff-error-invalid-value-unit",
                        "option" => option,
                        "value" => value
                    )
                }
                ParseSizeError::ParseFailure(_) => {
                    translate!(
                        "diff-error-invalid-value",
                        "option" => option,
                        "value" => value
                    )
                }
                ParseSizeError::SizeTooBig(_) => {
                    // dbg!(translate!(
                    //     "diff-error-invalid-value-overflow",
                    //     "option" => option,
                    //     "value" => value
                    // ));
                    translate!(
                        "diff-error-invalid-value-overflow",
                        "option" => option,
                        "value" => value
                    )
                }
                ParseSizeError::PhysicalMem(_value) => e.to_string(),
            },

            Self::ConflictingOutputStyle(opt_1, opt_2) => {
                translate!("diff-error-conflicting-output-options", "opt1" => opt_1, "opt2" => opt_2)
            }
            Self::ExtraOperand(extra_operand) => {
                translate!("base-common-extra-operand", "operand" => extra_operand.quote())
            }
            Self::InvalidContextLength(value) => {
                translate!("diff-error-invalid-context-length", "value" => value)
            }
            Self::InvalidUnifiedLength(value) => {
                translate!("diff-error-invalid-unified-length", "value" => value)
            }
            Self::MissingOperands => {
                translate!("diff-error-missing-operands", "util_name" => uucore::util_name())
            }
            Self::OptionsIncompatible(option_1, option_2) => translate!(
                "diff-error-incompatible-options",
                "opt1" => option_1,
                "opt2" => option_2,
            ),
            Self::NotYetImplemented(s) => {
                translate!("diff-error-not-yet-implemented", "option" => s)
            }
        };
        write!(f, "{msg}")
    }
}

// pub fn uu_app() -> Command {
//     Command::new(uucore::util_name())
//         .version(uucore::crate_version!())
//         .help_template(uucore::localized_help_template(uucore::util_name()))
//         .override_usage(uucore::format_usage(&translate!("diff-usage")))
//         .about(translate!("diff-about"))
//         .infer_long_args(true)
//         .arg(
//             Arg::new(options::FILE)
//                 .action(ArgAction::Set)
//                 .hide(true)
//                 .value_hint(clap::ValueHint::FilePath)
//                 .value_parser(clap::value_parser!(OsString)),
//         )
//         .arg(
//             Arg::new(options::BYTES_LIMIT)
//                 .long("bytes")
//                 .short('n')
//                 .value_name("LIMIT")
//                 .action(ArgAction::Set)
//                 .help(translate!("diff-help-bytes-limit")),
//         )
//         .arg(
//             Arg::new(options::IGNORE_INITIAL)
//                 .long("ignore-initial")
//                 .short('i')
//                 .value_name("SKIP[:SKIP2]")
//                 .action(ArgAction::Set)
//                 .help(translate!("diff-help-ignore-initial")),
//         )
//         .arg(
//             Arg::new(options::PRINT_BYTES)
//                 .long("print-bytes")
//                 .short('b')
//                 .action(ArgAction::SetTrue)
//                 .help(translate!("diff-help-print-bytes")),
//         )
//         .arg(
//             Arg::new(options::QUIET)
//                 .long("quiet")
//                 .action(ArgAction::SetTrue)
//                 .help(translate!("diff-help-quiet")),
//         )
//         .arg(
//             Arg::new(options::SILENT)
//                 .long("silent")
//                 .short('s')
//                 .action(ArgAction::SetTrue)
//                 .help(translate!("diff-help-silent")),
//         )
//         .arg(
//             Arg::new(options::VERBOSE)
//                 .long("verbose")
//                 .short('l')
//                 .action(ArgAction::SetTrue)
//                 .help(translate!("diff-help-verbose")),
//         )
// }

// Required for build.rs
// pub fn uu_app() -> Command {
//     Command::new(uucore::util_name())
//         .version(uucore::crate_version!())
//         .help_template(uucore::localized_help_template(uucore::util_name()))
//         .override_usage(uucore::format_usage(&translate!("diff-usage")))
//         .about(translate!("diff-about"))
//         .infer_long_args(true)
//         .arg(
//             Arg::new(options::FILE)
//                 .action(ArgAction::Set)
//                 .hide(true)
//                 .value_hint(clap::ValueHint::FilePath)
//                 .value_parser(clap::value_parser!(OsString)),
//         )
// }

// uu_app .args for the options
pub fn uu_app() -> Command {
    Command::new(uucore::util_name())
        .version(uucore::crate_version!())
        .help_template(uucore::localized_help_template(uucore::util_name()))
        .override_usage(uucore::format_usage(&translate!("diff-usage")))
        .about(translate!("diff-about"))
        .infer_long_args(true)
        .arg(
            Arg::new(options::FILE)
                .action(ArgAction::Append)
                .hide(true)
                .value_hint(clap::ValueHint::FilePath)
                .value_parser(clap::value_parser!(OsString)),
        )
        .arg(
            Arg::new(options::BRIEF)
                .long("brief")
                .short('q')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-brief")),
        )
        .arg(
            Arg::new(options::COLOR)
                .long("color")
                .value_name("WHEN]")
                .action(ArgAction::Set)
                .help(translate!("diff-help-color")),
        )
        .arg(
            Arg::new(options::CONTEXT_LINES)
                .long("context")
                .short('c')
                .value_name("NUM")
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value("3")
                .action(ArgAction::Set)
                // TODO help text
                .help(translate!("diff-help-context")),
        )
        .arg(
            Arg::new(options::CONTEXT_LINES_UPPER)
                .short('C')
                .value_name("NUM")
                .action(ArgAction::Set)
                .help(translate!("diff-help-context")),
        )
        .arg(
            Arg::new(options::ED)
                .long("ed")
                .short('e')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-ed")),
        )
        .arg(
            Arg::new(options::EXCLUDE)
                .long("exclude")
                .short('x')
                .value_name("PAT")
                .action(ArgAction::Set)
                .help(translate!("diff-help-exclude")),
        )
        .arg(
            Arg::new(options::EXCLUDE_FROM)
                .long("exclude-from")
                .short('X')
                .value_name("FILE")
                .action(ArgAction::Set)
                .help(translate!("diff-help-exclude-from")),
        )
        .arg(
            Arg::new(options::EXPAND_TABS)
                .long("expand-tabs")
                .short('t')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-expand-tabs")),
        )
        .arg(
            Arg::new(options::FROM_FILE)
                .long("from-file")
                .value_name("FILE1")
                .action(ArgAction::Set)
                .help(translate!("diff-help-from-file")),
        )
        .arg(
            Arg::new(options::GTYPE_GROUP_FORMAT)
                .long("gtype-group-format")
                .value_name("GFMT")
                .action(ArgAction::Set)
                .help(translate!("diff-help-gtype-group-format")),
        )
        .arg(
            Arg::new(options::HORIZON_LINES)
                .long("horizon-lines")
                .value_name("NUM")
                .action(ArgAction::Set)
                .help(translate!("diff-help-horizon-lines")),
        )
        .arg(
            Arg::new(options::IFDEF)
                .long("ifdef")
                .short('D')
                .value_name("NAME")
                .action(ArgAction::Set)
                .help(translate!("diff-help-ifdef")),
        )
        .arg(
            Arg::new(options::IGNORE_ALL_SPACE)
                .long("ignore-all-space")
                .short('w')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-ignore-all-space")),
        )
        .arg(
            Arg::new(options::IGNORE_BLANK_LINES)
                .long("ignore-blank-lines")
                .short('B')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-ignore-blank-lines")),
        )
        .arg(
            Arg::new(options::IGNORE_CASE)
                .long("ignore-case")
                .short('i')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-ignore-case")),
        )
        .arg(
            Arg::new(options::IGNORE_FILE_NAME_CASE)
                .long("ignore-file-name-case")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-ignore-file-name-case")),
        )
        .arg(
            Arg::new(options::IGNORE_MATCHING_LINES)
                .long("ignore-matching-lines")
                .short('I')
                .value_name("RE")
                .action(ArgAction::Set)
                .help(translate!("diff-help-ignore-matching-lines")),
        )
        .arg(
            Arg::new(options::IGNORE_SPACE_CHANGE)
                .long("ignore-space-change")
                .short('b')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-ignore-space-change")),
        )
        .arg(
            Arg::new(options::IGNORE_TAB_EXPANSION)
                .long("ignore-tab-expansion")
                .short('E')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-ignore-tab-expansion")),
        )
        .arg(
            Arg::new(options::IGNORE_TRAILING_SPACE)
                .long("ignore-trailing-space")
                .short('Z')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-ignore-trailing-space")),
        )
        .arg(
            Arg::new(options::INITIAL_TAB)
                .long("initial-tab")
                .short('T')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-initial-tab")),
        )
        .arg(
            Arg::new(options::LABEL)
                .long("label")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-label")),
        )
        .arg(
            Arg::new(options::LEFT_COLUMN)
                .long("left-column")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-left-column")),
        )
        .arg(
            Arg::new(options::LINE_FORMAT)
                .long("line-format")
                .value_name("LFMT")
                .action(ArgAction::Set)
                .help(translate!("diff-help-line-format")),
        )
        .arg(
            Arg::new(options::LTYPE_LINE_FORMAT)
                .long("ltype-line-format")
                .value_name("LFMT")
                .action(ArgAction::Set)
                .help(translate!("diff-help-ltype-line-format")),
        )
        .arg(
            Arg::new(options::MINIMAL)
                .long("minimal")
                .short('d')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-minimal")),
        )
        .arg(
            Arg::new(options::NEW_FILE)
                .long("new-file")
                .short('N')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-new-file")),
        )
        .arg(
            Arg::new(options::NO_DEREFERENCE)
                .long("no-dereference")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-no-dereference")),
        )
        .arg(
            Arg::new(options::NO_IGNORE_FILE_NAME_CASE)
                .long("no-ignore-file-name-case")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-no-ignore-file-name-case")),
        )
        .arg(
            Arg::new(options::NORMAL)
                .long("normal")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-normal")),
        )
        .arg(
            Arg::new(options::PAGINATE)
                .long("paginate")
                .short('l')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-paginate")),
        )
        .arg(
            Arg::new(options::PALETTE)
                .long("palette")
                .value_name("PALETTE")
                .action(ArgAction::Set)
                .help(translate!("diff-help-palette")),
        )
        .arg(
            Arg::new(options::RCS)
                .long("rcs")
                .short('n')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-rcs")),
        )
        .arg(
            Arg::new(options::RECURSIVE)
                .long("recursive")
                .short('r')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-recursive")),
        )
        .arg(
            Arg::new(options::REPORT_IDENTICAL_FILES)
                .long("report-identical-files")
                .short('s')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-report-identical-files")),
        )
        .arg(
            Arg::new(options::SHOW_C_FUNCTION)
                .long("show-c-function")
                .short('p')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-show-c-function")),
        )
        .arg(
            Arg::new(options::SHOW_FUNCTION_LINE)
                .long("show-function-line")
                .short('F')
                .value_name("RE")
                .action(ArgAction::Set)
                .help(translate!("diff-help-show-function-line")),
        )
        .arg(
            Arg::new(options::SIDE_BY_SIDE)
                .long("side-by-side")
                .short('y')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-side-by-side")),
        )
        .arg(
            Arg::new(options::SPEED_LARGE_FILES)
                .long("speed-large-files")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-speed-large-files")),
        )
        .arg(
            Arg::new(options::STARTING_FILE)
                .long("starting-file")
                .short('S')
                .value_name("FILE")
                .action(ArgAction::Set)
                .help(translate!("diff-help-starting-file")),
        )
        .arg(
            Arg::new(options::STRIP_TRAILING_CR)
                .long("strip-trailing-cr")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-strip-trailing-cr")),
        )
        .arg(
            Arg::new(options::SUPPRESS_BLANK_EMPTY)
                .long("suppress-blank-empty")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-suppress-blank-empty")),
        )
        .arg(
            Arg::new(options::SUPPRESS_COMMON_LINES)
                .long("suppress-common-lines")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-suppress-common-lines")),
        )
        .arg(
            Arg::new(options::TABSIZE)
                .long("tabsize")
                .value_name("NUM")
                .action(ArgAction::Set)
                .help(translate!("diff-help-tabsize")),
        )
        .arg(
            Arg::new(options::TEXT)
                .long("text")
                .short('a')
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-text")),
        )
        .arg(
            Arg::new(options::TO_FILE)
                .long("to-file")
                .value_name("FILE2")
                .action(ArgAction::Set)
                .help(translate!("diff-help-to-file")),
        )
        .arg(
            Arg::new(options::UNIDIRECTIONAL_NEW_FILE)
                .long("unidirectional-new-file")
                .action(ArgAction::SetTrue)
                .help(translate!("diff-help-unidirectional-new-file")),
        )
        .arg(
            Arg::new(options::UNIFIED_LINES)
                .long("unified")
                .short('u')
                .value_name("NUM")
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value("3")
                .action(ArgAction::Set)
                .help(translate!("diff-help-unified")),
        )
        .arg(
            Arg::new(options::UNIFIED_LINES_UPPER)
                .short('U')
                .value_name("NUM")
                .action(ArgAction::Set)
                // TODO help text
                .help(translate!("diff-help-unified")),
        )
        .arg(
            Arg::new(options::WIDTH)
                .long("width")
                .short('W')
                .value_name("NUM")
                // .allow_negative_numbers(yes)
                .action(ArgAction::Set)
                .help(translate!("diff-help-width")),
        )
}
