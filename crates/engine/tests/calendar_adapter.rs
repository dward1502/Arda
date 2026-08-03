use arda_engine::personal_ops::calendar::{
    deduplicate_events, CalendarAdapterConfig, CalendarAdapterKind, CalendarEvent,
    CalendarEventSource, IcsExporter, IcsImporter,
};
use chrono::{TimeZone, Utc};

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
