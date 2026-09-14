// sigil: REPAIR
import { invoke } from '@tauri-apps/api/core'
import { envEndpointUrl } from './endpointConfig'

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
const MANWE_BASE_URL = envEndpointUrl({ url: import.meta.env.VITE_MANWE_BASE_URL, port: 7171 })

export type CharonCapabilityState = 'passed' | 'failed' | 'expired' | 'unknown'

export interface CharonCapabilitySummary {
  receipt_model_count: number
  models_with_failed_tool_receipts: number
  models_with_failed_structured_output_receipts: number
  models_with_failed_streaming_receipts: number
  recent_capability_failures: number
  providers_with_no_capability_evidence: number
}

export interface CharonCapabilityReceiptView {
  state: CharonCapabilityState
  observed_at_utc: string | null
  expires_at_utc: string | null
  outcome_class: string | null
  status_code: number | null
  expired: boolean
}

export interface CharonCapabilityModelView {
  model_id: string
  is_default: boolean
  healthy: boolean
  capabilities: Record<string, CharonCapabilityReceiptView>
}

export interface CharonCapabilityProviderView {
  provider_id: string
  enabled: boolean
  access_tier: string
  evidence_state: string
  models: CharonCapabilityModelView[]
}

export interface CharonCapabilitiesPayload {
  ok: boolean
  capabilities: {
    schema_version: string
    generated_at_utc: string
    summary: CharonCapabilitySummary
    providers: CharonCapabilityProviderView[]
  }
}

export interface CharonPromotionCandidateView {
  id: string
  name: string
  status: string
  free_kind: string
  access_tier_candidate: string
  requires_adapter: boolean
  promotion_ready: boolean
  reasons: string[]
}

export interface CharonProviderCandidatesPayload {
  ok: boolean
  promotion_guard: {
    schema_version: string
    generated_at_utc: string
    active_capability_probes_enabled: boolean
    candidates: CharonPromotionCandidateView[]
  }
}

export interface CharonBudgetPressureProvider {
  provider_id: string
  provider_name: string
  level: string
  minute_usage_ratio: number | null
  day_usage_ratio: number | null
  in_cooldown: boolean
  cooldown_until_utc: string | null
}

export interface CharonHealthAlert {
  level: string
  message: string
  provider_id: string
  provider_name: string
}

export interface CharonHealthPayload {
  ok: boolean
  providers_enabled: number
  providers_healthy: number
  providers_ready: number
  providers_blocked: number
  recent_route_failures: number
  recent_route_successes: number
  alerts: CharonHealthAlert[]
  budget_pressure: {
    highest_level: string
    providers_total: number
    cooldown_total: number
    critical_total: number
    warning_total: number
    exhausted_total: number
    providers: CharonBudgetPressureProvider[]
  }
  route_guardrails: {
    hermes_tool_routing: string
    tool_execution_min_context_window: number
    low_context_tool_model_total: number
    tool_incompatible_model_total: number
    visible_reasoning_model_total: number
  }
}

export interface ManweLiveSnapshot {
  schemaVersion: 'arda.system-health.manwe.v1'
  state: 'healthy' | 'degraded' | 'partial' | 'unavailable'
  sourceRevision: string
  sourceTimeUtc: string
  recoveryAction: string | null
  sources: Array<{
    sourceId: 'health' | 'capabilities' | 'provider_candidates'
    path: string
    state: 'observed' | 'degraded' | 'unavailable'
    error: string | null
  }>
  health: CharonHealthPayload | null
  capabilities: CharonCapabilitiesPayload | null
  providerCandidates: CharonProviderCandidatesPayload | null
  loadedAt: string | null
}

async function readManweJson<T>(path: string): Promise<T> {
  const response = await fetch(`${MANWE_BASE_URL}${path}`, {
    headers: { Accept: 'application/json' },
  })
  if (!response.ok) {
    throw new Error(`Manwe ${path} returned ${response.status}`)
  }
  return response.json() as Promise<T>
}

export async function loadManweLiveSnapshot(): Promise<ManweLiveSnapshot> {
  if (IS_TAURI) {
    const projection = await invoke<Omit<ManweLiveSnapshot, 'loadedAt'>>('read_manwe_runtime_projection')
    return { ...projection, loadedAt: projection.sourceTimeUtc }
  }

  const observedAt = new Date().toISOString()
  const requests = [
    ['health', '/healthz'],
    ['capabilities', '/providers/capabilities'],
    ['provider_candidates', '/provider_candidates'],
  ] as const
  const results = await Promise.allSettled(requests.map(([, path]) => readManweJson<unknown>(path)))
  const values = results.map((result) => result.status === 'fulfilled' ? result.value : null)
  const sources = results.map((result, index) => ({
    sourceId: requests[index][0],
    path: requests[index][1],
    state: result.status === 'rejected'
      ? 'unavailable' as const
      : (result.value as { ok?: boolean }).ok === false
        ? 'degraded' as const
        : 'observed' as const,
    error: result.status === 'rejected' ? (result.reason instanceof Error ? result.reason.message : String(result.reason)) : null,
  }))
  const available = values.filter((value) => value !== null).length
  const state = available === 0
    ? 'unavailable' as const
    : available < requests.length
      ? 'partial' as const
      : sources.some((source) => source.state === 'degraded')
        ? 'degraded' as const
        : 'healthy' as const
  const recoveryAction = state === 'partial'
    ? 'Restore the unavailable Manwe projection source; observed sources remain authoritative.'
    : state === 'degraded'
      ? 'Inspect Manwe source diagnostics before routing new work.'
      : state === 'unavailable'
        ? 'Start or repair the configured Manwe runtime, then refresh system health.'
        : null
  return {
    schemaVersion: 'arda.system-health.manwe.v1',
    state,
    sourceRevision: `browser-development-${observedAt}`,
    sourceTimeUtc: observedAt,
    recoveryAction,
    sources,
    health: values[0] as CharonHealthPayload | null,
    capabilities: values[1] as CharonCapabilitiesPayload | null,
    providerCandidates: values[2] as CharonProviderCandidatesPayload | null,
    loadedAt: observedAt,
  }
}
