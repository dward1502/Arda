use arda_engine::personal_ops::calendar::{
    deduplicate_events, CaldavFetchRequest, CaldavFetchResponse, CaldavSecretResolver,
    CaldavSyncClient, CaldavSyncError, CaldavTransport, CalendarAdapterConfig, CalendarAdapterKind,
    CalendarEvent, CalendarEventSource, IcsExporter, IcsImporter, RetrySleeper,
};
use chrono::{TimeZone, Utc};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct FixtureSecrets {
    values: BTreeMap<String, String>,
    requested: RefCell<Vec<String>>,
}

impl FixtureSecrets {
    fn with(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }
}

impl CaldavSecretResolver for FixtureSecrets {
    fn resolve(&self, secret_ref: &str) -> Result<String, CaldavSyncError> {
        self.requested.borrow_mut().push(secret_ref.to_string());
        self.values
            .get(secret_ref)
            .cloned()
            .ok_or_else(|| CaldavSyncError::SecretUnavailable {
                secret_ref: secret_ref.to_string(),
            })
    }
}

struct FixtureTransport {
    responses: RefCell<Vec<Result<CaldavFetchResponse, CaldavSyncError>>>,
    requests: RefCell<Vec<CaldavFetchRequest>>,
}

impl FixtureTransport {
    fn new(responses: Vec<Result<CaldavFetchResponse, CaldavSyncError>>) -> Self {
        Self {
            responses: RefCell::new(responses.into_iter().rev().collect()),
            requests: RefCell::new(Vec::new()),
        }
    }
}

impl CaldavTransport for FixtureTransport {
    fn fetch_calendar(
        &self,
        request: CaldavFetchRequest,
        username: &str,
        password: &str,
    ) -> Result<CaldavFetchResponse, CaldavSyncError> {
        assert_eq!(username, "fixture-user");
        assert_eq!(password, "fixture-password");
        self.requests.borrow_mut().push(request);
        self.responses
            .borrow_mut()
            .pop()
            .expect("fixture response available")
    }
}

#[derive(Default)]
struct FixtureSleeper {
    delays_ms: RefCell<Vec<u64>>,
}

impl RetrySleeper for FixtureSleeper {
    fn sleep_ms(&self, delay_ms: u64) {
        self.delays_ms.borrow_mut().push(delay_ms);
    }
}

fn caldav_config() -> CalendarAdapterConfig {
    CalendarAdapterConfig {
        adapter: CalendarAdapterKind::Caldav,
        timezone: "UTC".to_string(),
        caldav_url: Some("https://caldav.example.test/dav".to_string()),
        caldav_username_ref: Some("secret://calendar/user".to_string()),
        caldav_password_ref: Some("secret://calendar/pass".to_string()),
        calendar_id: "personal".to_string(),
        request_timeout_ms: 5_000,
        max_attempts: 3,
        initial_retry_delay_ms: 10,
        max_retry_delay_ms: 40,
    }
}

fn caldav_secrets() -> FixtureSecrets {
    FixtureSecrets::default()
        .with("secret://calendar/user", "fixture-user")
        .with("secret://calendar/pass", "fixture-password")
}

fn caldav_ics() -> String {
    IcsExporter::export(&[
        CalendarEvent {
            uid: "external-1@example.test".to_string(),
            summary: "External appointment".to_string(),
            description: None,
            start: Utc.with_ymd_and_hms(2026, 11, 1, 5, 30, 0).unwrap(),
            end: Some(Utc.with_ymd_and_hms(2026, 11, 1, 6, 30, 0).unwrap()),
            timezone: "UTC".to_string(),
            source: CalendarEventSource::Import,
            recurrence_rule: None,
        },
        CalendarEvent {
            uid: "external-2@example.test".to_string(),
            summary: "External follow-up".to_string(),
            description: Some("Provider changed the note".to_string()),
            start: Utc.with_ymd_and_hms(2026, 3, 8, 7, 30, 0).unwrap(),
            end: None,
            timezone: "UTC".to_string(),
            source: CalendarEventSource::Import,
            recurrence_rule: None,
        },
    ])
}

fn sample_event() -> CalendarEvent {
    CalendarEvent {
        uid: "test-uid-001@example.com".to_string(),
        summary: "Transplant follow-up".to_string(),
        description: Some("Cardiology appointment".to_string()),
        start: Utc.with_ymd_and_hms(2026, 8, 15, 14, 0, 0).unwrap(),
        end: Some(Utc.with_ymd_and_hms(2026, 8, 15, 15, 0, 0).unwrap()),
        timezone: "UTC".to_string(),
        source: CalendarEventSource::Arda,
        recurrence_rule: Some("FREQ=WEEKLY;COUNT=10".to_string()),
    }
}

