import type { JsonRecord } from '../lib/ardaSource'
import {
  getSceneSlotWorkstationManifestById,
  getSceneSlotWorkstationManifestByZoneId,
} from '../scene/workstations/sceneSlotWorkstationTemplates'
import { type ArdaSection, type ArdaSceneZone, type ArdaWorkstationManifest, type ArdaBundle } from './ardaSource'
import { resolveAgentSigilFromContract } from './soterionRender'
import {ReviewGateItem, OperatorCockpitSurface} from "../components/arda/types";
export function getString(value: unknown, fallback = 'n/a'): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback
}

export function getNumber(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

export function getBoolean(value: unknown, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback
}

export function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`
}

export function formatMetric(value: number): string {
  return Number.isInteger(value) ? `${value}` : value.toFixed(2)
}
export function getTimestamp(value: JsonRecord): string {
  return getString(
    value.ts_utc ??
      value.created_at_utc ??
      value.completed_at_utc ??
      value.received_at_utc ??
      value.generated_at_utc,
    'not recorded',
  )
}

export function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let size = value
  let unitIndex = 0
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex += 1
  }
  return `${size >= 10 || unitIndex === 0 ? size.toFixed(0) : size.toFixed(1)} ${units[unitIndex]}`
}


export function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as JsonRecord
}

export function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}





export function getSectionById(sections: ArdaSection[], activeId: string | null): ArdaSection | null {
  if (!activeId) return sections[0] ?? null
  return sections.find((section) => section.id === activeId) ?? null
}

export function getSceneZoneById(zones: ArdaSceneZone[], activeId: string | null): ArdaSceneZone | null {
  if (!activeId) return zones[0] ?? null
  return zones.find((zone) => zone.id === activeId) ?? null
}

export function getWorkstationManifestByZoneId(
  manifests: ArdaWorkstationManifest[],
  zoneId: string | null,
): ArdaWorkstationManifest | null {
  if (!zoneId) return null
  const sceneSlotManifest = getSceneSlotWorkstationManifestByZoneId(zoneId)
  if (sceneSlotManifest) return sceneSlotManifest
  if (zoneId === 'settings') {
    return {
      id: 'settings_workstation',
      title: 'Settings',
      source_zone_id: 'settings',
      entry_anchor_id: 'settings_workstation_entry',
      module_ids: ['settings'],
      presentation_modes: ['in_scene', 'native_window'],
    }
  }
  if (zoneId === 'hermes_runtime') {
    return {
      id: 'hermes_dashboard_workstation',
      title: 'Hermes Dashboard',
      source_zone_id: 'hermes_runtime',
      entry_anchor_id: 'hermes_dashboard_entry',
      module_ids: ['hermes_dashboard', 'operations_and_packages'],
      presentation_modes: ['in_scene', 'native_window'],
    }
  }
  return manifests.find((manifest) => manifest.source_zone_id === zoneId) ?? null
}

export function getWorkstationManifestById(
  manifests: ArdaWorkstationManifest[],
  manifestId: string | null,
): ArdaWorkstationManifest | null {
  if (!manifestId) return null
  const sceneSlotManifest = getSceneSlotWorkstationManifestById(manifestId)
  if (sceneSlotManifest) return sceneSlotManifest
  if (manifestId === 'settings_workstation') {
    return {
      id: 'settings_workstation',
      title: 'Settings',
      source_zone_id: 'settings',
      entry_anchor_id: 'settings_workstation_entry',
      module_ids: ['settings'],
      presentation_modes: ['in_scene', 'native_window'],
    }
  }
  if (manifestId === 'hermes_dashboard_workstation') {
    return {
      id: 'hermes_dashboard_workstation',
      title: 'Hermes Dashboard',
      source_zone_id: 'hermes_runtime',
      entry_anchor_id: 'hermes_dashboard_entry',
      module_ids: ['hermes_dashboard', 'operations_and_packages'],
      presentation_modes: ['in_scene', 'native_window'],
    }
  }
  return manifests.find((manifest) => manifest.id === manifestId) ?? null
}

export function getHumanDocs(bundle: ArdaBundle): Array<{ title: string; path: string; body_preview: string }> {
  const portal = asRecord(bundle.humanContext?.human_portal)
  const docs = asArray(portal?.docs)
  return docs
    .map((doc) => asRecord(doc))
    .filter((doc): doc is JsonRecord => doc !== null)
    .map((doc) => ({
      title: getString(doc.title, 'Untitled'),
      path: getString(doc.path),
      body_preview: getString(doc.body_preview, ''),
    }))
}

export function getHumanNotes(bundle: ArdaBundle): Array<{ title: string; path: string; body_preview: string }> {
  const portal = asRecord(bundle.humanContext?.human_portal)
  const notes = asArray(portal?.notes)
  return notes
    .map((note) => asRecord(note))
    .filter((note): note is JsonRecord => note !== null)
    .slice(0, 6)
    .map((note) => ({
      title: getString(note.title, 'Untitled'),
      path: getString(note.path),
      body_preview: getString(note.body_preview, ''),
    }))
}

export function getAgents(bundle: ArdaBundle): Array<{ name: string; realm: string; status: string; trustScore: number; sigil: string }> {
  return asArray(bundle.world?.agents)
    .map((agent) => asRecord(agent))
    .filter((agent): agent is JsonRecord => agent !== null)
    .map((agent) => ({
      name: getString(agent.name, 'Unknown'),
      realm: getString(agent.realm, 'unknown'),
      status: getString(agent.status, 'UNKNOWN'),
      trustScore: getNumber(agent.trust_score, 0),
      sigil: resolveAgentSigilFromContract(asRecord(bundle.soterionRenderContract), agent),
    }))
}

export function getPackageTools(bundle: ArdaBundle): Array<{ tool: string; status: string; version: string; repo: string }> {
  return asArray(bundle.packageHealth?.tools)
    .map((tool) => asRecord(tool))
    .filter((tool): tool is JsonRecord => tool !== null)
    .slice(0, 8)
    .map((tool) => ({
      tool: getString(tool.tool, 'unknown'),
      status: getString(tool.observation_status, 'unknown'),
      version: getString(tool.version_hint, 'version hidden'),
      repo: getString(tool.repo, 'n/a'),
    }))
}

export function getPackageEnablement(bundle: ArdaBundle): {
  summary: Array<{ label: string; value: string }>
  tools: Array<{ tool: string; lane: string; state: string; readiness: string; nextAction: string }>
} {
  const enablement = asRecord(bundle.packageEnablement)
  const summary = asRecord(enablement?.summary)
  return {
    summary: [
      { label: 'Policy Ready', value: `${getNumber(summary?.policy_ready_total, 0)}` },
      { label: 'Activation Ready', value: `${getNumber(summary?.ready_for_activation_total, 0)}` },
      { label: 'Config Ready', value: `${getNumber(summary?.configuration_ready_total, 0)}` },
      { label: 'Evidence Ready', value: `${getNumber(summary?.evidence_ready_total, 0)}` },
      { label: 'Observed Only', value: `${getNumber(summary?.observed_only_total, 0)}` },
    ],
    tools: asArray(enablement?.tools)
      .map((tool) => asRecord(tool))
      .filter((tool): tool is JsonRecord => tool !== null)
      .slice(0, 8)
      .map((tool) => ({
        tool: getString(tool.tool, 'unknown'),
        lane: getString(tool.integration_lane, 'unknown'),
        state: getString(tool.integration_state, 'unknown'),
        readiness: getString(tool.policy_readiness, 'untracked'),
        nextAction: getString(tool.next_action, 'n/a'),
      })),
  }
}

export function getPackageRuntimeActivation(bundle: ArdaBundle): Array<{ tool: string; status: string; detail: string; ok: string }> {
  const activation = asRecord(bundle.packageRuntimeActivation)
  const surfaces = asRecord(activation?.surfaces)
  return Object.entries(surfaces ?? {})
    .map(([tool, value]) => {
      const record = asRecord(value)
      const status = getString(record?.status, 'unknown')
      const detail =
        getString(record?.proxy_url, '') ||
        getString(record?.base_url, '') ||
        getString(record?.runtime_mode, '') ||
        getString(record?.tool_bin_dir, '') ||
        getString(record?.command, '') ||
        getString(record?.summary, 'n/a').split('\n')[0]
      return {
        tool,
        status,
        detail,
        ok: getBoolean(record?.ok, false) ? 'ok' : 'check',
      }
    })
    .sort((a, b) => a.tool.localeCompare(b.tool))
    .slice(0, 8)
}

export function getStorageStores(bundle: ArdaBundle): Array<{ path: string; changed: boolean; bytesAfter: number }> {
  const compaction = asRecord(bundle.storagePressure?.compaction)
  return asArray(compaction?.stores)
    .map((store) => asRecord(store))
    .filter((store): store is JsonRecord => store !== null)
    .slice(0, 6)
    .map((store) => ({
      path: getString(store.path),
      changed: getBoolean(store.changed, false),
      bytesAfter: getNumber(store.bytes_after, 0),
    }))
}

export function getStoragePressureSummary(bundle: ArdaBundle): {
  summary: Array<{ label: string; value: string }>
  roots: Array<{ path: string; classification: string; bytes: string }>
  candidates: Array<{ path: string; action: string; bytes: string }>
} {
  const pressure = asRecord(bundle.storagePressure)
  const summary = asRecord(pressure?.summary)
  const guard = asRecord(pressure?.pressure_guard)
  const observed = asRecord(guard?.observed)
  return {
    summary: [
      { label: '/var Used', value: `${getNumber(observed?.disk_used_pct, 0)}%` },
      { label: 'Workspace Seen', value: formatBytes(getNumber(summary?.total_observed_workspace_bytes, 0)) },
      { label: 'Rebuildable', value: formatBytes(getNumber(summary?.rebuildable_bytes, 0)) },
      { label: 'Operational', value: formatBytes(getNumber(summary?.operational_bytes, 0)) },
      { label: 'History', value: formatBytes(getNumber(summary?.history_bytes, 0)) },
      { label: 'Mirror', value: formatBytes(getNumber(summary?.accounting_mirror_bytes, 0)) },
    ],
    roots: asArray(pressure?.workspace_roots)
      .map((item) => asRecord(item))
      .filter((item): item is JsonRecord => item !== null)
      .sort((a, b) => getNumber(b.bytes, 0) - getNumber(a.bytes, 0))
      .slice(0, 5)
      .map((item) => ({
        path: getString(item.path, 'unknown'),
        classification: getString(item.classification, 'unknown'),
        bytes: formatBytes(getNumber(item.bytes, 0)),
      })),
    candidates: asArray(pressure?.reclaim_candidates)
      .map((item) => asRecord(item))
      .filter((item): item is JsonRecord => item !== null)
      .slice(0, 4)
      .map((item) => ({
        path: getString(item.path, 'unknown'),
        action: getString(item.recommended_action, 'observe_only'),
        bytes: formatBytes(getNumber(item.bytes, 0)),
      })),
  }
}

export function getOutputTopology(bundle: ArdaBundle): {
  surfaces: Array<{ id: string; path: string; classification: string; purpose: string }>
  candidates: Array<{ path: string; reason: string; action: string; priority: string; estimatedJoulework: number }>
  counts: { dataJsonl: number; humanMarkdown: number; historySnapshots: number }
} {
  const topology = asRecord(bundle.outputTopology)
  const counts = asRecord(topology?.counts)
  return {
    surfaces: asArray(topology?.surfaces)
      .map((surface) => asRecord(surface))
      .filter((surface): surface is JsonRecord => surface !== null)
      .map((surface) => ({
        id: getString(surface.id, 'unknown'),
        path: getString(surface.path),
        classification: getString(surface.classification, 'unknown'),
        purpose: getString(surface.purpose, ''),
      })),
    candidates: asArray(topology?.long_term_accounting_candidates)
      .map((candidate) => asRecord(candidate))
      .filter((candidate): candidate is JsonRecord => candidate !== null)
      .map((candidate) => ({
        path: getString(candidate.path),
        reason: getString(candidate.reason, ''),
        action: getString(candidate.recommended_action, 'mirror_tree'),
        priority: getString(candidate.priority, 'unknown'),
        estimatedJoulework: getNumber(candidate.estimated_joulework, 0),
      })),
    counts: {
      dataJsonl: getNumber(counts?.data_jsonl_files, 0),
      humanMarkdown: getNumber(counts?.human_markdown_files, 0),
      historySnapshots: getNumber(counts?.metrics_history_snapshots, 0),
    },
  }
}

export function getOutputAccounting(bundle: ArdaBundle): {
  mirrorRoot: string
  mode: string
  summary: Array<{ label: string; value: string }>
  candidates: Array<{ path: string; status: string; mirrorPath: string; bytes: string; skippedFiles: number; compressedFiles: number }>
} {
  const accounting = asRecord(bundle.outputAccounting)
  const summary = asRecord(accounting?.summary)
  return {
    mirrorRoot: getString(accounting?.mirror_root, 'data/accounting/output_mirror'),
    mode: getString(accounting?.mode, 'mirror_only_non_destructive'),
    summary: [
      { label: 'Candidates', value: `${getNumber(summary?.candidate_count, 0)}` },
      { label: 'Mirrored Files', value: `${getNumber(summary?.mirrored_files, 0)}` },
      { label: 'Mirrored MB', value: formatMetric(getNumber(summary?.mirrored_bytes, 0) / (1024 * 1024)) },
      { label: 'Observed MB', value: formatMetric(getNumber(summary?.observed_source_bytes, 0) / (1024 * 1024)) },
      { label: 'Est. JW', value: formatMetric(getNumber(summary?.estimated_joulework, 0)) },
      { label: 'Est. Minutes', value: `${getNumber(summary?.estimated_operator_minutes, 0)}` },
    ],
    candidates: asArray(accounting?.candidates)
      .map((candidate) => asRecord(candidate))
      .filter((candidate): candidate is JsonRecord => candidate !== null)
      .map((candidate) => ({
        path: getString(candidate.path),
        status: getString(candidate.status, 'unknown'),
        mirrorPath: getString(candidate.mirror_path, 'n/a'),
        bytes: formatMetric(getNumber(candidate.mirrored_bytes, 0) / (1024 * 1024)),
        skippedFiles: getNumber(candidate.skipped_files, 0),
        compressedFiles: getNumber(candidate.compressed_files, 0),
      })),
  }
}

export function getGovernanceRuntimeSignals(bundle: ArdaBundle): Array<{ label: string; value: string }> {
  const governance = asRecord(bundle.governanceRuntime)
  const signals = asRecord(governance?.signals)
  const derived = asRecord(governance?.derived)
  return [
    { label: 'Autonomy', value: formatMetric(getNumber(signals?.autonomy_observation_score, 0)) },
    { label: 'Gap', value: formatMetric(getNumber(derived?.autonomy_gap, 0)) },
    { label: 'JW', value: formatMetric(getNumber(signals?.avg_joulework, 0)) },
    { label: 'LE', value: formatMetric(getNumber(signals?.avg_love_eq, 0)) },
    { label: 'Bacon', value: formatMetric(getNumber(signals?.bacon_lite_recent_confidence, 0)) },
    { label: 'Triad', value: formatMetric(getNumber(signals?.triad_pass_rate, 0)) },
  ]
}

export function getOperationsFlowSummary(bundle: ArdaBundle): Array<{ label: string; value: string }> {
  const operations = asRecord(bundle.operationsFlow)
  const derived = asRecord(operations?.derived)
  const posture = asRecord(derived?.queue_posture)
  return [
    { label: 'Projects Queue', value: `${getNumber(posture?.projects_queue_queued, 0)}` },
    { label: 'Known Work', value: `${getNumber(posture?.total_known_work_items, 0)}` },
    { label: 'Escalations', value: `${getNumber(posture?.pending_escalations, 0)}` },
    { label: 'Lockdown', value: getBoolean(derived?.control_plane_ready, false) ? 'ready' : 'gap' },
    { label: 'Autonomy', value: getBoolean(derived?.autonomy_ready, false) ? 'ready' : 'degraded' },
  ]
}

export function getOperatorActions(bundle: ArdaBundle): {
  summary: Array<{ label: string; value: string }>
  actions: Array<{ title: string; owner: string; status: string; note: string }>
} {
  const actions = asRecord(bundle.operatorActions)
  const summary = asRecord(actions?.summary)
  return {
    summary: [
      { label: 'Human Needed', value: `${getNumber(summary?.human_needed_total, 0)}` },
      { label: 'External Blockers', value: `${getNumber(summary?.external_blockers_total, 0)}` },
      { label: 'Auth Required', value: `${getNumber(summary?.auth_required_total, 0)}` },
      { label: 'Config Required', value: `${getNumber(summary?.configuration_required_total, 0)}` },
    ],
    actions: asArray(actions?.actions)
      .map((item) => asRecord(item))
      .filter((item): item is JsonRecord => item !== null)
      .slice(0, 8)
      .map((item) => ({
        title: getString(item.title, 'Untitled action'),
        owner: getString(item.owner, 'unknown'),
        status: getString(item.status, 'unknown'),
        note: getString(item.note, 'n/a'),
      })),
  }
}


export function getPaperclipAlignment(bundle: ArdaBundle): {
  summary: Array<{ label: string; value: string }>
  domains: Array<{ label: string; value: string }>
  tasks: Array<{ title: string; owner: string; status: string }>
  evidence: Array<{ sourceId: string; readiness: string; confidence: string }>
} {
  const alignment = asRecord(bundle.paperclipAlignment)
  const derived = asRecord(alignment?.derived)
  const evidence = asRecord(alignment?.evidence)
  const readiness = asRecord(derived?.paperclip_readiness)

  return {
    summary: [
      { label: 'Evidence Ready', value: getBoolean(derived?.evidence_ready, false) ? 'yes' : 'no' },
      { label: 'Policy Ready', value: `${getNumber(evidence?.policy_ready_sources, 0)}/${getNumber(evidence?.expected_policy_ready_sources, 0)}` },
      { label: 'Open Tasks', value: `${getNumber(derived?.comparison_tasks_open, 0)}` },
    ],
    domains: [
      { label: 'Governance', value: getBoolean(readiness?.governance, false) ? 'aligned' : 'gap' },
      { label: 'Edge Runtime', value: getBoolean(readiness?.edge_runtime, false) ? 'aligned' : 'gap' },
      { label: 'Deployment', value: getBoolean(readiness?.deployment, false) ? 'aligned' : 'gap' },
      { label: 'Ledger', value: getBoolean(readiness?.ledger_topology, false) ? 'aligned' : 'gap' },
    ],
    tasks: asArray(evidence?.comparison_tasks)
      .map((item) => asRecord(item))
      .filter((item): item is JsonRecord => item !== null)
      .map((item) => ({
        title: getString(item.title, 'Untitled task'),
        owner: getString(item.owner, 'unknown'),
        status: getString(item.status, 'unknown'),
      })),
    evidence: asArray(evidence?.sources)
      .map((item) => asRecord(item))
      .filter((item): item is JsonRecord => item !== null)
      .map((item) => ({
        sourceId: getString(item.source_id, 'unknown'),
        readiness: getString(item.policy_readiness, 'unknown'),
        confidence: formatMetric(getNumber(item.confidence, 0)),
      })),
  }
}

export function getEscalationRuntime(bundle: ArdaBundle): {
  summary: Array<{ label: string; value: string }>
  reasons: Array<{ label: string; value: string }>
} {
  const escalation = asRecord(bundle.escalationRuntime)
  const summary = asRecord(escalation?.summary)
  return {
    summary: [
      { label: 'Pending', value: `${getNumber(summary?.pending_total, 0)}` },
      { label: 'Deduped', value: `${getNumber(summary?.pending_deduped, 0)}` },
      { label: 'Duplicates', value: `${getNumber(summary?.duplicate_pending_count, 0)}` },
    ],
    reasons: asArray(escalation?.reason_buckets)
      .map((item) => asRecord(item))
      .filter((item): item is JsonRecord => item !== null)
      .slice(0, 6)
      .map((item) => ({
        label: getString(item.reason, 'unknown'),
        value: `${getNumber(item.count, 0)}`,
      })),
  }
}
export function getGovernanceSummary(bundle: ArdaBundle): { ready: boolean; weights: Array<{ label: string; value: number }>; thresholds: Array<{ label: string; value: number }> } {
  const governance = asRecord(bundle.runtimeSettings?.governance)
  const weights = asRecord(governance?.weights)
  const thresholds = asRecord(governance?.thresholds)

  return {
    ready: getBoolean(asRecord(governance?.always_on)?.triad_influence, false),
    weights: Object.entries(weights ?? {})
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([label, value]) => ({
        label,
        value: getNumber(value, 0),
      })),
    thresholds: Object.entries(thresholds ?? {})
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([label, value]) => ({
        label,
        value: getNumber(value, 0),
      })),
  }
}

export function getAutonomyReadinessSummary(bundle: ArdaBundle): {
  posture: string
  checkpoint: Array<{ label: string; value: string }>
  evidence: Array<{ phase: string; title: string; status: string; source: string }>
  nextUnlocks: Array<{ title: string; status: string; requires: string }>
} {
  const readiness = asRecord(bundle.autonomyReadiness)
  const checkpoint = asRecord(readiness?.checkpoint)
  const evidence = asArray(readiness?.evidence)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
    .map((item) => ({
      phase: getString(item.phase, 'n/a'),
      title: getString(item.title, 'Untitled evidence'),
      status: getString(item.status, 'unknown'),
      source: getString(item.source, 'unknown source'),
    }))
  const nextUnlocks = asArray(readiness?.next_unlocks)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
    .map((item) => ({
      title: getString(item.title, 'Untitled unlock'),
      status: getString(item.status, 'unknown'),
      requires: asArray(item.requires).map((entry) => getString(entry)).filter(Boolean).join(', '),
    }))

  return {
    posture: getString(checkpoint?.overall_posture, 'unknown'),
    checkpoint: Object.entries(checkpoint ?? {})
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([label, value]) => ({
        label,
        value: getString(value, `${value}`),
      })),
    evidence,
    nextUnlocks,
  }
}

export function getSnapshotSectionStats(bundle: ArdaBundle): Array<{ id: string; status: string }> {
  return Object.entries(asRecord(bundle.snapshot?.sections) ?? {})
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([id, section]) => ({
      id,
      status: getString(asRecord(section)?.schema_version, 'unknown'),
    }))
}

export function getQueueSummary(bundle: ArdaBundle): { completed: number; priorities: Array<{ label: string; value: number }>; owners: Array<{ label: string; value: number }> } {
  const projectTasks = asRecord(bundle.queueSummary?.project_tasks)
  const countsByStatus = asRecord(projectTasks?.counts_by_status)
  const countsByPriority = asRecord(projectTasks?.counts_by_priority)
  const countsByOwner = asRecord(projectTasks?.counts_by_owner)

  return {
    completed: getNumber(countsByStatus?.completed, 0),
    priorities: Object.entries(countsByPriority ?? {})
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([label, value]) => ({
        label,
        value: getNumber(value, 0),
      })),
    owners: Object.entries(countsByOwner ?? {})
      .sort(([left], [right]) => left.localeCompare(right))
      .slice(0, 8)
      .map(([label, value]) => ({
        label,
        value: getNumber(value, 0),
      })),
  }
}

export function latestTaskEntries(entries: JsonRecord[]): JsonRecord[] {
  const byId = new Map<string, JsonRecord>()
  for (const entry of entries) {
    const id = getString(entry.id ?? entry.task_id, '')
    if (!id) continue
    byId.set(id, entry)
  }
  return [...byId.values()]
}

export function providerOperationalState(provider: JsonRecord): string {
  if (!getBoolean(provider.enabled, false)) return 'disabled'
  if (!getBoolean(provider.has_api_key, false)) return 'missing_api_key'
  const requestsPerDay = getNumber(provider.requests_per_day, NaN)
  const requestsUsedDay = getNumber(provider.requests_used_day, 0)
  const requestsPerMinute = getNumber(provider.requests_per_minute, NaN)
  const requestsUsedMinute = getNumber(provider.requests_used_minute, 0)
  if (Number.isFinite(requestsPerDay) && requestsPerDay > 0 && requestsUsedDay >= requestsPerDay) return 'rate_limited'
  if (Number.isFinite(requestsPerMinute) && requestsPerMinute > 0 && requestsUsedMinute >= requestsPerMinute) return 'minute_quota_exhausted'
  if (getBoolean(provider.in_cooldown, false)) return 'cooldown'
  if (!getBoolean(provider.healthy, false)) return 'unhealthy'
  if (getNumber(provider.consecutive_failures, 0) >= 2 || getNumber(provider.error_count, 0) >= 5) return 'degraded'
  return 'ready'
}

export function providerUsageRatio(used: unknown, max: unknown): number | null {
  const maxValue = getNumber(max, NaN)
  if (!Number.isFinite(maxValue) || maxValue <= 0) return null
  return Math.min(Math.max(getNumber(used, 0) / maxValue, 0), 1)
}

export function providerBudgetPressureLevel(provider: JsonRecord): string {
  const state = providerOperationalState(provider)
  const minuteRatio = providerUsageRatio(provider.requests_used_minute, provider.requests_per_minute)
  const dayRatio = providerUsageRatio(provider.requests_used_day, provider.requests_per_day)
  if (
    ['rate_limited', 'minute_quota_exhausted', 'cooldown'].includes(state) ||
    (minuteRatio !== null && minuteRatio >= 0.9) ||
    (dayRatio !== null && dayRatio >= 0.9)
  ) {
    return 'critical'
  }
  if ((minuteRatio !== null && minuteRatio >= 0.75) || (dayRatio !== null && dayRatio >= 0.75)) return 'warning'
  return 'ok'
}

export function getOperatorCockpitSurface(bundle: ArdaBundle, reviewGateItems: ReviewGateItem[]): OperatorCockpitSurface {
  const latestTasks = latestTaskEntries(bundle.taskQueueEntries)
  const openTasks = latestTasks
    .filter((entry) => ['queued', 'in_progress', 'blocked', 'pending'].includes(getString(entry.status, '')))
    .sort((left, right) => getTimestamp(right).localeCompare(getTimestamp(left)))
  const humanGates = reviewGateItems
    .filter((item) => ['pending_review', 'write_pending', 'blocked', 'human_required'].some((status) => item.status.includes(status)))
    .slice(0, 8)
  const repairPressure = asRecord(asRecord(bundle.controlLoopTruth.wardenGuardhouse?.queue)?.repair_pressure)
  const auditRunner = asRecord(bundle.controlLoopTruth.chronosStatus?.audit_runner)
  const scheduledTasks = asArray(auditRunner?.scheduled_tasks)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
  const dueTasks = scheduledTasks
    .filter((task) => getBoolean(task.due, false))
    .slice(0, 5)
    .map((task) => ({
      id: getString(task.id, 'unknown'),
      name: getString(task.name, 'Untitled audit'),
      cadence: getString(task.cadence, 'unknown'),
      owner: getString(task.owner, 'chronos'),
    }))
  const hermesReceiptRecords = [
    ...bundle.hermesAgentGatewayReceipts.map((receipt) => ({ ...(receipt as JsonRecord), __source: 'gateway' })),
    ...bundle.flywheelDispatchReceipts.map((receipt) => ({ ...(receipt as JsonRecord), __source: 'dispatch' })),
  ] as Array<JsonRecord & { __source: string }>
  const latestReceipts = hermesReceiptRecords
    .sort((left, right) => getTimestamp(right).localeCompare(getTimestamp(left)))
    .slice(0, 5)
    .map((receipt, index) => ({
      id: getString(receipt.receipt_id ?? receipt.task_id ?? receipt.ts_utc, `receipt-${index + 1}`),
      status: getString(receipt.status ?? (getBoolean(receipt.dry_run, false) ? 'dry_run' : 'recorded'), 'recorded'),
      task: getString(receipt.task_ref ?? receipt.task_id ?? receipt.packet_id, 'unlinked task'),
      source: getString(receipt.__source, 'receipt'),
    }))
  const latestReadinessBySource = new Map<string, JsonRecord>()
  for (const entry of bundle.athenaPolicyReadiness) {
    const sourceId = getString(entry.source_id, '')
    if (!sourceId) continue
    const previous = latestReadinessBySource.get(sourceId)
    if (!previous || getTimestamp(entry).localeCompare(getTimestamp(previous)) >= 0) {
      latestReadinessBySource.set(sourceId, entry)
    }
  }
  const readinessRecords = [...latestReadinessBySource.values()]
  const readinessCount = (readiness: string) =>
    readinessRecords.filter((entry) => getString(entry.policy_readiness, 'reference_only') === readiness).length
  const providerPressure = asRecord(bundle.charonRouter?.provider_pressure)
  const pressureProviders = [
    ...asArray(providerPressure?.providers),
    ...asArray(providerPressure?.cooldowns),
    ...asArray(providerPressure?.budget_pressure),
  ]
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)
  const uniquePressureProviders = new Map<string, JsonRecord>()
  for (const provider of pressureProviders) {
    const id = getString(provider.id, getString(provider.provider_id, 'unknown'))
    const state = getString(provider.operational_state, providerOperationalState(provider))
    const level = getString(provider.budget_pressure_level, providerBudgetPressureLevel(provider))
    uniquePressureProviders.set(`${id}:${state}:${level}`, {
      ...provider,
      operational_state: state,
      budget_pressure_level: level,
    })
  }
  const charonWarnings = [...uniquePressureProviders.values()]
    .filter((provider) => {
      const state = getString(provider.operational_state, 'ready')
      const level = getString(provider.budget_pressure_level, 'ok')
      const healthy = getBoolean(provider.healthy, true)
      const blocked = getBoolean(provider.operational_blocked, getBoolean(provider.blocked, false))
      return blocked || !healthy || state === 'cooldown' || level !== 'ok' || getBoolean(provider.in_cooldown, false)
    })
    .slice(0, 6)
    .map((provider) => ({
      providerId: getString(provider.id, getString(provider.provider_id, 'unknown')),
      state: getString(provider.operational_state, getBoolean(provider.in_cooldown, false) ? 'cooldown' : 'unknown'),
      level: getString(provider.budget_pressure_level, getBoolean(provider.in_cooldown, false) ? 'critical' : 'ok'),
      detail: getString(provider.last_error, getString(provider.cooldown_until_utc, 'provider pressure')),
    }))
  const routeGuardrails = asRecord(bundle.charonRouter?.route_guardrails)
  const providerRecords = asArray(providerPressure?.providers)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)
  const availableProviderCount = providerRecords.filter((provider) =>
    getBoolean(provider.enabled, false)
    && getBoolean(provider.healthy, false)
    && !getBoolean(provider.operational_blocked, getBoolean(provider.blocked, false))
  ).length
  const blockedProviderCount = providerRecords.filter((provider) =>
    getBoolean(provider.enabled, false)
    && getBoolean(provider.operational_blocked, getBoolean(provider.blocked, false))
  ).length
  const liveAutonomyReadiness = asRecord(bundle.ceoAutopilotState?.autonomy_readiness)
  const autonomyGateReasons = asArray(liveAutonomyReadiness?.reasons)
    .map((reason) => getString(reason))
    .filter(Boolean)
  const storageSummary = asRecord(bundle.storagePressure?.summary)
  const storageApply = asRecord(bundle.storagePressure?.latest_apply ?? bundle.storagePressure?.apply)
  const storageApplySummary = asRecord(storageApply?.summary)
  const storageClasses = asArray(bundle.storagePressure?.classes)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)
  const storageWarnings = storageClasses
    .filter((entry) => ['generated_tick_output', 'runtime_backup', 'rebuildable_temp', 'misplaced_model_artifact'].includes(getString(entry.category, '')))
    .slice(0, 5)
    .map((entry) => ({
      label: getString(entry.category, 'storage'),
      value: `${getNumber(entry.files, 0)} files`,
      detail: getString(entry.recommended_action, 'review'),
    }))
  const deletedBytes = getNumber(storageApplySummary?.deleted_bytes, 0)

  const statusSplit = latestTasks.reduce<Record<string, number>>((acc, entry) => {
    const status = getString(entry.status, 'unknown')
    acc[status] = (acc[status] || 0) + 1
    return acc
  }, {} as Record<string, number>)

  const queueStatusSplit = {
    ready: getNumber(statusSplit['ready'], 0) + getNumber(statusSplit['planned'], 0),
    pending: getNumber(statusSplit['pending'], 0) + getNumber(statusSplit['queued'], 0),
    inProgress: getNumber(statusSplit['in_progress'], 0),
    blocked: getNumber(statusSplit['blocked'], 0),
  }

  return {
    queue: {
      openTotal: openTasks.length,
      items: openTasks.slice(0, 8).map((entry) => ({
        id: getString(entry.id ?? entry.task_id, 'unknown'),
        title: getString(entry.title, 'Untitled task'),
        owner: getString(entry.owner, 'unknown'),
        status: getString(entry.status, 'unknown'),
        priority: getString(entry.priority, 'normal'),
      })),
      statusSplit: queueStatusSplit,
    },
    humanGates: {
      blockedTotal: humanGates.length,
      items: humanGates.map((item) => ({
        id: item.id,
        title: item.title,
        status: item.status,
        decisionClass: item.decisionClass,
      })),
    },
    warden: {
      effectiveAttention: getNumber(repairPressure?.effective_attention_required, 0),
      rawAttention: getNumber(repairPressure?.raw_attention_required, 0),
      repeatedNoise: getNumber(repairPressure?.repeated_repair_noise, 0),
      activeRepairFiles: getNumber(repairPressure?.active_repair_files, 0),
      resolvedRepairFiles: getNumber(repairPressure?.resolved_repair_files, 0),
    },
    chronos: {
      runnerStatus: getString(auditRunner?.runner_status, getString(bundle.controlLoopTruth.chronosStatus?.status, 'missing')),
      readyTaskCount: getNumber(auditRunner?.ready_task_count, 0),
      scheduledTaskCount: getNumber(auditRunner?.scheduled_task_count, scheduledTasks.length),
      dueTasks,
    },
    hermes: {
      gatewayReceiptCount: bundle.hermesAgentGatewayReceipts.length,
      dispatchReceiptCount: bundle.flywheelDispatchReceipts.length,
      latestReceipts,
    },
    athena: {
      policyReady: readinessCount('policy_ready'),
      referenceOnly: readinessCount('reference_only'),
      implementationReady: readinessCount('implementation_ready'),
      latest: readinessRecords.slice(-6).reverse().map((entry) => {
        const observed = asRecord(asRecord(entry.gate)?.observed)
        return {
          sourceId: getString(entry.source_id, 'unknown'),
          readiness: getString(entry.policy_readiness, 'unknown'),
          confidence: formatMetric(getNumber(observed?.confidence, 0)),
        }
      }),
    },
    charon: {
      providerCount: providerRecords.length,
      availableProviderCount,
      blockedProviderCount,
      cooldownCount: charonWarnings.filter((warning) => warning.state === 'cooldown').length,
      budgetPressureCount: charonWarnings.filter((warning) => warning.level !== 'ok').length,
      toolContextFloor: getNumber(routeGuardrails?.tool_execution_min_context_window, 64000),
      warnings: charonWarnings,
    },
    autonomyGate: {
      decision: getString(liveAutonomyReadiness?.decision, 'unknown'),
      cleanupPacketCount: getNumber(liveAutonomyReadiness?.cleanup_packet_count, 0),
      externalSourceBlockedCount: getNumber(liveAutonomyReadiness?.external_source_blocked_count, 0),
      reasons: autonomyGateReasons.slice(0, 5),
    },
    storageHygiene: {
      status: getString(bundle.storagePressure?.status, 'missing'),
      cleanupCandidateCount: getNumber(storageSummary?.cleanup_candidate_count, 0),
      deletedBytes,
      warnings: deletedBytes > 0
        ? [
            {
              label: 'latest_apply',
              value: `${deletedBytes} bytes`,
              detail: `${getNumber(storageApplySummary?.deleted_total, 0)} deleted`,
            },
            ...storageWarnings,
          ].slice(0, 6)
        : storageWarnings,
    },
    ledgerGaps: bundle.controlLoopTruth.ledgerStates
      .filter((ledger) => ledger.status !== 'ready')
      .map((ledger) => ({
        label: ledger.label,
        path: ledger.path,
        status: ledger.status,
        detail: ledger.detail,
      })),
  }
}