//! Error type for usaki-core. Messages follow strict STE form.

use std::path::PathBuf;

/// Result alias for usaki-core.
pub type Result<T> = std::result::Result<T, Error>;

/// Every failure that a provider can report.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file exists but a read failed.
    #[error("Cannot read the file {path}. {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The file is not valid JSON.
    #[error("The file {path} is not valid JSON. {reason}")]
    Malformed { path: PathBuf, reason: String },
    /// The JSON is valid but a field has the wrong type or value.
    #[error("The file {path} has a bad field. Field: {field}. {reason}")]
    InvalidField {
        path: PathBuf,
        field: String,
        reason: String,
    },
    /// The account has no cache location and no home directory is known.
    #[error("Cannot find the cache directory. Set XDG_CACHE_HOME or HOME.")]
    NoCacheDir,
    /// The account id holds a character that is not safe in a file name.
    #[error(
        "The account id {account} is not valid. Use only letters, digits, underscore and hyphen."
    )]
    InvalidAccountId { account: String },
    /// The account uses a provider that this provider cannot read.
    #[error("The account {account} does not use the provider {expected}.")]
    WrongProvider { account: String, expected: String },
}
