use arda_core::error::Result;
use std::sync::OnceLock;
use std::time::Duration;

use super::athena_error;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(20);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const USER_AGENT: &str = "arda-athena/0.1";

static ASYNC_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
static BLOCKING_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

pub(super) fn async_client() -> Result<&'static reqwest::Client> {
    if let Some(client) = ASYNC_CLIENT.get() {
        return Ok(client);
    }
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .build()
        .map_err(|error| athena_error(format!("failed to build shared HTTP client: {error}")))?;
    let _ = ASYNC_CLIENT.set(client);
    ASYNC_CLIENT
        .get()
        .ok_or_else(|| athena_error("shared HTTP client initialization failed"))
}

pub(super) fn blocking_client() -> Result<&'static reqwest::blocking::Client> {
    if let Some(client) = BLOCKING_CLIENT.get() {
        return Ok(client);
    }
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        // The blocking builder has no independent read timeout; its total
        // request timeout bounds response reads.
        .timeout(READ_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .build()
        .map_err(|error| {
            athena_error(format!(
                "failed to build shared blocking HTTP client: {error}"
            ))
        })?;
    let _ = BLOCKING_CLIENT.set(client);
    BLOCKING_CLIENT
        .get()
        .ok_or_else(|| athena_error("shared blocking HTTP client initialization failed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clients_are_reused_across_callers() {
        let first_async = async_client().expect("async client");
        let second_async = async_client().expect("async client");
        let first_blocking = blocking_client().expect("blocking client");
        let second_blocking = blocking_client().expect("blocking client");

        assert!(std::ptr::eq(first_async, second_async));
        assert!(std::ptr::eq(first_blocking, second_blocking));
    }
}
