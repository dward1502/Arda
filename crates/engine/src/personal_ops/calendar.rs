//! Calendar adapter for personal-ops reminders.
//!
//! Supports .ics import/export first. CalDAV is a supervised adapter
//! added after local fixtures pass. Secret references only in config —
//! never credentials.
//!
//! Time-zone and daylight-saving transitions are handled via
//! `chrono-tz` when available; the core logic is pure and testable.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A calendar event imported from or exported to an external calendar.
/// External updates remain distinguishable from Arda-authored reminders
/// via the `source` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    /// RFC 5545 TZID or "UTC"
    pub timezone: String,
    /// "import" for external sources, "arda" for Arda-authored reminders
    pub source: CalendarEventSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarEventSource {
    Import,
    Arda,
}

/// Calendar adapter configuration.
/// Store secret *references* (e.g. Vault paths), never credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarAdapterConfig {
    pub adapter: CalendarAdapterKind,
    pub timezone: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caldav_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caldav_username_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caldav_password_ref: Option<String>,
    /// Calendar ID / path to sync (e.g. "personal" or a URL path component).
    #[serde(default)]
    pub calendar_id: String,
    #[serde(default = "default_request_timeout_ms")]
    pub request_timeout_ms: u64,
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u8,
    #[serde(default = "default_initial_retry_delay_ms")]
    pub initial_retry_delay_ms: u64,
    #[serde(default = "default_max_retry_delay_ms")]
    pub max_retry_delay_ms: u64,
}

const fn default_request_timeout_ms() -> u64 {
    15_000
}

const fn default_max_attempts() -> u8 {
    3
}

const fn default_initial_retry_delay_ms() -> u64 {
    250
}

