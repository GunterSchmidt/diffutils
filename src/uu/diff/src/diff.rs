// This file is part of the uutils diffutils package.
//
// For the full copyright and license information, please view the LICENSE-*
// files that was distributed with this source code.

pub mod context_diff;
pub mod ed_diff;
pub mod normal_diff;
pub mod params;
pub mod parser_diff;
pub mod side_diff;
pub mod unified_diff;

use crate::parser_diff::{Format, Params};
use clap::Command;
use std::ffi::OsString;
use std::io::{Read, Write, stdout};
use std::{fs, io};
use uudiff::error::{FromIo, UError, UResult};

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

pub fn clap_preparation(mut args: impl uucore::Args) -> Vec<OsString> {
    // handle constellations, clap can't do
    // so clap is limited to -c=num, while GNU allows -c42 and -42c (and 4c2)
    let mut args_checked = Vec::new();
    while let Some(mut arg_os) = args.next() {
        if arg_os.len() > 2 {
            let arg = arg_os.to_string_lossy();
            if arg.as_bytes()[0] == b'-' {
                // short options with num or multiple short options, multiple will be discarded
                let mut opt = '-';
                let mut num = String::new();
                let mut ok = true;
                // let c = arg.as_bytes()[1] as char;
                for c in arg.chars().skip(1) {
                    if c.is_ascii_digit() {
                        num.push(c);
                    } else if c == 'c' || c == 'u' {
                        if opt == '-' {
                            opt = c;
                        } else {
                            // multiple chars, reject
                            ok = false;
                            break;
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

// TODO split parser and logic
pub fn diff_compare(params: &Params) -> UResult<()> {
    // if from and to are the same file, no need to perform any comparison
    let maybe_report_identical_files = || {
        if params.report_identical_files {
            println!(
                "Files {} and {} are identical",
                params.from.to_string_lossy(),
                params.to.to_string_lossy(),
            );
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
        read_file_contents(&params.from).map_err_context(|| params.from.to_string_lossy().to_string());
    // read_file_contents(&params.from);
    // read_file_contents(&params.from).map_err(|e| {let mut io = UIoError::from(e); io.context =Some(params.from.to_string_lossy().to_string()); DiffError::Io(io)});
    let r_to_content =
        read_file_contents(&params.to).map_err_context(|| params.to.to_string_lossy().to_string());

    // Diff returns both errors
    let from_content = match r_from_content {
        Ok(c) => c,
        Err(e1) => match r_to_content {
            Ok(_) => return Err(DiffError::Io(e1).into()),
            Err(e2) => return Err(DiffError::IoDouble(e1, e2).into()),
        },
    };
    let to_content = match r_to_content {
        Ok(c) => c,
        Err(e2) => return Err(DiffError::Io(e2).into()),
    };

    // run diff
    let result: Vec<u8> = match params.format_out {
        Format::Normal => normal_diff::diff(&from_content, &to_content, params),
        Format::Unified => unified_diff::diff(&from_content, &to_content, params),
        Format::Context => context_diff::diff(&from_content, &to_content, params),
        Format::Ed => ed_diff::diff(&from_content, &to_content, params).unwrap_or_else(|error| {
            eprintln!("{error}");
            uucore::error::set_exit_code(2);
            std::process::exit(2);
        }),
        Format::SideBySide => {
            let mut output = stdout().lock();
            side_diff::diff(&from_content, &to_content, &mut output, params)
        }
    };
    if params.brief && !result.is_empty() {
        println!(
            "Files {} and {} differ",
            params.from.to_string_lossy(),
            params.to.to_string_lossy()
        );
    } else {
        let result = io::stdout().write_all(&result);
        match result {
            // This code is taken from coreutils.
            // <https://github.com/uutils/coreutils/blob/main/src/uu/seq/src/seq.rs>
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => {
                // GNU seq prints the Broken pipe message but still exits with status 0
                // unless SIGPIPE was explicitly ignored, in which case it should fail.
                let err = err.map_err_context(|| "write error".into());
                uucore::show_error!("{err}");
                #[cfg(unix)]
                if uucore::signals::sigpipe_was_ignored() {
                    uucore::error::set_exit_code(1);
                }
            }
            Err(error) => {
                eprintln!("{}", uucore::error::strip_errno(&error));
                uucore::error::set_exit_code(1);
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

/// Contains all cmp errors and their text messages.
///
/// All errors can be output easily using the normal Display functionality.
/// To format the error message for the typical diffutils output, use [format_error_text].
#[derive(Debug)]
pub enum DiffError {
    //     /// cmp does not handle directories
    //     ///
    //     /// Param: wrong operand (dir name)
    //     DirectoryNotAllowed(OsString),
    //
    //     /// File Read IO error
    //     ///
    //     /// Param: filepath, io error
    //     FileReadError(OsString, io::Error),
    //
    //     /// Generic IO error
    //     FileIo(OsString, io::Error),
    /// Generic IO error, here only Output errors
    Io(Box<dyn UError>),
    IoDouble(Box<dyn UError>, Box<dyn UError>),
}

impl std::error::Error for DiffError {}

impl uudiff::error::UError for DiffError {
    fn code(&self) -> i32 {
        2
    }

    // fn usage(&self) -> bool {
    //     // dbg!("CmpError: running usage");
    //     // match self {
    //     //     CmpError::DirectoryNotAllowed(os_string) => todo!(),
    //     //     CmpError::FileReadError(os_string, error) => todo!(),
    //     //     CmpError::FileIo(os_string, error) => todo!(),
    //     //     CmpError::GenericIo(error) => todo!(),
    //     // }
    //     false
    // }
}

impl From<std::io::Error> for DiffError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.into())
    }
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            // Self::DirectoryNotAllowed(dir) => write!(f, "'{}': is a directory", dir.to_string_lossy()),
            // Self::DirectoryNotAllowed(dir) => {
            //     translate!("cmp-error-is-directory", "name" => dir.to_string_lossy())
            // }
            // Self::FileReadError(path, error) => {
            //     utils::format_failure_to_read_input_file(path, error)
            // }
            // Self::FileIo(path, e) => format!("{}: {}", path.to_string_lossy(), strip_errno(e)),
            // very unlikely, not translated
            Self::Io(e) => {
                // dbg!("Io");
                return e.fmt(f);
            }
            Self::IoDouble(e1, e2) => {
                format!("{e1}\n{}: {e2}", uucore::util_name())
            }
        };

        write!(f, "{msg}")
    }
}

// Required for build.rs
pub fn uu_app() -> Command {
    crate::parser_diff::uu_app()
}
