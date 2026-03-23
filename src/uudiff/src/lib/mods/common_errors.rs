// This file is part of the uutils diffutils package.
//
// For the full copyright and license information, please view the LICENSE-*
// files that was distributed with this source code.

//! Common errors for all diffutils utilities.

use std::ffi::OsString;

use crate::{error::UError, translate};

/// Contains common Core/DiffUtils errors and their text messages.
///
/// Returns exit code 2, if a different exit code is required,
/// use [UtilsErrorCode]
///
/// A typical way to return an std::io:Error as
/// Box<dyn UError> (from [crate::error::UResult]) is:
/// Err => {
///     let io = error.map_err_context(|| path.to_string_lossy().to_string());
///     return Err(UtilsError::Io(io).into());
/// }
// Clone and PartialEq cannot be derived for Box<dyn Error>.
#[derive(Debug)]
pub enum UtilsError {
    /// When a util does not handle directories (e.g. cmp).
    ///
    /// Param: wrong operand (dir name)
    DirectoryNotAllowed(OsString),

    /// Generic IO error, Display handled by [crate::error::UIoError]
    Io(Box<dyn UError>),
    IoDouble(Box<dyn UError>, Box<dyn UError>),
}

impl std::error::Error for UtilsError {}

impl UError for UtilsError {
    fn code(&self) -> i32 {
        2
    }
}

impl From<std::io::Error> for UtilsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.into())
    }
}

impl std::fmt::Display for UtilsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::DirectoryNotAllowed(dir) => {
                translate!("error-is-a-directory", "file" => dir.to_string_lossy())
            }
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

/// Like [UtilsError] with the option to specify the exit code.
///
/// A typical way to return an std::io:Error as
/// Box<dyn UError> (from [crate::error::UResult]) is:
/// Err => {
///     let io = error.map_err_context(|| path.to_string_lossy().to_string());
///     return Err(UtilsErrorCode::new(UtilsError::Io(io), 4).into());
/// }
#[derive(Debug)]
pub struct UtilsErrorCode {
    pub utils_error: UtilsError,
    pub code: i32,
}

impl UtilsErrorCode {
    pub fn new(utils_error: UtilsError, code: i32) -> Self {
        Self { utils_error, code }
    }
}

impl std::error::Error for UtilsErrorCode {}

impl UError for UtilsErrorCode {
    fn code(&self) -> i32 {
        self.code
    }
}

impl std::fmt::Display for UtilsErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.utils_error.fmt(f)
    }
}
