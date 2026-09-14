use arda_mirromere::{
    project_mirromere_surface_at, ContinuityFreshness, ContinuityPrivacyClass,
    ContinuityProjectionReference, HandoffState, LifecycleAggregateState,
    LifecycleProjectionReference, MirromereInteractionOutcome, MirromereInteractionReceiptState,
    MirromereInteractionRequest, MirromereInteractionStatus, MirromereProjectionInput,
    MirromereProjectionSourceMode,
};
use arda_outpost_protocol::{
    MirromereAvailability, MirromereDisplayRole, MirromereFreshness, MirromereInteractionId,
    MirromerePrivacyClass, MirromereSceneId,
};
use chrono::{Duration, TimeZone, Utc};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 20, 20, 0, 0)
        .single()
        .expect("valid time")
}

fn lifecycle(state: LifecycleAggregateState) -> LifecycleProjectionReference {
    LifecycleProjectionReference {
        aggregate_state: state,
        observed_at: now(),
        evidence_ref: "system-lifecycle://test".to_string(),
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

fn input(display_role: MirromereDisplayRole) -> MirromereProjectionInput {
    MirromereProjectionInput {
        display_role,
        source_mode: MirromereProjectionSourceMode::Runtime,
        lifecycle: Some(lifecycle(LifecycleAggregateState::Healthy)),
        continuity: Some(continuity()),
    }
}

fn request(
    surface: arda_outpost_protocol::MirromereSurfaceProjection,
    interaction_id: MirromereInteractionId,
    explicit_operator_action: bool,
) -> MirromereInteractionRequest {
    MirromereInteractionRequest {
        presented_privacy_class: surface.privacy.privacy_class,
        visibility_ceiling: surface.privacy.visibility_ceiling,
        surface,
        interaction_id,
        requested_at: now(),
        explicit_operator_action,
    }
}

#[test]
fn lifecycle_and_continuity_project_registered_scenes() {
    let idle = project_mirromere_surface_at(input(MirromereDisplayRole::HudAperture), now())
        .expect("idle projection");
    assert_eq!(idle.scene.scene_id, MirromereSceneId::AmbientIdle);

    let mut handoff = input(MirromereDisplayRole::NativeOutpost);
    let continuity = handoff.continuity.as_mut().expect("continuity");
    continuity.handoff_id = Some("handoff-c1".to_string());
    continuity.handoff_state = Some(HandoffState::Prepared);
    continuity.privacy_class = Some(ContinuityPrivacyClass::OperatorPrivate);
    let handoff = project_mirromere_surface_at(handoff, now()).expect("handoff projection");
    assert_eq!(
        handoff.scene.scene_id,
        MirromereSceneId::ContinuityHandoffReady
    );
}

#[test]
fn privacy_veils_private_continuity_on_public_aperture() {
    let mut request = input(MirromereDisplayRole::HudAperture);
    let continuity = request.continuity.as_mut().expect("continuity");
    continuity.privacy_class = Some(ContinuityPrivacyClass::OperatorPrivate);
    continuity.evidence_ref = "continuity://projection/private".to_string();

    let surface = project_mirromere_surface_at(request, now()).expect("privacy projection");
    assert_eq!(surface.scene.scene_id, MirromereSceneId::PrivacyVeil);
    assert_eq!(
        surface.privacy.privacy_class,
        MirromerePrivacyClass::PublicAmbient
    );
    assert!(surface.allowed_interactions.is_empty());
    assert_eq!(
        surface.evidence[1].evidence_ref,
        "continuity://projection/withheld"
    );
}

#[test]
fn stale_and_offline_sources_fail_closed() {
    let mut stale = input(MirromereDisplayRole::HudAperture);
    stale.lifecycle.as_mut().expect("lifecycle").observed_at = now() - Duration::seconds(31);
    let stale = project_mirromere_surface_at(stale, now()).expect("stale projection");
    assert_eq!(stale.scene.scene_id, MirromereSceneId::SystemDegraded);
    assert_eq!(stale.freshness, MirromereFreshness::Stale);

    let mut offline = input(MirromereDisplayRole::HudAperture);
    offline.lifecycle = None;
    let offline = project_mirromere_surface_at(offline, now()).expect("offline projection");
    assert_eq!(offline.scene.scene_id, MirromereSceneId::OfflineLocal);
    assert_eq!(offline.availability, MirromereAvailability::Unavailable);
}

#[test]
fn lifecycle_starting_precedes_expected_continuity_unavailability() {
    let mut starting = input(MirromereDisplayRole::HudAperture);
    starting.lifecycle = Some(lifecycle(LifecycleAggregateState::Starting));
    starting.continuity.as_mut().expect("continuity").freshness = ContinuityFreshness::Unavailable;

    let starting = project_mirromere_surface_at(starting, now()).expect("starting projection");
    assert_eq!(starting.scene.scene_id, MirromereSceneId::SystemStarting);
    assert_eq!(starting.freshness, MirromereFreshness::Unavailable);
}

#[test]
fn receipts_require_backend_issued_current_surface_and_explicit_mutation() {
    let state = MirromereInteractionReceiptState::default();
    let surface = project_mirromere_surface_at(input(MirromereDisplayRole::HudAperture), now())
        .expect("surface");

    let not_current = state
        .record(
            request(
                surface.clone(),
                MirromereInteractionId::InspectProvenance,
                false,
            ),
            now(),
        )
        .expect("rejection receipt");
    assert_eq!(not_current.outcome, MirromereInteractionOutcome::Rejected);
    assert_eq!(not_current.status, MirromereInteractionStatus::Rejected);
    assert_eq!(not_current.reason, "surface_not_current");

    state
        .remember_surface(surface.clone())
        .expect("remember surface");
    let accepted = state
        .record(
            request(surface, MirromereInteractionId::InspectProvenance, false),
            now(),
        )
        .expect("accepted receipt");
    assert_eq!(accepted.outcome, MirromereInteractionOutcome::Accepted);
    assert_eq!(accepted.status, MirromereInteractionStatus::Requested);

    let mut handoff = input(MirromereDisplayRole::NativeOutpost);
    let continuity = handoff.continuity.as_mut().expect("continuity");
    continuity.handoff_id = Some("handoff-c1".to_string());
    continuity.handoff_state = Some(HandoffState::Prepared);
    continuity.privacy_class = Some(ContinuityPrivacyClass::OperatorPrivate);
    let handoff = project_mirromere_surface_at(handoff, now()).expect("handoff surface");
    state
        .remember_surface(handoff.clone())
        .expect("remember handoff");
    let rejected = state
        .record(
            request(handoff, MirromereInteractionId::ContinueHandoff, false),
            now(),
        )
        .expect("operator-action rejection");
    assert_eq!(rejected.reason, "explicit_operator_action_required");
}
