// sigil: REPAIR
import { invoke } from '@tauri-apps/api/core'
import { envEndpointUrl } from './endpointConfig'

const IS_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
const CHARON_BASE_URL = envEndpointUrl({ url: import.meta.env.VITE_CHARON_BASE_URL, port: 5110 })

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

export interface CharonLiveSnapshot {
  health: CharonHealthPayload | null
  capabilities: CharonCapabilitiesPayload | null
  providerCandidates: CharonProviderCandidatesPayload | null
  loadedAt: string | null
}

async function readCharonJson<T>(path: string): Promise<T> {
  if (IS_TAURI) {
    return invoke<T>('read_charon_json', { path })
  }
  const response = await fetch(`${CHARON_BASE_URL}${path}`, {
    headers: { Accept: 'application/json' },
  })
  if (!response.ok) {
    throw new Error(`Charon ${path} returned ${response.status}`)
  }
  return response.json() as Promise<T>
}

export async function loadCharonLiveSnapshot(): Promise<CharonLiveSnapshot> {
  const [health, capabilities, providerCandidates] = await Promise.all([
    readCharonJson<CharonHealthPayload>('/health'),
    readCharonJson<CharonCapabilitiesPayload>('/providers/capabilities'),
    readCharonJson<CharonProviderCandidatesPayload>('/provider_candidates'),
  ])
  return {
    health,
    capabilities,
    providerCandidates,
    loadedAt: new Date().toISOString(),
  }
}
