// sigil: REPAIR

export type WorkstationRoleId = 'fleet' | 'work' | 'decisions' | 'knowledge' | 'evidence' | 'settings'
export type WorkstationPresentationMode = 'in_scene' | 'native_window'

export interface WorkstationRoleDefinition {
  id: WorkstationRoleId
  label: string
  description: string
  purpose: string
  operatorQuestion: string
  previewKinds: string[]
  focusedCapabilities: string[]
  defaultPresentationModes: WorkstationPresentationMode[]
  debugRawAllowed: boolean
}

export const WORKSTATION_ROLE_IDS: WorkstationRoleId[] = [
  'fleet',
  'work',
  'decisions',
  'knowledge',
  'evidence',
  'settings',
]

export const WORKSTATION_ROLE_DEFINITIONS: WorkstationRoleDefinition[] = [
  {
    id: 'fleet',
    label: 'Fleet',
    description: 'Operator and provider fleet health, routing, lane ownership, and capacity.',
    purpose: 'Health of model/API/provider connectivity and runtime services.',
    operatorQuestion: 'Which providers, routes, and lanes are healthy and available?',
    previewKinds: [
      'provider_health_bars',
      'latency_waveform',
      'local_cloud_split',
      'error_pulse_count',
      'routable_model_count',
      'animated_line_noise',
    ],
    focusedCapabilities: [
      'provider_list',
      'local_cloud_source_labels',
      'model_availability',
      'latency_error_failure_history',
      'routing_lane_ownership',
      'service_checks',
      'read_only_refresh_actions',
    ],
    defaultPresentationModes: ['in_scene', 'native_window'],
    debugRawAllowed: false,
  },
  {
    id: 'work',
    label: 'Work',
    description: 'Current queue, active tasks, lifecycle state, and execution flow.',
    purpose: 'Active tasks, plans, queues, scheduled runs, and execution receipts.',
    operatorQuestion: 'What work is active, blocked, scheduled, or owned right now?',
    previewKinds: [
      'active_pending_completed_counts',
      'queue_flow_bands',
      'blocked_item_indicator',
      'latest_receipt_pulse',
    ],
    focusedCapabilities: [
      'active_queue',
      'plan_task_relationship',
      'scheduled_operations',
      'dry_run_controls',
      'task_capture_preview',
      'execution_receipts',
    ],
    defaultPresentationModes: ['in_scene', 'native_window'],
    debugRawAllowed: false,
  },
  {
    id: 'decisions',
    label: 'Decisions',
    description: 'Governed approvals, recommendations, and decision gates.',
    purpose: 'Human gates, approvals, policy reviews, and pending delegations.',
    operatorQuestion: 'What governed actions are waiting for operator judgment?',
    previewKinds: [
      'pending_gate_count',
      'risk_severity_colors',
      'oldest_wait_timer',
      'compact_provenance_indicator',
    ],
    focusedCapabilities: [
      'approval_packet_list',
      'reason_consequence_evidence',
      'approve_reject_dry_run_controls',
    ],
    defaultPresentationModes: ['in_scene', 'native_window'],
    debugRawAllowed: false,
  },
  {
    id: 'knowledge',
    label: 'Knowledge',
    description: 'Knowledge triage, memory, source maps, and context surfaces.',
    purpose: 'Memory, source freshness, docs, citations, and unresolved conflicts.',
    operatorQuestion: 'Which sources, memories, and citations are fresh, stale, or contested?',
    previewKinds: [
      'freshness_rings',
      'citation_source_pulses',
      'conflict_count',
      'ingestion_activity',
    ],
    focusedCapabilities: [
      'source_map',
      'knowledge_graph_status',
      'citations',
      'stale_missing_projection_list',
      'media_docs_viewer',
    ],
    defaultPresentationModes: ['in_scene', 'native_window'],
    debugRawAllowed: false,
  },
  {
    id: 'evidence',
    label: 'Evidence',
    description: 'Evidence ledger, provenance, source freshness, and audit trails.',
    purpose: 'Trust, provenance, audits, validation, and known gaps.',
    operatorQuestion: 'What supported or missing evidence backs current claims?',
    previewKinds: [
      'validation_pass_fail_blocks',
      'source_freshness_strip',
      'audit_warning_pulses',
    ],
    focusedCapabilities: [
      'receipts',
      'audit_results',
      'source_provenance',
      'missing_projection_explanations',
      'debug_raw_disclosure',
    ],
    defaultPresentationModes: ['in_scene', 'native_window'],
    debugRawAllowed: false,
  },
  {
    id: 'settings',
    label: 'Settings',
    description: 'Operator settings and guarded configuration surfaces.',
    purpose: 'Operator customization, slot assignment, adapter/profile setup, and layout export/import.',
    operatorQuestion: 'How is this surface configured, assigned, and connected?',
    previewKinds: [
      'setup_readiness',
      'current_profile',
      'slot_assignment_mode',
    ],
    focusedCapabilities: [
      'slot_assignment',
      'preview_widget_editor',
      'adapter_profile_setup',
      'api_key_service_setup_guidance',
      'export_import_layout_profile',
    ],
    defaultPresentationModes: ['in_scene'],
    debugRawAllowed: true,
  },
]

const WORKSTATION_ROLE_DEFINITION_BY_ID = new Map(
  WORKSTATION_ROLE_DEFINITIONS.map((definition) => [definition.id, definition]),
)

export function getWorkstationRoleDefinition(roleId: string | null | undefined): WorkstationRoleDefinition | null {
  if (!roleId) return null
  return WORKSTATION_ROLE_DEFINITION_BY_ID.get(roleId as WorkstationRoleId) ?? null
}