#[test]
fn ics_export_and_reimport_roundtrips() {
    let event = sample_event();
    let ics = IcsExporter::export(std::slice::from_ref(&event));

    assert!(ics.contains("BEGIN:VCALENDAR"));
    assert!(ics.contains("BEGIN:VEVENT"));
    assert!(ics.contains("UID:test-uid-001@example.com"));
    assert!(ics.contains("SUMMARY:Transplant follow-up"));
    assert!(ics.contains("DTSTART:20260815T140000Z"));
    assert!(ics.contains("DTEND:20260815T150000Z"));
    assert!(ics.contains("RRULE:FREQ=WEEKLY;COUNT=10"));
    assert!(ics.contains("END:VCALENDAR"));

    let imported = IcsImporter::parse(&ics);
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].uid, event.uid);
    assert_eq!(imported[0].summary, event.summary);
    assert_eq!(imported[0].start, event.start);
    assert_eq!(imported[0].end, event.end);
    assert_eq!(imported[0].recurrence_rule, event.recurrence_rule);
    assert_eq!(imported[0].source, CalendarEventSource::Import);
}

#[test]
fn ics_export_empty_event_list() {
    let ics = IcsExporter::export(&[]);
    assert!(ics.contains("BEGIN:VCALENDAR"));
    assert!(ics.contains("END:VCALENDAR"));
    // No VEVENT entries for empty list
    assert!(!ics.contains("BEGIN:VEVENT"));
}

#[test]
fn ics_importer_parses_multiple_events() {
    let ics = "\
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//test//EN
BEGIN:VEVENT
UID:evt-1@local
SUMMARY:Task One
DTSTART:20260730T090000Z
DTEND:20260730T100000Z
END:VEVENT
BEGIN:VEVENT
UID:evt-2@local
SUMMARY:Task Two
DTSTART:20260731T120000Z
END:20260731T130000Z
RRULE:FREQ=DAILY;COUNT=5
END:VEVENT
END:VCALENDAR
";
    let events = IcsImporter::parse(ics);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].uid, "evt-1@local");
    assert_eq!(events[0].summary, "Task One");
    assert_eq!(events[1].uid, "evt-2@local");
    assert_eq!(events[1].summary, "Task Two");
    assert_eq!(
        events[1].recurrence_rule.as_deref(),
        Some("FREQ=DAILY;COUNT=5")
    );
}

#[test]
fn ics_importer_ignores_events_without_uid() {
    let ics = "\
BEGIN:VCALENDAR
BEGIN:VEVENT
SUMMARY:No UID Event
DTSTART:20260730T090000Z
END:VEVENT
END:VCALENDAR
";
    let events = IcsImporter::parse(ics);
    assert_eq!(events.len(), 0);
}

#[test]
fn deduplicate_prefers_arda_over_import() {
    let arda_event = CalendarEvent {
        uid: "shared-uid".to_string(),
        summary: "Arda-authored".to_string(),
        description: None,
        start: Utc.with_ymd_and_hms(2026, 8, 1, 9, 0, 0).unwrap(),
        end: None,
        timezone: "UTC".to_string(),
        source: CalendarEventSource::Arda,
        recurrence_rule: None,
    };
    let imported_event = CalendarEvent {
        uid: "shared-uid".to_string(),
        summary: "Imported".to_string(),
        description: None,
        start: Utc.with_ymd_and_hms(2026, 8, 2, 9, 0, 0).unwrap(),
        end: None,
        timezone: "UTC".to_string(),
        source: CalendarEventSource::Import,
        recurrence_rule: None,
    };

    let deduped = deduplicate_events(vec![imported_event.clone(), arda_event.clone()]);
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0].source, CalendarEventSource::Arda);
    assert_eq!(deduped[0].summary, "Arda-authored");
    assert_eq!(deduped[0].start, arda_event.start);
}

#[test]
fn deduplicate_keeps_unique_uids() {
    let events = vec![
        sample_event(),
        CalendarEvent {
            uid: "different-uid".to_string(),
            summary: "Other event".to_string(),
            description: None,
            start: Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap(),
            end: None,
            timezone: "UTC".to_string(),
            source: CalendarEventSource::Import,
            recurrence_rule: None,
        },
    ];
    let deduped = deduplicate_events(events);
    assert_eq!(deduped.len(), 2);
}

#[test]
fn calendar_adapter_config_defaults_to_ics() {
    let config = CalendarAdapterConfig::default();
    assert_eq!(config.adapter, CalendarAdapterKind::Ics);
    assert_eq!(config.timezone, "UTC");
    assert_eq!(config.calendar_id, "personal");
    assert!(config.caldav_url.is_none());
    assert!(config.caldav_username_ref.is_none());
    assert!(config.caldav_password_ref.is_none());
}

#[test]
fn calendar_adapter_config_deserializes_caldav() {
    let toml_str = r#"
adapter = "caldav"
timezone = "America/New_York"
caldav_url = "https://caldav.example.com"
caldav_username_ref = "secret://caldav/username"
caldav_password_ref = "secret://caldav/password"
calendar_id = "my-calendar"
"#;
    let config: CalendarAdapterConfig = toml::from_str(toml_str).expect("config deserializes");
    assert_eq!(config.adapter, CalendarAdapterKind::Caldav);
    assert_eq!(config.timezone, "America/New_York");
    assert_eq!(
        config.caldav_url.as_deref(),
        Some("https://caldav.example.com")
    );
    assert_eq!(config.calendar_id, "my-calendar");
}