const fn default_max_retry_delay_ms() -> u64 {
    2_000
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarAdapterKind {
    Ics,
    Caldav,
}

impl Default for CalendarAdapterConfig {
    fn default() -> Self {
        Self {
            adapter: CalendarAdapterKind::Ics,
            timezone: "UTC".to_string(),
            caldav_url: None,
            caldav_username_ref: None,
            caldav_password_ref: None,
            calendar_id: "personal".to_string(),
            request_timeout_ms: default_request_timeout_ms(),
            max_attempts: default_max_attempts(),
            initial_retry_delay_ms: default_initial_retry_delay_ms(),
            max_retry_delay_ms: default_max_retry_delay_ms(),
        }
    }
}

/// Minimal iCalendar (RFC 5126-ish) writer — produces compliant-enough
/// .ics output for import by most calendar clients.
pub struct IcsExporter;

impl IcsExporter {
    pub fn export(events: &[CalendarEvent]) -> String {
        let mut buf = String::new();
        buf.push_str("BEGIN:VCALENDAR\r\n");
        buf.push_str("VERSION:2.0\r\n");
        buf.push_str("PRODID:-//arda-personal-ops//EN\r\n");
        buf.push_str("CALSCALE:GREGORIAN\r\n");
        for event in events {
            buf.push_str("BEGIN:VEVENT\r\n");
            buf.push_str(&format!("UID:{}\r\n", event.uid));
            buf.push_str(&format!("SUMMARY:{}\r\n", ics_escape(&event.summary)));
            if let Some(desc) = &event.description {
                buf.push_str(&format!("DESCRIPTION:{}\r\n", ics_escape(desc)));
            }
            buf.push_str(&format!(
                "DTSTART:{}T{}Z\r\n",
                event.start.format("%Y%m%d"),
                event.start.format("%H%M%S"),
            ));
            if let Some(end) = &event.end {
                buf.push_str(&format!(
                    "DTEND:{}T{}Z\r\n",
                    end.format("%Y%m%d"),
                    end.format("%H%M%S"),
                ));
            }
            if let Some(rrule) = &event.recurrence_rule {
                buf.push_str(&format!("RRULE:{}\r\n", rrule));
            }
            buf.push_str("END:VEVENT\r\n");
        }
        buf.push_str("END:VCALENDAR\r\n");
        buf
    }
}

fn ics_escape(s: &str) -> String {
    s.replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

/// Minimal iCalendar (RFC 5545 subset) parser.
pub struct IcsImporter;

impl IcsImporter {
    /// Parse an .ics document into calendar events.
    /// Only parses the properties needed for personal-ops reminders.
    pub fn parse(ics: &str) -> Vec<CalendarEvent> {
        let mut events = Vec::new();
        let mut current: Option<CalendarEvent> = None;

        for raw_line in ics.lines() {
            let line = raw_line.trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            if line == "BEGIN:VCALENDAR" || line == "END:VCALENDAR" {
                continue;
            }
            if line == "BEGIN:VEVENT" {
                current = Some(CalendarEvent {
                    uid: String::new(),
                    summary: String::new(),
                    description: None,
                    start: Utc::now(),
                    end: None,
                    timezone: "UTC".to_string(),
                    source: CalendarEventSource::Import,
                    recurrence_rule: None,
                });
                continue;
            }
            if line == "END:VEVENT" {
                if let Some(ev) = current.take() {
                    if !ev.uid.is_empty() {
                        events.push(ev);
                    }
                }
                continue;
            }
            if let Some(ev) = current.as_mut() {
                if let Some(rest) = line.strip_prefix("UID:") {
                    ev.uid = rest.to_string();
                } else if let Some(rest) = line.strip_prefix("SUMMARY:") {
                    ev.summary = ics_unescape(rest);
                } else if let Some(rest) = line.strip_prefix("DESCRIPTION:") {
                    ev.description = Some(ics_unescape(rest));
                } else if let Some(rest) = line.strip_prefix("DTSTART:") {
                    if let Ok(dt) = parse_ics_datetime(rest) {
                        ev.start = dt;
                    }
                } else if let Some(rest) = line.strip_prefix("DTEND:") {
                    if let Ok(dt) = parse_ics_datetime(rest) {
                        ev.end = Some(dt);
                    }
                } else if let Some(rest) = line.strip_prefix("RRULE:") {
                    ev.recurrence_rule = Some(rest.to_string());
                }
            }
        }
        events
    }
}

fn ics_unescape(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
}

/// Parse a DTSTART/DTEND value into a UTC DateTime.
/// Handles both floating local time (e.g. "20260815T140000")
/// and UTC (e.g. "20260815T140000Z"). Does NOT handle TZID-prefixed
/// datetimes with zone names — those fall back to naive + UTC assumption.
fn parse_ics_datetime(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
    let trimmed = s.trim();
    if let Some(stripped) = trimmed.strip_suffix('Z') {
        let naive = NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S")?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
    } else {
        let naive = NaiveDateTime::parse_from_str(trimmed, "%Y%m%dT%H%M%S")?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
    }
}

use chrono::NaiveDateTime;

/// Deduplicate events by UID, preferring Arda-authored entries
/// over imported ones so external sync never overwrites local truth.
pub fn deduplicate_events(events: Vec<CalendarEvent>) -> Vec<CalendarEvent> {
    let mut seen: std::collections::BTreeMap<String, CalendarEvent> =
        std::collections::BTreeMap::new();
    for event in events {
        seen.entry(event.uid.clone())
            .and_modify(|existing| {
                // Prefer Arda-authored (source == Arda) over Import
                let existing_is_arda = existing.source == CalendarEventSource::Arda;
                if !existing_is_arda && event.source == CalendarEventSource::Arda {
                    *existing = event.clone();
                }
            })
            .or_insert(event);
    }
    seen.into_values().collect()
}

/// A bounded CalDAV read request. Authentication is passed separately so it is
/// never retained in a serializable request or receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaldavFetchRequest {
    pub url: String,
    pub calendar_id: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaldavFetchResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaldavRetryPolicy {
    pub max_attempts: u8,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl CaldavRetryPolicy {
    fn from_config(config: &CalendarAdapterConfig) -> Self {
        Self {
            max_attempts: config.max_attempts,
            initial_delay_ms: config.initial_retry_delay_ms,
            max_delay_ms: config.max_retry_delay_ms,
        }
    }

    fn delay_after(&self, attempt: u8) -> u64 {
        let shift = u32::from(attempt.saturating_sub(1)).min(31);
        self.initial_delay_ms
            .saturating_mul(1_u64 << shift)
            .min(self.max_delay_ms)
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CaldavSyncError {
    #[error("invalid CalDAV configuration: {0}")]
    InvalidConfiguration(String),
    #[error("calendar secret reference is unavailable: {secret_ref}")]
    SecretUnavailable { secret_ref: String },
    #[error("CalDAV transport failed: {message}")]
    Transport { message: String, retryable: bool },
    #[error("CalDAV endpoint returned HTTP {status}")]
    HttpStatus { status: u16, retryable: bool },
    #[error("CalDAV response contained no iCalendar data")]
    InvalidResponse,
}

impl CaldavSyncError {
    fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Transport {
                retryable: true,
                ..
            } | Self::HttpStatus {
                retryable: true,
                ..
            }
        )
    }
}

pub trait CaldavSecretResolver {
    fn resolve(&self, secret_ref: &str) -> Result<String, CaldavSyncError>;
}

pub trait CaldavTransport {
    fn fetch_calendar(
        &self,
        request: CaldavFetchRequest,
        username: &str,
        password: &str,
    ) -> Result<CaldavFetchResponse, CaldavSyncError>;
}

pub trait RetrySleeper {
    fn sleep_ms(&self, delay_ms: u64);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ThreadSleeper;

impl RetrySleeper for ThreadSleeper {
    fn sleep_ms(&self, delay_ms: u64) {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }
}

/// Concrete CalDAV REPORT transport. It is blocking by design and must run on
/// the adapter supervisor's worker thread, never directly on an async reactor.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReqwestCaldavTransport;

impl CaldavTransport for ReqwestCaldavTransport {
    fn fetch_calendar(
        &self,
        request: CaldavFetchRequest,
        username: &str,
        password: &str,
    ) -> Result<CaldavFetchResponse, CaldavSyncError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(request.timeout_ms))
            .build()
            .map_err(|error| CaldavSyncError::Transport {
                message: error.to_string(),
                retryable: false,
            })?;
        let method = reqwest::Method::from_bytes(b"REPORT").expect("REPORT is a valid method");
        let response = client
            .request(method, &request.url)
            .basic_auth(username, Some(password))
            .header("Depth", "1")
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/xml; charset=utf-8",
            )
            .header(reqwest::header::ACCEPT, "application/xml, text/calendar")
            .body(caldav_calendar_query(&request.calendar_id))
            .send()
            .map_err(|error| CaldavSyncError::Transport {
                retryable: error.is_timeout() || error.is_connect(),
                message: error.to_string(),
            })?;
        let status = response.status().as_u16();
        let body = response
            .text()
            .map_err(|error| CaldavSyncError::Transport {
                message: error.to_string(),
                retryable: false,
            })?;
        Ok(CaldavFetchResponse { status, body })
    }
}

fn caldav_calendar_query(calendar_id: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\" ?>\
         <c:calendar-query xmlns:d=\"DAV:\" xmlns:c=\"urn:ietf:params:xml:ns:caldav\">\
         <d:prop><d:getetag/><c:calendar-data/></d:prop>\
         <c:filter><c:comp-filter name=\"VCALENDAR\"/></c:filter>\
         <!-- calendar-id:{} --></c:calendar-query>",
        xml_escape(calendar_id)
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncOutcome {
    pub events: Vec<CalendarEvent>,
    pub imported_uids: std::collections::BTreeSet<String>,
    pub attempts: u8,
}

pub struct CaldavSyncClient<'a, T, R, S> {
    transport: &'a T,
    secrets: &'a R,
    sleeper: &'a S,
}

impl<'a, T, R, S> CaldavSyncClient<'a, T, R, S>
where
    T: CaldavTransport,
    R: CaldavSecretResolver,
    S: RetrySleeper,
{
    pub fn new(transport: &'a T, secrets: &'a R, sleeper: &'a S) -> Self {
        Self {
            transport,
            secrets,
            sleeper,
        }
    }

    pub fn sync(
        &self,
        config: &CalendarAdapterConfig,
        existing: Vec<CalendarEvent>,
    ) -> Result<SyncOutcome, CaldavSyncError> {
        validate_caldav_config(config)?;
        let username_ref = config.caldav_username_ref.as_deref().expect("validated");
        let password_ref = config.caldav_password_ref.as_deref().expect("validated");
        let username = self.secrets.resolve(username_ref)?;
        let password = self.secrets.resolve(password_ref)?;
        if username.is_empty() || password.is_empty() {
            return Err(CaldavSyncError::InvalidConfiguration(
                "resolved credentials cannot be empty".to_string(),
            ));
        }

        let request = CaldavFetchRequest {
            url: config.caldav_url.clone().expect("validated"),
            calendar_id: config.calendar_id.clone(),
            timeout_ms: config.request_timeout_ms,
        };
        let policy = CaldavRetryPolicy::from_config(config);

        for attempt in 1..=policy.max_attempts {
            match self
                .transport
                .fetch_calendar(request.clone(), &username, &password)
                .and_then(response_to_events)
            {
                Ok(imported) => {
                    let imported_uids = imported.iter().map(|event| event.uid.clone()).collect();
                    let events = deduplicate_events(existing.into_iter().chain(imported).collect());
                    return Ok(SyncOutcome {
                        events,
                        imported_uids,
                        attempts: attempt,
                    });
                }
                Err(error) if error.retryable() && attempt < policy.max_attempts => {
                    self.sleeper.sleep_ms(policy.delay_after(attempt));
                }
                Err(error) => return Err(error),
            }
        }
        unreachable!("validated retry policy always performs at least one attempt")
    }
}

fn validate_caldav_config(config: &CalendarAdapterConfig) -> Result<(), CaldavSyncError> {
    if config.adapter != CalendarAdapterKind::Caldav {
        return Err(CaldavSyncError::InvalidConfiguration(
            "adapter must be caldav".to_string(),
        ));
    }
    let url = config.caldav_url.as_deref().unwrap_or_default();
    if !url.starts_with("https://") {
        return Err(CaldavSyncError::InvalidConfiguration(
            "caldav_url must use https".to_string(),
        ));
    }
    if config.calendar_id.trim().is_empty() || config.request_timeout_ms == 0 {
        return Err(CaldavSyncError::InvalidConfiguration(
            "calendar_id and request_timeout_ms must be non-empty".to_string(),
        ));
    }
    if config.max_attempts == 0
        || config.initial_retry_delay_ms == 0
        || config.max_retry_delay_ms < config.initial_retry_delay_ms
    {
        return Err(CaldavSyncError::InvalidConfiguration(
            "retry bounds are invalid".to_string(),
        ));
    }
    for (name, value) in [
        ("caldav_username_ref", &config.caldav_username_ref),
        ("caldav_password_ref", &config.caldav_password_ref),
    ] {
        let value = value.as_deref().unwrap_or_default();
        if !value.starts_with("secret://") || value.len() == "secret://".len() {
            return Err(CaldavSyncError::InvalidConfiguration(format!(
                "{name} must be a secret:// reference"
            )));
        }
    }
    Ok(())
}

fn response_to_events(
    response: CaldavFetchResponse,
) -> Result<Vec<CalendarEvent>, CaldavSyncError> {
    if !(200..300).contains(&response.status) {
        return Err(CaldavSyncError::HttpStatus {
            status: response.status,
            retryable: response.status == 429 || response.status >= 500,
        });
    }
    let calendar_data = extract_calendar_data(&response.body);
    let mut events = IcsImporter::parse(&calendar_data);
    if events.is_empty() && !calendar_data.contains("BEGIN:VCALENDAR") {
        return Err(CaldavSyncError::InvalidResponse);
    }
    for event in &mut events {
        event.source = CalendarEventSource::Import;
    }
    Ok(events)
}

fn extract_calendar_data(body: &str) -> String {
    if body.contains("BEGIN:VCALENDAR") && !body.contains("calendar-data") {
        return body.to_string();
    }
    let mut output = String::new();
    let mut remainder = body;
    while let Some(open) = remainder.find("<c:calendar-data") {
        remainder = &remainder[open..];
        let Some(start) = remainder.find('>') else {
            break;
        };
        remainder = &remainder[start + 1..];
        let Some(end) = remainder.find("</c:calendar-data>") else {
            break;
        };
        output.push_str(&xml_unescape(&remainder[..end]));
        output.push('\n');
        remainder = &remainder[end + "</c:calendar-data>".len()..];
    }
    output
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}
