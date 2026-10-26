//! Reads the token and its expiry from a watcher credentials file.
//!
//! This module only reads. The official `claude` binary owns the file. Error messages never
//! hold the token or any other value from the file.

use std::fmt;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::error::{Error, Result};

/// An OAuth access token. Its `Debug` output hides the value, so a log line cannot leak it.
#[derive(Clone, PartialEq, Eq)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the raw value. Use it only to build the Authorization header.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessToken(<hidden>)")
    }
}

/// The two fields of a credentials file that usaKi uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub token: AccessToken,
    pub expires_at: DateTime<Utc>,
}

/// Reads a credentials file. A missing file gives `None`.
pub fn read_credentials(path: &Path) -> Result<Option<Credentials>> {
    // read_to_string opens the file for reading only.
    match std::fs::read_to_string(path) {
        Ok(text) => parse_credentials(&text, path).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(Error::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Parses the text of a credentials file.
///
/// It reads `claudeAiOauth.accessToken` and `claudeAiOauth.expiresAt` (epoch ms).
/// `origin` is the file path, used only in error messages.
pub fn parse_credentials(text: &str, origin: &Path) -> Result<Credentials> {
    // The parse goes through Value, because a typed parse can quote a field value in its error.
    let root: Value = serde_json::from_str(text).map_err(|e| Error::Malformed {
        path: origin.to_path_buf(),
        reason: e.to_string(),
    })?;
    let bad = |field: &str, reason: &str| Error::InvalidField {
        path: origin.to_path_buf(),
        field: field.to_string(),
        reason: reason.to_string(),
    };
    let oauth = root
        .get("claudeAiOauth")
        .and_then(Value::as_object)
        .ok_or_else(|| bad("claudeAiOauth", "The value must be an object."))?;
    let token = oauth
        .get("accessToken")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| {
            bad(
                "claudeAiOauth.accessToken",
                "The value must be a string that is not empty.",
            )
        })?;
    let expires_at = oauth
        .get("expiresAt")
        .and_then(Value::as_i64)
        .and_then(DateTime::from_timestamp_millis)
        .ok_or_else(|| {
            bad(
                "claudeAiOauth.expiresAt",
                "The value must be epoch milliseconds.",
            )
        })?;
    Ok(Credentials {
        token: AccessToken::new(token),
        expires_at,
    })
}