#[test]
fn caldav_sync_resolves_secret_refs_and_preserves_arda_authority() {
    let transport = FixtureTransport::new(vec![Ok(CaldavFetchResponse {
        status: 207,
        body: caldav_ics(),
    })]);
    let secrets = caldav_secrets();
    let sleeper = FixtureSleeper::default();
    let existing = CalendarEvent {
        uid: "external-1@example.test".to_string(),
        summary: "Arda-owned appointment".to_string(),
        description: None,
        start: Utc.with_ymd_and_hms(2026, 11, 1, 5, 30, 0).unwrap(),
        end: None,
        timezone: "UTC".to_string(),
        source: CalendarEventSource::Arda,
        recurrence_rule: None,
    };

    let outcome = CaldavSyncClient::new(&transport, &secrets, &sleeper)
        .sync(&caldav_config(), vec![existing])
        .expect("CalDAV sync succeeds");

    assert_eq!(outcome.attempts, 1);
    assert_eq!(outcome.events.len(), 2);
    assert_eq!(
        outcome
            .events
            .iter()
            .find(|event| event.uid == "external-1@example.test")
            .unwrap()
            .source,
        CalendarEventSource::Arda
    );
    assert_eq!(
        outcome.imported_uids,
        BTreeSet::from([
            "external-1@example.test".to_string(),
            "external-2@example.test".to_string(),
        ])
    );
    assert_eq!(
        secrets.requested.borrow().as_slice(),
        ["secret://calendar/user", "secret://calendar/pass"]
    );
    let request = transport.requests.borrow();
    assert_eq!(request.len(), 1);
    assert_eq!(request[0].timeout_ms, 5_000);
    assert_eq!(request[0].calendar_id, "personal");
}

#[test]
fn caldav_sync_retries_only_transient_failures_with_bounded_backoff() {
    let transport = FixtureTransport::new(vec![
        Err(CaldavSyncError::Transport {
            message: "temporary connection failure".to_string(),
            retryable: true,
        }),
        Ok(CaldavFetchResponse {
            status: 503,
            body: String::new(),
        }),
        Ok(CaldavFetchResponse {
            status: 207,
            body: caldav_ics(),
        }),
    ]);
    let secrets = caldav_secrets();
    let sleeper = FixtureSleeper::default();

    let outcome = CaldavSyncClient::new(&transport, &secrets, &sleeper)
        .sync(&caldav_config(), Vec::new())
        .expect("third bounded attempt succeeds");

    assert_eq!(outcome.attempts, 3);
    assert_eq!(sleeper.delays_ms.borrow().as_slice(), [10, 20]);
    assert_eq!(transport.requests.borrow().len(), 3);
}

#[test]
fn caldav_sync_does_not_retry_authentication_failures() {
    let transport = FixtureTransport::new(vec![Ok(CaldavFetchResponse {
        status: 401,
        body: "credential rejected".to_string(),
    })]);
    let secrets = caldav_secrets();
    let sleeper = FixtureSleeper::default();

    let error = CaldavSyncClient::new(&transport, &secrets, &sleeper)
        .sync(&caldav_config(), Vec::new())
        .unwrap_err();

    assert_eq!(
        error,
        CaldavSyncError::HttpStatus {
            status: 401,
            retryable: false,
        }
    );
    assert!(sleeper.delays_ms.borrow().is_empty());
    assert_eq!(transport.requests.borrow().len(), 1);
}

#[test]
fn caldav_sync_rejects_inline_credentials_before_transport() {
    let mut config = caldav_config();
    config.caldav_password_ref = Some("plain-text-password".to_string());
    let transport = FixtureTransport::new(Vec::new());
    let secrets = caldav_secrets();
    let sleeper = FixtureSleeper::default();

    let error = CaldavSyncClient::new(&transport, &secrets, &sleeper)
        .sync(&config, Vec::new())
        .unwrap_err();

    assert!(matches!(error, CaldavSyncError::InvalidConfiguration(_)));
    assert!(transport.requests.borrow().is_empty());
    assert!(secrets.requested.borrow().is_empty());
}

#[test]
fn caldav_sync_parses_multistatus_calendar_data() {
    let encoded = caldav_ics().replace('&', "&amp;").replace('<', "&lt;");
    let body = format!(
        "<d:multistatus xmlns:d=\"DAV:\" xmlns:c=\"urn:ietf:params:xml:ns:caldav\"><d:response><c:calendar-data>{encoded}</c:calendar-data></d:response></d:multistatus>"
    );
    let transport = FixtureTransport::new(vec![Ok(CaldavFetchResponse { status: 207, body })]);
    let secrets = caldav_secrets();
    let sleeper = FixtureSleeper::default();

    let outcome = CaldavSyncClient::new(&transport, &secrets, &sleeper)
        .sync(&caldav_config(), Vec::new())
        .expect("CalDAV multistatus parses");

    assert_eq!(outcome.events.len(), 2);
    assert!(outcome
        .events
        .iter()
        .all(|event| event.source == CalendarEventSource::Import));
}
