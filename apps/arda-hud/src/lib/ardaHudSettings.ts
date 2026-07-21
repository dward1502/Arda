// sigil: REPAIR
import { getNumenorPath, readFile, writeScopedFile } from './weathertop'
import { loopbackUrl } from './endpointConfig'
import { parseJsonOrDefault } from './jsonParse'

export interface ArdaHudSettings {
  schema_version: string
  authority: string
  workspace_root: string
  core_state_root: string
  arda_snapshot_path: string
  arda_source_map_path: string
  remote_confidence_snapshot_path: string
  world_path: string
  human_context_path: string
  business_runtime_path: string
  personal_runtime_path: string
  runtime_settings_path: string
  config_walkthrough_profiles_path: string
  governance_runtime_path: string
  operations_flow_path: string
  soterion_render_contract_path: string
  paperclip_alignment_path: string
  escalation_runtime_path: string
  operator_actions_path: string
  output_topology_path: string
  output_accounting_path: string
  package_health_path: string
  package_enablement_path: string
  package_runtime_activation_path: string
  model_control_surface_path: string
  async_user_intake_contract_path: string
  async_user_intake_runtime_path: string
  agent_continuity_contract_path: string
  intake_confidence_ladder_path: string
  operator_legibility_contract_path: string
  manwe_router_path: string
  storage_pressure_path: string
  queue_active_path: string
  queue_summary_path: string
  fleet_runtime_drift_path: string
  task_lifecycle_runtime_path: string
  operator_runtime_status_path: string
  human_augmentation_runtime_path: string
  ceo_council_runtime_path: string
  autonomy_readiness_path: string
  autonomy_daily_eval_path: string
  autonomy_hold_reason_path: string
  professionalization_phase6_matrix_path: string
  professionalization_phase8_backlog_path: string
  athena_runtime_path: string
  athena_digest_path: string
  athena_deep_graph_path: string
  athena_policy_readiness_path: string
  human_root: string
  human_plan_root: string
  human_plan_index_path: string
  core_plan_root: string
  core_plan_index_path: string
  task_queue_path: string
  manwe_service_base_url: string
  hermes_dashboard_service_base_url: string
}

export const ARDA_HUD_SETTINGS_RELATIVE_PATH = 'apps/arda-hud/arda_hud.settings.json'

export const DEFAULT_ARDA_HUD_SETTINGS: ArdaHudSettings = {
  schema_version: 'arda.arda_hud.settings.v1',
  authority: 'arda_hud_settings',
  workspace_root: '.',
  core_state_root: 'core/state',
  arda_snapshot_path: 'core/state/arda_snapshot.json',
  arda_source_map_path: 'core/state/arda_source_map.json',
  remote_confidence_snapshot_path: 'core/state/remote_confidence_snapshot.json',
  world_path: 'core/state/world.json',
  human_context_path: 'core/state/human_context.json',
  business_runtime_path: 'core/state/business_runtime.json',
  personal_runtime_path: 'core/state/personal_runtime.json',
  runtime_settings_path: 'core/state/runtime_settings.json',
  config_walkthrough_profiles_path: 'core/state/config_walkthrough_profiles.json',
  governance_runtime_path: 'core/state/governance_runtime.json',
  operations_flow_path: 'core/state/operations_flow.json',
  soterion_render_contract_path: '/var/home/mythos/Arda/core/state/soterion_render_contract.json',
  paperclip_alignment_path: 'core/state/paperclip_alignment.json',
  escalation_runtime_path: 'core/state/escalation_runtime.json',
  operator_actions_path: 'core/state/operator_actions.json',
  output_topology_path: 'core/state/output_topology.json',
  output_accounting_path: 'core/state/output_accounting.json',
  package_health_path: 'core/state/package_health.json',
  package_enablement_path: 'core/state/package_enablement.json',
  package_runtime_activation_path: 'core/state/package_runtime_activation.json',
  model_control_surface_path: 'core/state/model_control_surface.json',
  async_user_intake_contract_path: 'core/state/async_user_intake_contract.json',
  async_user_intake_runtime_path: 'core/state/async_user_intake_runtime.json',
  agent_continuity_contract_path: 'core/state/agent_continuity_contract.json',
  intake_confidence_ladder_path: 'core/state/intake_confidence_ladder.json',
  operator_legibility_contract_path: 'core/state/operator_legibility_contract.json',
  manwe_router_path: 'core/state/manwe_router.json',
  storage_pressure_path: 'core/state/storage_pressure.json',
  queue_active_path: 'core/state/queue_active.json',
  queue_summary_path: 'core/state/queue_summary.json',
  fleet_runtime_drift_path: 'data/prometheus/fleet_runtime_drift_last.json',
  task_lifecycle_runtime_path: 'core/state/task_lifecycle_runtime.json',
  operator_runtime_status_path: 'core/state/operator_runtime_status.json',
  human_augmentation_runtime_path: 'core/state/human_augmentation_runtime.json',
  ceo_council_runtime_path: 'core/state/ceo_council_runtime.json',
  autonomy_readiness_path: 'core/state/autonomy_readiness.json',
  autonomy_daily_eval_path: 'data/autonomy/daily_eval_last.json',
  autonomy_hold_reason_path: 'data/autonomy/hold_reason_last.json',
  professionalization_phase6_matrix_path: 'audit/PROFESSIONALIZATION_AUDIT_2026-05-25/phase6-execution-matrix.jsonl',
  professionalization_phase8_backlog_path: 'audit/PROFESSIONALIZATION_AUDIT_2026-05-25/phase8-hardening-backlog.jsonl',
  athena_runtime_path: 'core/state/athena_runtime.json',
  athena_digest_path: 'data/athena/digest.jsonl',
  athena_deep_graph_path: 'data/athena/deep_graph.jsonl',
  athena_policy_readiness_path: 'data/athena/policy_readiness.jsonl',
  human_root: 'human',
  human_plan_root: 'docs/plans',
  human_plan_index_path: 'docs/plans/index.json',
  core_plan_root: 'core/projects/Plans',
  core_plan_index_path: 'core/state/plan_map.json',
  task_queue_path: 'core/projects/tasks/queue.jsonl',
  manwe_service_base_url: loopbackUrl({ port: 7200 }),
  hermes_dashboard_service_base_url: loopbackUrl({ port: 1421 }),
}

function asSettings(value: unknown): Partial<ArdaHudSettings> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {}
  }
  return value as Partial<ArdaHudSettings>
}

export async function loadArdaHudSettings(): Promise<{ rootPath: string; settingsPath: string; settings: ArdaHudSettings }> {
  const rootPath = await getNumenorPath()
  const settingsPath = `${rootPath}/${ARDA_HUD_SETTINGS_RELATIVE_PATH}`
  const result = await readFile(settingsPath)

  if (!result.success || !result.content) {
    return { rootPath, settingsPath, settings: DEFAULT_ARDA_HUD_SETTINGS }
  }

  const parsed = parseJsonOrDefault<unknown>(result.content, {})
  return {
    rootPath,
    settingsPath,
    settings: {
      ...DEFAULT_ARDA_HUD_SETTINGS,
      ...asSettings(parsed),
    },
  }
}

export async function saveArdaHudSettings(rootPath: string, settings: ArdaHudSettings) {
  return writeScopedFile(rootPath, ARDA_HUD_SETTINGS_RELATIVE_PATH, JSON.stringify(settings, null, 2))
}
