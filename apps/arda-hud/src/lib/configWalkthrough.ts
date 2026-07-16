// sigil: REPAIR
import { readFile, writeScopedFile } from './weathertop'
import { primarySigilForSource } from './soterionRender'
import type { JsonRecord } from './ardaSource'
import { parseJsonOrNull } from './jsonParse'

export interface ConfigWalkthroughProfile {
  id: string
  title: string
  glyph: string
  productPosture: string
  summary: string
  settings: Record<string, string | number>
  writes: string[]
  nextAction: string
}

export interface ConfigProjectionChange {
  path: string
  changed: boolean
  summary: string
  beforePreview: string
  afterPreview: string
}

export interface ConfigProjectionPreview {
  profileId: string
  profileTitle: string
  generatedAtUtc: string
  changes: ConfigProjectionChange[]
  validation: JsonRecord
  projectedFiles: Record<string, string>
}

function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as JsonRecord
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function getString(value: unknown, fallback = ''): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback
}

function getNumber(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function parseJson(content: string | null): JsonRecord {
  if (!content) return {}
  return asRecord(parseJsonOrNull<unknown>(content)) ?? {}
}

function stableJson(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`
}

function compactPreview(value: string, max = 420): string {
  const compact = value.replace(/\s+/g, ' ').trim()
  return compact.length > max ? `${compact.slice(0, max - 1)}…` : compact
}

function normalizeProfile(value: unknown): ConfigWalkthroughProfile | null {
  const record = asRecord(value)
  if (!record) return null
  const settings = asRecord(record.settings) ?? {}
  return {
    id: getString(record.id),
    title: getString(record.title, 'Untitled Profile'),
    glyph: getString(record.glyph, primarySigilForSource('prometheus')),
    productPosture: getString(record.product_posture, 'unclassified'),
    summary: getString(record.summary),
    settings: Object.fromEntries(
      Object.entries(settings).filter(([, setting]) => typeof setting === 'string' || typeof setting === 'number'),
    ) as Record<string, string | number>,
    writes: asArray(record.writes).map((write) => getString(write)).filter(Boolean),
    nextAction: getString(record.next_action),
  }
}

export function getConfigProfiles(contract: JsonRecord | null): ConfigWalkthroughProfile[] {
  return asArray(contract?.profiles).map(normalizeProfile).filter((profile): profile is ConfigWalkthroughProfile => profile !== null)
}

function budgetPolicy(providerBudget: string): JsonRecord {
  if (providerBudget === 'local_free') {
    return {
      mode: 'local_free',
      cost_default: 'free_only',
      external_spend_allowed: false,
      paid_provider_cap_usd_day: 0,
      provider_order: ['local', 'charon_local', 'fleet_local'],
    }
  }
  if (providerBudget === 'paid_allowed') {
    return {
      mode: 'paid_allowed',
      cost_default: 'quality_when_justified',
      external_spend_allowed: true,
      paid_provider_cap_usd_day: 25,
      provider_order: ['local', 'free_cloud', 'paid_cloud'],
    }
  }
  return {
    mode: 'balanced',
    cost_default: 'balanced',
    external_spend_allowed: false,
    paid_provider_cap_usd_day: 0,
    provider_order: ['local', 'free_cloud'],
  }
}

function governancePolicy(strictness: number): JsonRecord {
  return {
    strictness,
    approval_required_for_external_spend: strictness >= 3,
    approval_required_for_public_sharing: strictness >= 3,
    destructive_action_quorum: strictness >= 4 ? 'operator_plus_guardian' : 'operator',
    triad_required: strictness >= 4,
    athena_lookup_required_before_action: strictness >= 3,
  }
}

function charonPolicy(providerBudget: string): JsonRecord {
  const budget = budgetPolicy(providerBudget)
  return {
    provider_budget: providerBudget,
    external_provider_policy: budget.external_spend_allowed ? 'allow_with_budget' : 'deny_paid_without_override',
    paid_provider_cap_usd_day: budget.paid_provider_cap_usd_day,
    preferred_provider_order: budget.provider_order,
    fallback_policy: providerBudget === 'local_free' ? 'local_or_defer' : 'free_cloud_then_defer',
  }
}

function mergeProjection(existing: JsonRecord, projection: JsonRecord): JsonRecord {
  return {
    ...existing,
    ...projection,
    config_walkthrough: {
      ...(asRecord(existing.config_walkthrough) ?? {}),
      ...(asRecord(projection.config_walkthrough) ?? {}),
    },
  }
}

function buildProjectedJson(path: string, existing: JsonRecord, profile: ConfigWalkthroughProfile, generatedAtUtc: string): JsonRecord {
  const runtimeMode = getString(profile.settings.runtime_mode, profile.id)
  const autonomyLevel = getNumber(profile.settings.autonomy_level, 0)
  const nodeVisibility = getString(profile.settings.node_visibility, 'operational')
  const providerBudget = getString(profile.settings.provider_budget, 'balanced')
  const governanceStrictness = getNumber(profile.settings.governance_strictness, 3)
  const base = {
    config_walkthrough: {
      schema_version: 'arda.config_projection.v1',
      profile_id: profile.id,
      profile_title: profile.title,
      product_posture: profile.productPosture,
      projected_at_utc: generatedAtUtc,
    },
  }

  if (path.endsWith('runtime_settings.json')) {
    return mergeProjection(existing, {
      ...base,
      authority: 'config_walkthrough_projection',
      runtime_mode: runtimeMode,
      autonomy_level: autonomyLevel,
      node_visibility: nodeVisibility,
      provider_budget: providerBudget,
    })
  }
  if (path.endsWith('runtime_budget_policy.json')) {
    return mergeProjection(existing, {
      ...base,
      authority: 'config_walkthrough_budget_projection',
      provider_budget_policy: budgetPolicy(providerBudget),
    })
  }
  if (path.endsWith('governance_runtime.json')) {
    return mergeProjection(existing, {
      ...base,
      authority: 'config_walkthrough_governance_projection',
      guided_governance: governancePolicy(governanceStrictness),
    })
  }
  if (path.endsWith('model_control_surface.json')) {
    return mergeProjection(existing, {
      ...base,
      authority: 'config_walkthrough_model_projection',
      routing_posture: {
        runtime_mode: runtimeMode,
        provider_budget: providerBudget,
        autonomy_level: autonomyLevel,
        node_visibility: nodeVisibility,
      },
    })
  }
  if (path.endsWith('operator_runtime_status.json')) {
    return mergeProjection(existing, {
      ...base,
      authority: 'config_walkthrough_operator_projection',
      operator_controls: {
        active_profile_id: profile.id,
        runtime_mode: runtimeMode,
        node_visibility: nodeVisibility,
        autonomy_level: autonomyLevel,
      },
    })
  }
  if (path.endsWith('charon_router.json')) {
    return mergeProjection(existing, {
      ...base,
      authority: 'config_walkthrough_charon_projection',
      guided_routing_policy: charonPolicy(providerBudget),
    })
  }
  return mergeProjection(existing, base)
}

function buildValidation(profile: ConfigWalkthroughProfile, generatedAtUtc: string, changes: ConfigProjectionChange[]): JsonRecord {
  const missingTargets = changes.filter((change) => change.beforePreview === '').map((change) => change.path)
  const changedTargets = changes.filter((change) => change.changed).map((change) => change.path)
  return {
    schema_version: 'arda.config_walkthrough_validation.v1',
    authority: 'config_walkthrough_validator',
    generated_at_utc: generatedAtUtc,
    active_profile_id: profile.id,
    active_profile_title: profile.title,
    status: missingTargets.length > 0 ? 'ready_with_missing_projection_targets' : 'ready',
    missing_targets: missingTargets,
    changed_targets: changedTargets,
    provider_budget_projection: profile.settings.provider_budget,
    governance_strictness_projection: profile.settings.governance_strictness,
    safety: {
      apply_mode: 'operator_approved_scoped_write',
      destructive_changes: false,
      external_spend_enabled: profile.settings.provider_budget === 'paid_allowed',
    },
  }
}

export async function previewConfigProfile(
  rootPath: string,
  contract: JsonRecord | null,
  profileId: string,
): Promise<ConfigProjectionPreview> {
  const profiles = getConfigProfiles(contract)
  const profile = profiles.find((item) => item.id === profileId) ?? profiles[0]
  if (!profile) {
    throw new Error('No config profile available')
  }
  const generatedAtUtc = new Date().toISOString()
  const projectionTargets = [
    'core/state/runtime_settings.json',
    'core/state/runtime_budget_policy.json',
    'core/state/governance_runtime.json',
    'core/state/model_control_surface.json',
    'core/state/operator_runtime_status.json',
    'core/state/charon_router.json',
  ]
  const projectedFiles: Record<string, string> = {}
  const changes: ConfigProjectionChange[] = []

  for (const path of projectionTargets) {
    const result = await readFile(`${rootPath}/${path}`)
    const before = result.success ? (result.content ?? '') : ''
    const after = stableJson(buildProjectedJson(path, parseJson(before), profile, generatedAtUtc))
    projectedFiles[path] = after
    changes.push({
      path,
      changed: before.trim() !== after.trim(),
      summary: before ? 'merge config walkthrough projection' : 'create config walkthrough projection',
      beforePreview: compactPreview(before),
      afterPreview: compactPreview(after),
    })
  }

  const validation = buildValidation(profile, generatedAtUtc, changes)
  projectedFiles['core/state/config_walkthrough_validation.json'] = stableJson(validation)
  changes.push({
    path: 'core/state/config_walkthrough_validation.json',
    changed: true,
    summary: 'write validation report',
    beforePreview: '',
    afterPreview: compactPreview(projectedFiles['core/state/config_walkthrough_validation.json']),
  })

  return {
    profileId: profile.id,
    profileTitle: profile.title,
    generatedAtUtc,
    changes,
    validation,
    projectedFiles,
  }
}

export async function applyConfigProfile(rootPath: string, preview: ConfigProjectionPreview): Promise<void> {
  for (const [path, content] of Object.entries(preview.projectedFiles)) {
    const result = await writeScopedFile(rootPath, path, content)
    if (!result.success) {
      throw new Error(result.error ?? `Failed to write ${path}`)
    }
  }
}
