// This file is part of the uutils diffutils package.
//
// For the full copyright and license information, please view the LICENSE-*
// files that was distributed with this source code.

pub mod context_diff;
pub mod ed_diff;
pub mod normal_diff;
pub mod params;
pub mod params_diff;
pub mod side_diff;
pub mod unified_diff;

use crate::params_diff::{Format, Params};
use clap::Command;
use std::ffi::OsString;
use std::io::{Read, Write, stdout};
use std::{fs, io};
use uudiff::common_errors::UtilsError;
use uudiff::error::{FromIo, UResult};
use uudiff::translate;

// Exit codes are documented at
// https://www.gnu.org/software/diffutils/manual/html_node/Invoking-diff.html.
//     An exit status of 0 means no differences were found,
//     1 means some differences were found,
//     and 2 means trouble.
#[uucore::main]
pub fn uumain(args: impl uucore::Args) -> UResult<()> {
    let args_checked = clap_preparation(args);
    let matches =
        uudiff::clap_localization::handle_clap_result_with_exit_code(uu_app(), args_checked, 2)?;
    // dbg!(&matches);

    let params: Params = matches.try_into()?;
    // dbg!(params.format_out);

    // let mut args = uucore::args_os().peekable();
    // if args.peek().unwrap().to_string_lossy().ends_with("utils") {
    //     args.next();
    // }
    // args.next();
    // let params_old = match parse_params(args) {
    //     Ok(p) => p,
    //     Err(error) => {
    //         eprintln!("{error}");
    //         uucore::error::set_exit_code(2);
    //         return Ok(());
    //     }
    // };
    // dbg!(&params_old);

    diff_compare(&params)?;

    // match diff_compare(&params) {
    //     Ok(_) => todo!(),
    //     Err(e) => {
    //         dbg!(&e, e.code());
    //         return Err(e);
    //     }
    // }

    Ok(())
}

pub fn clap_preparation(args: impl uucore::Args) -> Vec<OsString> {
    // handle constellations, clap can't do
    // so clap is limited to -c=num, while GNU allows -c42 and -42c (and 4c2)
    let mut args_checked = Vec::new();
    for mut arg_os in args {
        if arg_os.len() > 2 {
            let arg = arg_os.to_string_lossy();
            if arg.as_bytes()[0] == b'-' {
                // short options with num or multiple short options
                let mut opt = '-';
                let mut num = String::new();
                let mut ok = false;
                // let c = arg.as_bytes()[1] as char;
                for c in arg.chars().skip(1) {
                    if c.is_ascii_digit() {
                        num.push(c);
                    } else if c.is_ascii_lowercase() {
                        // possibly multi-single-options, e.g. -sc4 is valid
                        if c == 'c' || c == 'u' {
                            if opt == '-' {
                                opt = c;
                                ok = true;
                            } else {
                                // multiple chars, reject
                                ok = false;
                                break;
                            }
                        }
                    } else {
                        // unknown char, reject
                        ok = false;
                        break;
                    }
                }
                if ok {
                    // create c=42 structure
                    let mut s = String::from("-");
                    s.push(opt);
                    s.push('=');
                    s.push_str(&num);
                    arg_os = s.into();
                }
            }
        }
        // dbg!(&arg_os);
        args_checked.push(arg_os);
    }

    args_checked
}

