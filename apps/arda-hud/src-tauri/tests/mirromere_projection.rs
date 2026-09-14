use arda_hud_lib::mirromere::{
    project_mirromere_surface_at, ContinuityFreshness, ContinuityPrivacyClass,
    ContinuityProjectionReference, HandoffState, LifecycleAggregateState,
    LifecycleProjectionReference, MirromereProjectionInput, MirromereProjectionSourceMode,
};
use arda_outpost_protocol::{
    MirromereAvailability, MirromereDisplayRole, MirromereFreshness, MirromereSceneId,
    MirromereSourceMode,
};
use chrono::{Duration, TimeZone, Utc};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 17, 21, 30, 0)
        .single()
        .expect("valid time")
}

fn lifecycle(state: LifecycleAggregateState) -> LifecycleProjectionReference {
    LifecycleProjectionReference {
        aggregate_state: state,
        observed_at: now(),
        evidence_ref: "system-lifecycle://2026-08-17T21:30:00Z".to_string(),
    }
}

fn continuity() -> ContinuityProjectionReference {
    ContinuityProjectionReference {
        generated_at: now(),
        freshness: ContinuityFreshness::Fresh,
        active: false,
        privacy_class: Some(ContinuityPrivacyClass::PublicRoom),
        handoff_id: None,
        handoff_state: None,
        research_focus_ref: None,
        evidence_ref: "continuity://projection/current".to_string(),
    }
}

fn input() -> MirromereProjectionInput {
    MirromereProjectionInput {
        display_role: MirromereDisplayRole::HudAperture,
        source_mode: MirromereProjectionSourceMode::Runtime,
        lifecycle: Some(lifecycle(LifecycleAggregateState::Healthy)),
        continuity: Some(continuity()),
    }
}

#[test]
fn healthy_runtime_projects_live_idle_scene() {
    let surface = project_mirromere_surface_at(input(), now()).expect("projection");
    assert_eq!(surface.scene.scene_id, MirromereSceneId::AmbientIdle);
    assert_eq!(surface.source_mode, MirromereSourceMode::Runtime);
    assert_eq!(surface.freshness, MirromereFreshness::Fresh);
    assert_eq!(surface.availability, MirromereAvailability::Available);
    assert_eq!(surface.evidence.len(), 2);
}

#[test]
fn stale_lifecycle_is_explicitly_degraded() {
    let mut request = input();
    request.lifecycle.as_mut().unwrap().observed_at = now() - Duration::seconds(61);
    let surface = project_mirromere_surface_at(request, now()).expect("projection");
    assert_eq!(surface.scene.scene_id, MirromereSceneId::SystemDegraded);
    assert_eq!(surface.freshness, MirromereFreshness::Stale);
}

#[test]
fn unavailable_lifecycle_fails_closed_to_offline() {
    let mut request = input();
    request.lifecycle = None;
    let surface = project_mirromere_surface_at(request, now()).expect("projection");
    assert_eq!(surface.scene.scene_id, MirromereSceneId::OfflineLocal);
    assert_eq!(surface.availability, MirromereAvailability::Unavailable);
    assert_eq!(surface.freshness, MirromereFreshness::Unavailable);
}

#[test]
fn continuity_outage_degrades_but_does_not_hide_healthy_lifecycle() {
    let mut request = input();
    request.continuity.as_mut().unwrap().freshness = ContinuityFreshness::Unavailable;
    let surface = project_mirromere_surface_at(request, now()).expect("projection");
    assert_eq!(surface.scene.scene_id, MirromereSceneId::SystemDegraded);
    assert_eq!(surface.availability, MirromereAvailability::Available);
    assert_eq!(surface.freshness, MirromereFreshness::Unavailable);
}

#[test]
fn private_continuity_is_veiled_on_public_hud_aperture() {
    let mut request = input();
    let continuity = request.continuity.as_mut().unwrap();
    continuity.privacy_class = Some(ContinuityPrivacyClass::OperatorPrivate);
    continuity.evidence_ref = "continuity://projection/private-handoff-id".to_string();
    let surface = project_mirromere_surface_at(request, now()).expect("projection");
    assert_eq!(surface.scene.scene_id, MirromereSceneId::PrivacyVeil);
    assert!(surface.allowed_interactions.is_empty());
    assert_eq!(
        surface.evidence[1].evidence_ref,
        "continuity://projection/withheld"
    );
}

#[test]
fn prepared_handoff_preserves_reference_and_requires_explicit_interaction() {
    let mut request = input();
    let continuity = request.continuity.as_mut().unwrap();
    continuity.handoff_id = Some("handoff-1".to_string());
    continuity.handoff_state = Some(HandoffState::Prepared);
    let surface = project_mirromere_surface_at(request, now()).expect("projection");
    assert_eq!(
        surface.scene.scene_id,
        MirromereSceneId::ContinuityHandoffReady
    );
    assert_eq!(
        surface.evidence[1].evidence_ref,
        "continuity://projection/current"
    );
    assert!(surface.allowed_interactions.iter().any(|id| {
        matches!(
            id,
            arda_outpost_protocol::MirromereInteractionId::ContinueHandoff
        )
    }));
}

#[test]
fn research_focus_preserves_varda_provenance() {
    let mut request = input();
    request.continuity.as_mut().unwrap().research_focus_ref =
        Some("varda:research:focus/task7-operator-session".to_string());
    let surface = project_mirromere_surface_at(request, now()).expect("projection");
    assert_eq!(surface.scene.scene_id, MirromereSceneId::ResearchFocus);
    assert!(surface.evidence.iter().any(|evidence| {
        evidence.source_id == "arda.varda.research-focus.v1"
            && evidence.evidence_ref == "varda:research:focus/task7-operator-session"
    }));
}

#[test]
fn runtime_backend_rejects_fixture_source_mode() {
    let mut request = input();
    request.source_mode = MirromereProjectionSourceMode::Fixture;
    assert!(project_mirromere_surface_at(request, now()).is_err());
}

#[test]
fn tauri_handler_registers_the_bounded_projection_command() {
    let lib_source = include_str!("../src/lib.rs");
    assert!(lib_source.contains("mirromere::get_mirromere_surface"));
    assert!(!lib_source.contains("open_mirromere_window"));
    assert!(!lib_source.contains("list_mirromere_displays"));
    assert!(!lib_source.contains("__view=mirromere"));
}
