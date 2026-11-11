//! The call to the status endpoint of a pool. A trait keeps the network out of the tests.

use std::time::Duration;

use crate::providers::claude_watch::HttpReply;

/// The longest time that one status call may take.
pub const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
/// The largest reply body that the client reads.
const MAX_BODY_BYTES: u64 = 1 << 20;

/// Makes the status call. An `Err` is a transport failure, for example a timeout.
///
/// The error text must not hold the key.
pub trait StatusClient {
    fn get_status(&self, url: &str, key: &str) -> Result<HttpReply, String>;
}

impl<T: StatusClient + ?Sized> StatusClient for Box<T> {
    fn get_status(&self, url: &str, key: &str) -> Result<HttpReply, String> {
        (**self).get_status(url, key)
    }
}

/// The real client. It uses ureq with rustls, follows no redirect and sends the key in the
/// `X-Api-Key` header.
#[derive(Debug, Clone)]
pub struct UreqStatusClient {
    agent: ureq::Agent,
}

impl UreqStatusClient {
    pub fn new() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(HTTP_TIMEOUT))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }
}

impl Default for UreqStatusClient {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusClient for UreqStatusClient {
    fn get_status(&self, url: &str, key: &str) -> Result<HttpReply, String> {
        let mut reply = self
            .agent
            .get(url)
            .header("X-Api-Key", key)
            .call()
            .map_err(|e| format!("The status call failed. {e}"))?;
        let status = reply.status().as_u16();
        let body = reply
            .body_mut()
            .with_config()
            .limit(MAX_BODY_BYTES)
            .read_to_string()
            .map_err(|e| format!("Cannot read the status reply. {e}"))?;
        Ok(HttpReply { status, body })
    }
}