pub fn diff_compare(params: &Params) -> UResult<()> {
    // if from and to are the same file, no need to perform any comparison
    let maybe_report_identical_files = || {
        if params.report_identical_files {
            let msg = translate!("diff-info-files-are-identical", 
                "file_1" => params.from.to_string_lossy(), 
                "file_2" => params.to.to_string_lossy());
            println!("{msg}");
        }
    };
    if params.from == "-" && params.to == "-"
        || same_file::is_same_file(&params.from, &params.to).unwrap_or(false)
    {
        maybe_report_identical_files();
        // ExitCode::SUCCESS;
        uucore::error::set_exit_code(0);
        return Ok(());
    }

    // read files
    fn read_file_contents(filepath: &OsString) -> io::Result<Vec<u8>> {
        if filepath == "-" {
            let mut content = Vec::new();
            io::stdin().read_to_end(&mut content).and(Ok(content))
        } else {
            fs::read(filepath)
        }
    }

    // UIoError has no code https://github.com/uutils/coreutils/issues/11453
    let r_from_content =
        // read_file_contents(&params.from).map_err(|e| UIoError::new_code(e, params.from.quote().to_string(), 2))?;
        read_file_contents(&params.from);
    // read_file_contents(&params.from);
    // read_file_contents(&params.from).map_err(|e| {let mut io = UIoError::from(e); io.context =Some(params.from.to_string_lossy().to_string()); DiffError::Io(io)});
    let r_to_content = read_file_contents(&params.to);

    // Diff returns both errors
    let from_content = match r_from_content {
        Ok(c) => c,
        Err(e1) => match r_to_content {
            Ok(_) => {
                let io = e1.map_err_context(|| params.from_as_string_lossy());
                return Err(UtilsError::Io(io).into());
            }
            Err(e2) => {
                let io1 = e1.map_err_context(|| params.from_as_string_lossy());
                let io2 = e2.map_err_context(|| params.to_as_string_lossy());
                return Err(UtilsError::IoDouble(io1, io2).into());
            }
        },
    };
    let to_content = match r_to_content {
        Ok(c) => c,
        Err(e2) => {
            let io = e2.map_err_context(|| params.to_as_string_lossy());
            return Err(UtilsError::Io(io).into());
        }
    };

    // run diff
    let result: Vec<u8> = match params.format_out {
        Format::Normal => normal_diff::diff(&from_content, &to_content, params),
        Format::Unified => unified_diff::diff(&from_content, &to_content, params),
        Format::Context => context_diff::diff(&from_content, &to_content, params),
        Format::Ed => ed_diff::diff(&from_content, &to_content, params)?,
        // .unwrap_or_else(|error| {
        //     // eprintln!("{error}");
        //     // uucore::error::set_exit_code(2);
        //     // std::process::exit(2);
        //     return super::Err(error);
        // }),
        Format::SideBySide => {
            let mut output = stdout().lock();
            side_diff::diff(&from_content, &to_content, &mut output, params)
        }
    };
    if params.brief && !result.is_empty() {
        let msg = translate!("diff-info-files-are-different", 
                "file_1" => params.from.to_string_lossy(), 
                "file_2" => params.to.to_string_lossy());
        println!("{msg}");
    } else {
        let result = io::stdout().write_all(&result);
        match result {
            // This code is adapted from coreutils.
            // <https://github.com/uutils/coreutils/blob/main/src/uu/seq/src/seq.rs>
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {
                // GNU seq prints the Broken pipe message but still exits with status 0
                // unless SIGPIPE was explicitly ignored, in which case it should fail.
                let err = err.map_err_context(|| "write error".into());
                uucore::show_error!("{err}");
                #[cfg(unix)]
                if uucore::signals::sigpipe_was_ignored() {
                    uucore::error::set_exit_code(0);
                }
            }
            Err(error) => {
                eprintln!("{}", uucore::error::strip_errno(&error));
                uucore::error::set_exit_code(2);
                return Ok(());
            }
        }
    }
    if result.is_empty() {
        maybe_report_identical_files();
        // ExitCode::SUCCESS;
        uucore::error::set_exit_code(0);
    } else {
        uucore::error::set_exit_code(1);
    }

    Ok(())
}

/// Contains all diff errors and their text messages.
///
/// All errors can be output easily using the normal Display functionality.
/// To format the error message for the typical diffutils output, use [format_error_text].
#[derive(Debug, PartialEq, Eq)]
pub enum DiffError {
    MissingNL,
}

impl std::error::Error for DiffError {}

impl uudiff::error::UError for DiffError {
    fn code(&self) -> i32 {
        2
    }
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::MissingNL => translate!("diff-error-missing-newline"),
        };

        write!(f, "{msg}")
    }
}

// Required for build.rs
pub fn uu_app() -> Command {
    crate::params_diff::uu_app()
}
