//! The HTTP call to the usage API. A trait keeps the network out of the tests.

use std::time::Duration;

use super::credentials::AccessToken;
use super::{OAUTH_BETA, USAGE_URL};

/// The longest time that one usage call may take.
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(20);
/// The largest reply body that the client reads.
const MAX_BODY_BYTES: u64 = 1 << 20;

/// The status and body of one HTTP reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpReply {
    pub status: u16,
    pub body: String,
}

/// Makes the usage call. An `Err` is a transport failure, for example a timeout.
pub trait UsageClient {
    fn fetch_usage(&self, token: &AccessToken, user_agent: &str) -> Result<HttpReply, String>;
}

/// The real client. It uses ureq with rustls, so it needs no OpenSSL.
#[derive(Debug, Clone)]
pub struct UreqClient {
    agent: ureq::Agent,
}

impl UreqClient {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(HTTP_TIMEOUT))
            // The watcher reads 401 and 429 as data, so a 4xx status must not be an error.
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

impl Default for UreqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageClient for UreqClient {
    fn fetch_usage(&self, token: &AccessToken, user_agent: &str) -> Result<HttpReply, String> {
        let mut reply = self
            .agent
            .get(USAGE_URL)
            .header("Authorization", format!("Bearer {}", token.expose()))
            .header("anthropic-beta", OAUTH_BETA)
            .header("User-Agent", user_agent)
            .call()
            .map_err(|e| format!("The usage call failed. {e}"))?;
        let status = reply.status().as_u16();
        let body = reply
            .body_mut()
            .with_config()
            .limit(MAX_BODY_BYTES)
            .read_to_string()
            .map_err(|e| format!("Cannot read the usage reply. {e}"))?;
        Ok(HttpReply { status, body })
    }
}
