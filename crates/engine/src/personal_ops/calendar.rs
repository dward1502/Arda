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
