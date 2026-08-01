// sigil: REPAIR
import { readFile } from './weathertop'
import { loadArdaHudSettings } from './ardaHudSettings'
import { derivePresenceLedgerProjection } from '../scene/systems/presenceState'
import { classifyFreshness, getOperatorLabel, getSafeRefreshCommand, normalizeTimestamp, type ArdaSourceProvenance } from './ardaProvenance'
import { deriveAutomationStatusSurface } from './automationStatus'
import { parseJsonOrNull } from './jsonParse'
import {
  collectInventoryPaths,
  filenameFromPath,
  ledgerState,
  readInventoryTree,
  readJson,
  readJsonLines,
  readText,
  summarizeReadable,
  titleFromPath,
} from './ardaReaders'

const BUNDLE_READ_LABEL_RE = /\b(?:readJson|readJsonLines|readText)\(([^)]+)\)/i
const bundleMetric = (() => {
  let t0 = 0
  let wrap = 0
  return {
    clear() {
      t0 = 0
      wrap = 0
      ;(globalThis as Record<string, unknown>).__ARDA_BUNDLE_PHASE__ = 'bundle'
    },
    begin() {
      t0 = performance.now()
      wrap = 0
      ;(globalThis as Record<string, unknown>).__ARDA_BUNDLE_PHASE__ = 'bundle'
      console.info(`[bundle] begin #${wrap}`)
    },
    async mark<T>(label: string, next: () => T | Promise<T>): Promise<T> {
      const stamp = performance.now()
      const delta = t0 ? stamp - t0 : 0
      const name = typeof label === 'string' ? label.replace(BUNDLE_READ_LABEL_RE, '$1') : String(label)
      ;(globalThis as Record<string, unknown>).__ARDA_BUNDLE_PHASE__ = name
      console.info(`[bundle] ${name} +${delta.toFixed(1)}ms`)
      t0 = stamp
      const result = await Promise.resolve(next())
      console.info(`[bundle] ${name} done`)
      return result
    },
    increment() {
      wrap += 1
      console.info(`[bundle] retry #${wrap}`)
      return wrap
    },
  }
})()

const timedReadJson = async (rootPath: string, relativePath: string) => {
  const label = `readJson ${relativePath}`
  return bundleMetric.mark(label, () => readJson(rootPath, relativePath))
}

const timedReadJsonLines = async (rootPath: string, relativePath: string) => {
  const label = `readJsonLines ${relativePath}`
  return bundleMetric.mark(label, () => readJsonLines(rootPath, relativePath))
}

const timedReadText = async (rootPath: string, relativePath: string) => {
  const label = `readText ${relativePath}`
  return bundleMetric.mark(label, () => readText(rootPath, relativePath))
}

import type {
  ArdaBundle,
  ArdaControlLoopTruth,
  ArdaDataSource,
  ArdaSceneAnchor,
  ArdaSceneSurface,
  ArdaSceneZone,
  ArdaSection,
  ArdaWorkstationManifest,
  JsonRecord,
} from './ardaBundleTypes'
export type {
  ArdaBundle,
  ArdaControlLoopTruth,
  ArdaDataSource,
  ArdaLedgerState,
  ArdaSceneAnchor,
  ArdaSceneSurface,
  ArdaSceneZone,
  ArdaSection,
  ArdaWorkstationManifest,
  JsonRecord,
} from './ardaBundleTypes'

const DERIVED_SECTION_BLUEPRINTS = [
  {
    id: 'sovereign_world',
    title: 'Sovereign World',
    owner: 'prometheus',
    arda_panels: ['executive_overview', 'systems'],
    primary_sources: ['core/state/world.json', 'core/state/arda_snapshot.json'],
    supplemental_sources: ['core/state/queue_summary.json'],
  },
  {
    id: 'governance_guardhouse',
    title: 'Governance Guardhouse',
    owner: 'oracle',
    arda_panels: ['section_focus', 'governance_controls'],
    primary_sources: ['core/state/active_ruleset.json', 'core/state/permission_profiles.json'],
    supplemental_sources: ['core/state/governance_runtime.json'],
  },
  {
    id: 'routing_and_comms',
    title: 'Routing And Comms',
    owner: 'manwe',
    arda_panels: ['operations_and_packages', 'systems'],
    primary_sources: ['core/state/operator_runtime_status.json', 'core/state/manwe_router.json'],
    supplemental_sources: ['core/state/hermes_command.json'],
  },
  {
    id: 'memory_and_continuity',
    title: 'Memory And Continuity',
    owner: 'mnemosyne',
    arda_panels: ['section_focus', 'human_realm'],
    primary_sources: ['core/state/mnemosyne_continuity.json', 'core/state/memory_scopes.json'],
    supplemental_sources: ['core/state/memory_activity.json', 'core/state/memory_identity.json'],
  },
  {
    id: 'planning_and_queue',
    title: 'Planning And Queue',
    owner: 'hades',
    arda_panels: ['planning', 'section_focus'],
    primary_sources: ['core/state/queue_active.json', 'core/state/queue_summary.json'],
    supplemental_sources: ['core/projects/tasks/queue.jsonl', 'core/state/task_lifecycle_runtime.json', 'core/state/operator_actions.json', 'core/state/l3_readiness_projection.json'],
  },
  {
    id: 'queue_evidence_ledger',
    title: 'Queue Evidence Ledger',
    owner: 'hades',
    arda_panels: ['planning', 'section_focus'],
    primary_sources: ['core/projects/tasks/queue.jsonl'],
    supplemental_sources: ['core/state/task_lifecycle_runtime.json', 'core/state/operator_actions.json', 'core/state/l3_readiness_projection.json'],
  },
  {
    id: 'human_realm',
    title: 'Human Realm',
    owner: 'human_context',
    arda_panels: ['human_realm', 'personal_growth', 'business'],
    primary_sources: ['core/state/human_context.json', 'core/state/business_runtime.json', 'core/state/personal_runtime.json'],
    supplemental_sources: ['docs/operator/index.md', 'docs/operator/company-view.md', 'docs/operator/onboard.md'],
  },
] as const

function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as JsonRecord
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function toStringValue(value: unknown, fallback = ''): string {
  return typeof value === 'string' ? value : fallback
}

function toNumberValue(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function toBooleanValue(value: unknown, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback
}

function toSection(value: unknown): ArdaSection | null {
  const record = asRecord(value)
  if (!record) return null

  return {
    id: toStringValue(record.id, 'unknown'),
    title: toStringValue(record.title, 'Untitled Section'),
    owner: toStringValue(record.owner, 'unknown'),
    status: toStringValue(record.status, 'unknown'),
    arda_panels: asArray(record.arda_panels).map((panel) => toStringValue(panel)).filter(Boolean),
    primary_sources: asArray(record.primary_sources).map((source) => toStringValue(source)).filter(Boolean),
    supplemental_sources: asArray(record.supplemental_sources).map((source) => toStringValue(source)).filter(Boolean),
    missing_projections: asArray(record.missing_projections).map((item) => toStringValue(item)).filter(Boolean),
  }
}

function summarizeFieldCounts(items: JsonRecord[], field: string): JsonRecord {
  return items.reduce<JsonRecord>((counts, item) => {
    const value = toStringValue(item[field], 'unknown')
    counts[value] = ((counts[value] as number | undefined) ?? 0) + 1
    return counts
  }, {})
}

function latestEntriesById(items: JsonRecord[]): JsonRecord[] {
  const byId = new Map<string, JsonRecord>()
  for (const item of items) {
    const id = toStringValue(item.id, '')
    if (!id) continue
    byId.set(id, item)
  }
  return [...byId.values()]
}

function normalizeTaskStatus(status: unknown): string {
  switch (toStringValue(status, 'unknown')) {
    case 'complete':
    case 'done':
      return 'completed'
    case 'active':
    case 'running':
      return 'in_progress'
    default:
      return toStringValue(status, 'unknown')
  }
}

function isOpenTaskStatus(status: unknown): boolean {
  return ['pending', 'queued', 'in_progress'].includes(normalizeTaskStatus(status))
}

function deriveQueueSummaryFromEntries(entries: JsonRecord[]): JsonRecord {
  const tasks = latestEntriesById(entries)
  const openTasks = tasks.filter((task) => isOpenTaskStatus(task.status))
  const recent = tasks.slice(-32)
  return {
    authority: 'arda_derived_queue_summary',
    agent_reading_policy: {
      default_surface: 'core/state/queue_active.json',
      summary_surface: 'core/state/queue_summary.json',
      raw_ledger: 'core/projects/tasks/queue.jsonl',
      raw_ledger_role: 'append_only_evidence_and_mutation_target',
      guidance: 'This is a last-resort latest-by-id fallback. Prefer queue_active.json or queue_summary.json for task selection.',
    },
    project_tasks: {
      counts_by_status: summarizeFieldCounts(tasks, 'status'),
      counts_by_owner: summarizeFieldCounts(tasks, 'owner'),
      counts_by_priority: summarizeFieldCounts(tasks, 'priority'),
      open_total: openTasks.length,
      open_compact_limit: 32,
      open_compact: openTasks.slice(0, 32),
      recent_compact: recent,
    },
    runtime_queue: {
      counts_by_status: {},
      counts_by_owner: {},
      recent: [],
    },
    plans: {
      count: 0,
      paths: [],
    },
    arda_hints: {
      primary_panel: 'task_board',
      boardroom_section: 'execution_queue',
      alert_on_queued_tasks: openTasks.length > 0,
      alert_on_failed_tasks: ((summarizeFieldCounts(tasks, 'result').failed as number | undefined) ?? 0) > 0,
    },
  }
}

function deriveQueueSummaryFromActiveProjection(activeProjection: JsonRecord | null): JsonRecord | null {
  if (!activeProjection) return null

  const tasks = asArray(activeProjection.tasks).map(asRecord).filter((task): task is JsonRecord => task !== null)
  const activeTaskCount = toNumberValue(activeProjection.active_task_count, tasks.length)

  return {
    authority: 'arda_derived_queue_summary_from_active_projection',
    agent_reading_policy: activeProjection.agent_reading_policy ?? {
      default_surface: 'core/state/queue_active.json',
      fallback_surface: 'core/state/queue_summary.json',
      raw_queue_policy: 'Raw queue entries are evidence only and not active backlog.',
    },
    project_tasks: {
      counts_by_status: summarizeFieldCounts(tasks, 'status'),
      counts_by_owner: summarizeFieldCounts(tasks, 'owner'),
      counts_by_priority: summarizeFieldCounts(tasks, 'priority'),
      open_total: activeTaskCount,
      open_compact_limit: 32,
      open_compact: tasks.slice(0, 32),
      recent_compact: tasks.slice(-32),
    },
    runtime_queue: {
      counts_by_status: {},
      counts_by_owner: {},
      recent_compact: [],
    },
    arda_hints: {
      primary_panel: 'task_board',
      boardroom_section: 'execution_queue',
      alert_on_queued_tasks: activeTaskCount > 0,
      alert_on_failed_tasks: false,
    },
  }
}

function deriveGovernanceRuntime(activeRuleset: JsonRecord | null, permissionProfiles: JsonRecord | null): JsonRecord {
  const policy = asRecord(activeRuleset?.policy)
  const validators = asRecord(policy?.validators)
  const coreValidators = asRecord(validators?.core)
  const scopes = asRecord(asRecord(permissionProfiles?.profiles)?.ceo_operator)
  return {
    authority: 'arda_derived_governance_runtime',
    signals: {
      autonomy_observation_score: toNumberValue(policy?.autonomy_score_threshold, 0),
      avg_joulework: toBooleanValue(asRecord(coreValidators?.joulework)?.required, false) ? 1 : 0,
      avg_love_eq: toBooleanValue(asRecord(coreValidators?.love_equation)?.required, false) ? 1 : 0,
      bacon_lite_recent_confidence: Array.isArray(validators?.light) ? 1 : 0,
      triad_pass_rate: toBooleanValue(asRecord(coreValidators?.philosopher_triad)?.required, false) ? 1 : 0,
    },
    derived: {
      autonomy_gap: toBooleanValue(policy?.enable_exec_council, false) ? 0 : 1,
      control_plane_ready: toBooleanValue(asRecord(asRecord(scopes?.scopes)?.network)?.allowed, false),
      autonomy_ready: true,
    },
  }
}

function deriveRuntimeSettings(activeRuleset: JsonRecord | null): JsonRecord {
  const policy = asRecord(activeRuleset?.policy)
  return {
    authority: 'arda_derived_runtime_settings',
    governance: {
      always_on: {
        triad_influence: true,
      },
      weights: {
        autonomy_score_threshold: toNumberValue(policy?.autonomy_score_threshold, 0.65),
        exec_council_enabled: toBooleanValue(policy?.enable_exec_council, true) ? 1 : 0,
        gate_strict: toBooleanValue(policy?.gate_strict, false) ? 1 : 0,
      },
      thresholds: {
        autonomy_score_threshold: toNumberValue(policy?.autonomy_score_threshold, 0.65),
      },
    },
  }
}

function deriveOperationsFlow(queueSummary: JsonRecord | null): JsonRecord {
  const projectTasks = asRecord(queueSummary?.project_tasks)
  const countsByStatus = asRecord(projectTasks?.counts_by_status)
  const totalKnownWorkItems = Object.values(countsByStatus ?? {}).reduce<number>(
    (sum, value) => sum + toNumberValue(value, 0),
    0,
  )
  return {
    authority: 'arda_derived_operations_flow',
    derived: {
      queue_posture: {
        projects_queue_queued: toNumberValue(countsByStatus?.queued, 0),
        total_known_work_items: totalKnownWorkItems,
        pending_escalations: 0,
      },
      control_plane_ready: true,
      autonomy_ready: true,
    },
  }
}

function deriveOperatorRuntimeStatus(world: JsonRecord | null, activeRuleset: JsonRecord | null): JsonRecord {
  const agents = asArray(world?.agents)
    .map((agent) => asRecord(agent))
    .filter((agent): agent is JsonRecord => agent !== null)
  const onlineAgents = agents.filter((agent) => toStringValue(agent.status, '').toUpperCase() === 'ONLINE')
  const offlineAgents = agents.filter((agent) => toStringValue(agent.status, '').toUpperCase() !== 'ONLINE')
  const providerIds = onlineAgents.map((agent) => toStringValue(agent.id, 'unknown')).filter(Boolean)
  const defaultProvider = providerIds[0] ?? 'manwe'
  const laneRoutes = {
    interactive: {
      provider_id: defaultProvider,
      model_id: 'default',
      route_class: 'derived',
      reason: 'Derived from world online roster',
    },
    execution: {
      provider_id: defaultProvider,
      model_id: 'default',
      route_class: 'derived',
      reason: 'Derived from world online roster',
    },
    background: {
      provider_id: defaultProvider,
      model_id: 'default',
      route_class: 'derived',
      reason: 'Derived from world online roster',
    },
  }
  const laneHeadroom = providerIds.reduce<JsonRecord>((acc, providerId) => {
    acc[providerId] = 1
    return acc
  }, {})

  return {
    authority: 'arda_derived_operator_runtime_status',
    summary: {
      fleet_live_llm_nodes_total: onlineAgents.length,
      fleet_routable_local_providers_total: providerIds.length,
      unexpected_offline_total: offlineAgents.length,
    },
    fleet: {
      targets_total: agents.length,
    },
    intentional_offline_targets: [],
    unexpected_offline_targets: offlineAgents.map((agent) => ({
      display_name: toStringValue(agent.name, 'unknown'),
      target_id: toStringValue(agent.id, 'unknown'),
      provider_id: toStringValue(agent.id, 'unknown'),
    })),
    routable_providers: onlineAgents.map((agent) => ({
      provider_id: toStringValue(agent.id, 'unknown'),
      models: [toStringValue(asRecord(activeRuleset?.policy)?.active_ruleset, 'default')],
      avg_latency_ms: null,
      active_connections: toNumberValue(agent.active_tasks, 0),
      soft_caps: {
        interactive: 1,
        execution: 1,
        background: 1,
      },
    })),
    lane_routes: laneRoutes,
    lane_headroom: {
      interactive: laneHeadroom,
      execution: laneHeadroom,
      background: laneHeadroom,
    },
    lane_fitness: {
      interactive: {},
      execution: {},
      background: {},
    },
  }
}

async function deriveHumanContext(rootPath: string): Promise<JsonRecord> {
  const [indexDoc, onboardDoc, companyView, arandurIndex, arandurThoughts, docsTree] = await Promise.all([
    summarizeReadable(rootPath, 'docs/operator/index.md'),
    summarizeReadable(rootPath, 'docs/operator/onboard.md'),
    summarizeReadable(rootPath, 'docs/operator/company-view.md'),
    summarizeReadable(rootPath, 'docs/operator/philosophy.md'),
    summarizeReadable(rootPath, 'docs/operator/philosophy.md'),
    readInventoryTree(rootPath, 'docs/arda', 4),
  ])
  const notesTree = docsTree
  const summariesTree = docsTree
  const libraryTree = docsTree

  const docs = collectInventoryPaths(docsTree, '.md').slice(0, 16).map((path) => ({
    title: titleFromPath(path),
    path,
    body_preview: '',
  }))
  const notes = collectInventoryPaths(notesTree, '.md').slice(0, 16).map((path) => ({
    title: titleFromPath(path),
    path,
    body_preview: '',
  }))
  const summaries = collectInventoryPaths(summariesTree, '.md')
  const library = collectInventoryPaths(libraryTree, '.md').slice(0, 24).map((path) => ({
    title: titleFromPath(path),
    path,
    body_preview: '',
  }))

  return {
    authority: 'arda_derived_human_context',
    human_portal: {
      index: indexDoc,
      onboard: onboardDoc,
      company_view: companyView,
      docs,
      notes,
      summaries: summaries.map((path) => ({ title: titleFromPath(path), path, body_preview: '' })),
      library,
      arandur: {
        index: arandurIndex,
        thoughts: arandurThoughts,
      },
      counts: {
        docs_total: docs.length,
        notes_total: notes.length,
        summaries_total: summaries.length,
        library_docs_total: library.length,
        arandur_docs_total: [arandurIndex, arandurThoughts].filter((entry) => toStringValue(entry.body_preview, '')).length,
      },
    },
  }
}

async function deriveBusinessRuntime(rootPath: string): Promise<JsonRecord> {
  const [companyView, businessState, clientTree] = await Promise.all([
    summarizeReadable(rootPath, 'docs/operator/company-view.md'),
    readJson(rootPath, 'data/business/soterion-business.json'),
    readInventoryTree(rootPath, 'data/business/clients', 5),
  ])
  const clientPaths = collectInventoryPaths(clientTree, '.json')
  const stateKeys = Object.keys(businessState ?? {})
  return {
    authority: 'arda_derived_business_runtime',
    mode: 'derived_from_workspace',
    company_view: companyView,
    state: businessState ?? {},
    counts: {
      client_records_total: clientPaths.length,
      state_keys_total: stateKeys.length,
    },
    highlights: {
      client_paths: clientPaths.slice(0, 4),
      state_keys: stateKeys.slice(0, 6),
    },
  }
}

async function derivePersonalRuntime(rootPath: string): Promise<JsonRecord> {
  const [onboard, personalState, personalTree] = await Promise.all([
    summarizeReadable(rootPath, 'docs/operator/onboard.md'),
    readJson(rootPath, 'data/personal/soterion-personal.json'),
    readInventoryTree(rootPath, 'data/personal', 5),
  ])
  const personalDocs = collectInventoryPaths(personalTree, '.md')
  const stateKeys = Object.keys(personalState ?? {})
  return {
    authority: 'arda_derived_personal_runtime',
    identity: {
      name: 'Daniel',
      role: 'Founder / Principal',
      location: 'Unknown',
    },
    state: personalState ?? {},
    onboard,
    counts: {
      personal_docs_total: personalDocs.length,
      research_domains_total: 0,
      creative_domains_total: 0,
      state_keys_total: stateKeys.length,
    },
    highlights: {
      priorities: [],
      values: stateKeys.slice(0, 6),
      state_keys: stateKeys.slice(0, 6),
    },
    research_domains: [],
    creative_domains: [],
  }
}

function deriveTaskLifecycleRuntime(queueSummary: JsonRecord | null): JsonRecord {
  const projectTasks = asRecord(queueSummary?.project_tasks)
  const countsByStatus = asRecord(projectTasks?.counts_by_status)
  const recent = asArray(projectTasks?.recent)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)

  return {
    authority: 'arda_derived_task_lifecycle_runtime',
    summary: {
      queued_total: toNumberValue(countsByStatus?.queued, 0),
      active_total: toNumberValue(countsByStatus?.in_progress, 0) + toNumberValue(countsByStatus?.active, 0),
      completed_total: toNumberValue(countsByStatus?.completed, 0),
      disposal_review_total: 0,
      archive_ready_total: 0,
    },
    contract: {
      pipeline: 'plan -> task_emission -> task_retrieval -> bounded_execution -> completion_evidence -> hades_disposal_review -> archive_or_retention',
    },
    disposal_candidates: recent
      .filter((item) => toStringValue(item.status, '') === 'completed')
      .slice(-5)
      .map((item) => ({
        id: toStringValue(item.id, 'unknown'),
        title: toStringValue(item.title, 'Untitled task'),
        owner: toStringValue(item.owner, 'unknown'),
        disposal_marker: '↝',
        next_phase: 'hades_disposal_review',
      })),
  }
}

function deriveOperatorActions(queueSummary: JsonRecord | null): JsonRecord {
  const projectTasks = asRecord(queueSummary?.project_tasks)
  const recent = asArray(projectTasks?.recent)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)

  const actions = recent
    .filter((item) => {
      const status = toStringValue(item.status, '')
      return status === 'queued' || status === 'in_progress'
    })
    .slice(-8)
    .reverse()
    .map((item) => ({
      title: toStringValue(item.title, 'Untitled action'),
      owner: toStringValue(item.owner, 'unknown'),
      status: toStringValue(item.status, 'unknown'),
      note: toStringValue(item.notes, 'Derived from task ledger'),
    }))

  return {
    authority: 'arda_derived_operator_actions',
    summary: {
      human_needed_total: 0,
      external_blockers_total: 0,
      auth_required_total: 0,
      configuration_required_total: 0,
    },
    actions,
  }
}

async function derivePlanMap(rootPath: string): Promise<JsonRecord> {
  const [humanPlanTree, corePlanTree] = await Promise.all([
    readInventoryTree(rootPath, 'docs/plans', 4),
    readInventoryTree(rootPath, 'core/projects/Plans', 4),
  ])
  const humanPlans = collectInventoryPaths(humanPlanTree, '.md')
  const corePlans = collectInventoryPaths(corePlanTree, '.md')
  const planIds = [...new Set([...humanPlans, ...corePlans].map((path) => filenameFromPath(path).replace(/\.[^.]+$/, '')))]

  return {
    authority: 'arda_derived_plan_map',
    humanPlanRoot: 'docs/plans',
    corePlanRoot: 'core/projects/Plans',
    plans: planIds.slice(0, 24).map((id) => {
      const humanPlanPath = humanPlans.find((path) => filenameFromPath(path).startsWith(id)) ?? 'docs/plans'
      const coreQuickRefPath = corePlans.find((path) => filenameFromPath(path).startsWith(id)) ?? 'core/projects/Plans'
      return {
        id,
        title: titleFromPath(id),
        owner: 'unknown',
        openTaskCount: 0,
        humanPlanPath,
        coreQuickRefPath,
      }
    }),
  }
}

function deriveSections(sourceMap: JsonRecord | null): ArdaSection[] {
  const explicit = asArray(sourceMap?.sections)
    .map(toSection)
    .filter((section): section is ArdaSection => section !== null)
  if (explicit.length > 0) return explicit

  return DERIVED_SECTION_BLUEPRINTS.map((section) => ({
    id: section.id,
    title: section.title,
    owner: section.owner,
    status: 'derived_runtime',
    arda_panels: [...section.arda_panels],
    primary_sources: [...section.primary_sources],
    supplemental_sources: section.supplemental_sources ? [...section.supplemental_sources] : [],
  }))
}

function deriveSceneZones(sections: ArdaSection[]): ArdaSceneZone[] {
  return sections.map((section) => ({
    id: section.id,
    title: section.title,
    scene: section.id === 'sovereign_world' ? 'boardroom' : 'world',
    owner: section.owner,
    status: section.status,
    anchor_ids: [
      `${section.id}_anchor_primary`,
      `${section.id}_workstation_entry`,
    ],
    surface_ids: [`${section.id}_surface_preview`],
    workstation_ids: [`${section.id}_workstation`],
    source_ids: [...section.primary_sources, ...(section.supplemental_sources ?? [])],
  }))
}

function deriveSceneAnchors(sections: ArdaSection[]): ArdaSceneAnchor[] {
  const baseBoardroomAnchors: ArdaSceneAnchor[] = [
    {
      id: 'world_gate',
      scene: 'boardroom',
      type: 'gate',
      label: 'World Gate',
      zone_id: 'sovereign_world',
      activation_behavior: 'transition_world',
      data_binding: ['core/state/world.json'],
    },
    {
      id: 'hologram_anchor',
      scene: 'boardroom',
      type: 'hologram',
      label: 'Arandur Presence',
      zone_id: 'sovereign_world',
      activation_behavior: 'focus',
      data_binding: ['core/state/human_context.json'],
    },
  ]

  const sectionAnchors = sections.flatMap<ArdaSceneAnchor>((section) => {
    const scene = section.id === 'sovereign_world' ? 'boardroom' : 'world'
    const primaryType: ArdaSceneAnchor['type'] = scene === 'boardroom' ? 'monitor' : 'district'
    const primaryBehavior: ArdaSceneAnchor['activation_behavior'] =
      scene === 'boardroom' ? 'focus' : 'open_terminal'
    return [
      {
        id: `${section.id}_anchor_primary`,
        scene,
        type: primaryType,
        label: section.title,
        zone_id: section.id,
        activation_behavior: primaryBehavior,
        data_binding: [...section.primary_sources],
      },
      {
        id: `${section.id}_workstation_entry`,
        scene,
        type: 'workstation_spawn',
        label: `${section.title} Workstation`,
        zone_id: section.id,
        activation_behavior: 'open_workstation',
        data_binding: [...section.primary_sources, ...(section.supplemental_sources ?? [])],
      },
    ]
  })

  return [...baseBoardroomAnchors, ...sectionAnchors]
}

function deriveSceneSurfaces(sections: ArdaSection[]): ArdaSceneSurface[] {
  return sections.map((section) => ({
    id: `${section.id}_surface_preview`,
    scene: section.id === 'sovereign_world' ? 'boardroom' : 'world',
    kind: section.id === 'sovereign_world' ? 'desk_surface' : 'district_overlay',
    label: `${section.title} Preview`,
    zone_id: section.id,
    module_ids: [...section.arda_panels],
    source_ids: [...section.primary_sources],
  }))
}

function deriveWorkstationManifests(sections: ArdaSection[]): ArdaWorkstationManifest[] {
  return sections.map((section) => ({
    id: `${section.id}_workstation`,
    title: `${section.title} Workstation`,
    source_zone_id: section.id,
    entry_anchor_id: `${section.id}_workstation_entry`,
    module_ids: [...section.arda_panels],
    presentation_modes: ['in_scene', 'native_window'],
  }))
}

function sourceKindForPath(sourcePath: string): ArdaSourceProvenance['sourceKind'] {
  if (sourcePath.startsWith('config/')) return 'config'
  if (sourcePath.startsWith('human/')) return 'manual'
  if (sourcePath.includes('/runtime') || sourcePath.includes('runtime_')) return 'live'
  return 'snapshot'
}

function generatedAtFromContent(sourcePath: string, content: string): string | null {
  if (!sourcePath.endsWith('.json')) return null
  const parsed = asRecord(parseJsonOrNull(content))
  return normalizeTimestamp(
    toStringValue(
      parsed?.generated_at_utc ?? parsed?.generated_at ?? parsed?.timestamp_utc ?? parsed?.timestamp ?? parsed?.updated_at_utc,
      '',
    ),
  )
}

async function deriveProvenanceRecords(rootPath: string, sections: ArdaSection[]): Promise<ArdaSourceProvenance[]> {
  const records = await Promise.all(
    sections.flatMap((section) => {
      const allSourcePaths = [...section.primary_sources, ...(section.supplemental_sources || [])]
      return allSourcePaths.map(async (sourcePath) => {
        const result = await readFile(`${rootPath}/${sourcePath}`)
        const sourceMarkedMissing = section.missing_projections?.includes(sourcePath) ?? false
        const sourceStatus = sourceMarkedMissing || !result.success || !result.content ? 'missing' : 'present'
        const generatedAtUtc = sourceStatus === 'present' ? generatedAtFromContent(sourcePath, result.content ?? '') : null
        const observedAtUtc = sourceStatus === 'present' ? new Date().toISOString() : null
        return {
          domainId: `${section.id}:${sourcePath}`,
          label: `${section.title} / ${getOperatorLabel(sourcePath.replace(/\.[^.]+$/, '').split('/').slice(-1)[0] ?? sourcePath)}`,
          sourcePaths: [sourcePath],
          generatedAtUtc,
          observedAtUtc,
          state: classifyFreshness(generatedAtUtc, observedAtUtc, sourceStatus, 6 * 60 * 60),
          sourceKind: sourceKindForPath(sourcePath),
          derivedFrom: section.primary_sources.includes(sourcePath) ? [] : section.primary_sources,
          safeRefreshCommand: getSafeRefreshCommand(sourcePath),
          notes: sourceMarkedMissing ? 'Source map marks this projection missing' : undefined,
        } satisfies ArdaSourceProvenance
      })
    }),
  )

  return records
}

function deriveSourceMap(rootPath: string, sections: ArdaSection[]): JsonRecord {
  return {
    authority: 'arda_derived_source_map',
    arda_primary_entrypoint_recommended: 'core/state/world.json',
    sections,
    workspace_root: rootPath,
  }
}

function deriveSnapshot(world: JsonRecord | null, sections: ArdaSection[]): JsonRecord {
  return {
    authority: 'arda_derived_snapshot',
    schema_version: 'arda.arda.snapshot.v1',
    generated_at_utc: new Date().toISOString(),
    sections: Object.fromEntries(
      sections.map((section) => [
        section.id,
        {
          schema_version: section.status,
          owner: section.owner,
          title: section.title,
          primary_sources: section.primary_sources,
        },
      ]),
    ),
    world_status: toStringValue(asRecord(world?.system)?.status, 'READY'),
  }
}

function normalizeRemoteConfidenceSnapshot(snapshot: JsonRecord | null): JsonRecord | null {
  const sideEffectPolicy = asRecord(snapshot?.side_effect_policy)
  const ardaHud = asRecord(snapshot?.arda_hud)
  if (
    snapshot?.schema_version !== 'arda.remote_confidence_snapshot.v1' ||
    snapshot?.mode !== 'local_runtime_published' ||
    sideEffectPolicy?.writes_generated_state !== true ||
    sideEffectPolicy?.external_messages_sent !== false ||
    sideEffectPolicy?.service_restart !== false ||
    sideEffectPolicy?.credential_change !== false ||
    ardaHud?.projection_mode !== 'local_runtime_state_file'
  ) {
    return null
  }

  return snapshot
}

function normalizeSafeLocalWorkCyclePreflight(preflight: JsonRecord | null): JsonRecord | null {
  const sideEffectPolicy = asRecord(preflight?.side_effect_policy)
  const ardaHud = asRecord(preflight?.arda_hud)
  if (
    preflight?.schema_version !== 'arda.safe_local_work_cycle_preflight.v1' ||
    preflight?.mode !== 'safe_local_preflight_report' ||
    sideEffectPolicy?.writes_local_report !== true ||
    sideEffectPolicy?.read_only_intake !== true ||
    sideEffectPolicy?.external_messages_sent !== false ||
    sideEffectPolicy?.service_restart !== false ||
    sideEffectPolicy?.credential_change !== false ||
    sideEffectPolicy?.destructive_operations !== false ||
    sideEffectPolicy?.mutates_task_status !== false ||
    sideEffectPolicy?.live_discord_validation !== 'human_gated_separate' ||
    ardaHud?.projection_mode !== 'local_report_file' ||
    ardaHud?.new_rail_required !== false ||
    ardaHud?.forks_autonomy_logic !== false
  ) {
    return null
  }

  return preflight
}

function deriveArandurQueueWriteRequests(requests: JsonRecord[], queueEntries: JsonRecord[]): JsonRecord[] {
  const executedByRequestId = new Map<string, JsonRecord>()
  for (const entry of queueEntries) {
    const requestId = toStringValue(entry.source_queue_write_request_id, '')
    if (requestId) executedByRequestId.set(requestId, entry)
  }

  const latestById = new Map<string, JsonRecord>()
  for (const request of requests) {
    const id = toStringValue(request.queue_write_request_id, '')
    if (!id) continue
    const executedEntry = executedByRequestId.get(id)
    latestById.set(id, {
      ...request,
      execution_status: executedEntry ? 'executed' : (toBooleanValue(request.write_pending, false) ? 'write_pending' : 'legacy_review'),
      canonical_queue_task_id: executedEntry ? toStringValue(executedEntry.id, '') : null,
    })
  }

  return [...latestById.values()].sort((left, right) => {
    const leftExecuted = toStringValue(left.execution_status, '') === 'executed'
    const rightExecuted = toStringValue(right.execution_status, '') === 'executed'
    if (leftExecuted !== rightExecuted) return leftExecuted ? 1 : -1
    return toStringValue(right.created_at_utc, '').localeCompare(toStringValue(left.created_at_utc, ''))
  })
}

function countByStatus(rows: JsonRecord[], status: string): number {
  return rows.filter((row) => toStringValue(row.status) === status).length
}

function deriveAuditReadiness(phase6Rows: JsonRecord[], phase8Rows: JsonRecord[], generatedAtUtc: string): JsonRecord {
  const verifiedPhase7Rows = phase6Rows.filter((row) => toStringValue(row.execution_status).startsWith('resolved_phase7_'))
  const unresolvedPhase7Rows = phase6Rows.filter((row) => !toStringValue(row.execution_status).startsWith('resolved_phase7_'))
  const phase8OpenRows = phase8Rows
    .filter((row) => toStringValue(row.status) !== 'completed')
    .slice(0, 4)
    .map((row) => ({
      id: toStringValue(row.id, 'unknown'),
      title: toStringValue(row.title, 'Untitled hardening item'),
      status: toStringValue(row.status, 'unknown'),
      scope: toStringValue(row.scope, 'scope unavailable'),
    }))

  return {
    authority: 'arda_audit_readiness_projection',
    schema_version: 'arda.arda.audit_readiness.v1',
    generated_at_utc: generatedAtUtc,
    evidence_sources: [
      'audit/PROFESSIONALIZATION_AUDIT_2026-05-25/phase6-execution-matrix.jsonl',
      'audit/PROFESSIONALIZATION_AUDIT_2026-05-25/phase8-hardening-backlog.jsonl',
    ],
    phase7_closeout: {
      status: unresolvedPhase7Rows.length === 0 && phase6Rows.length > 0 ? 'closed' : 'needs_review',
      verified_slices: verifiedPhase7Rows.length,
      total_slices: phase6Rows.length,
      unresolved_slices: unresolvedPhase7Rows.length,
    },
    phase8_hardening: {
      total: phase8Rows.length,
      completed: countByStatus(phase8Rows, 'completed'),
      in_progress: countByStatus(phase8Rows, 'in_progress'),
      queued: countByStatus(phase8Rows, 'queued'),
    },
    next_items: phase8OpenRows,
    boundary: {
      summary: 'Audit evidence only; this is not live runtime/service status.',
      closed_remediation: 'Phase 7 remediation is represented from verified audit ledgers, not chat memory.',
      roadmap: 'Phase 8 hardening and future embodied roadmap work remain separate from closed Phase 7 remediation.',
    },
  }
}

export function createCoreStateSource(): ArdaDataSource {
  return {
    name: 'CoreStateSource',
    async loadBundle(): Promise<ArdaBundle> {
      bundleMetric.clear()
      bundleMetric.begin()
      const asyncBundle = (attempt = 1) => this._loadBundleCore(attempt)
      return bundleMetric.mark('loadArdaHudSettings', async () => {
        const { rootPath, settings } = await loadArdaHudSettings()
        const bundle = await asyncBundle(1)
        return bundle
      })
    },
    async _loadBundleCore(attempt: number): Promise<ArdaBundle> {
      const { rootPath, settings } = await bundleMetric.mark('loadArdaHudSettings', () => loadArdaHudSettings())
      const [
        snapshot,
        remoteConfidenceSnapshot,
        world,
        humanContext,
        businessRuntime,
        personalRuntime,
        runtimeSettings,
        configWalkthroughProfiles,
        governanceRuntime,
        operationsFlow,
        soterionRenderContract,
        paperclipAlignment,
        escalationRuntime,
        operatorActions,
        outputTopology,
        outputAccounting,
        packageHealth,
        packageEnablement,
        packageRuntimeActivation,
        storagePressure,
        storageHygieneApply,
        queueActiveProjection,
        queueSummary,
        queueFederation,
        fleetRuntimeDrift,
        taskLifecycleRuntime,
        operatorRuntimeStatus,
        humanAugmentationRuntime,
        ceoCouncilRuntime,
        ceoAutopilotState,
        autonomyReadiness,
        autonomyDailyEval,
        autonomyHoldReason,
        setupConsoleReadiness,
        onboardingGuidedSession,
        onboardingPrivateConfigStage,
        onboardingServicePlan,
        hadesNightlyOperations,
        chronosRuntime,
        wardenGuardhouse,
        chronosStatus,
        chronosAuditTasks,
        providerIntelligence,
        professionalizationPhase6Rows,
        professionalizationPhase8Rows,
        sourceMap,
        planMap,
        providerTokenUsage,
        manweRouter,
        athenaRuntime,
        athenaDigest,
        athenaDeepGraph,
        athenaPolicyReadiness,
        activeRuleset,
        permissionProfiles,
        queueEntries,
        derivedPlanMap,
        derivedHumanContext,
        derivedBusinessRuntime,
        derivedPersonalRuntime,
        knowledgeTriage,
        arandurQueueWriteRequests,
        arandurRecommendations,
        arandurMissionApprovalRequests,
        hadesLifecycleReviewQueue,
        l3ReadinessProjection,
        flywheelPacketRuntime,
        hermesMessages,
        flywheelDispatchReceipts,
        hermesAgentGatewayReceipts,
        agentConversations,
        scoutRequests,
        scoutFindings,
        scoutRuntime,
        presenceEventLedgerText,
      ] = await Promise.all([
        readJson(rootPath, settings.arda_snapshot_path),
        readJson(rootPath, settings.remote_confidence_snapshot_path),
        readJson(rootPath, settings.world_path),
        readJson(rootPath, settings.human_context_path),
        readJson(rootPath, settings.business_runtime_path),
        readJson(rootPath, settings.personal_runtime_path),
        readJson(rootPath, settings.runtime_settings_path),
        readJson(rootPath, settings.config_walkthrough_profiles_path),
        readJson(rootPath, settings.governance_runtime_path),
        readJson(rootPath, settings.operations_flow_path),
        readJson(rootPath, settings.soterion_render_contract_path),
        readJson(rootPath, settings.paperclip_alignment_path),
        readJson(rootPath, settings.escalation_runtime_path),
        readJson(rootPath, settings.operator_actions_path),
        readJson(rootPath, settings.output_topology_path),
        readJson(rootPath, settings.output_accounting_path),
        readJson(rootPath, settings.package_health_path),
        readJson(rootPath, settings.package_enablement_path),
        readJson(rootPath, settings.package_runtime_activation_path),
        readJson(rootPath, settings.storage_pressure_path),
        readJson(rootPath, 'core/state/storage_hygiene_apply.json'),
        readJson(rootPath, settings.queue_active_path),
        readJson(rootPath, settings.queue_summary_path),
        readJson(rootPath, 'core/state/queue_federation.json'),
        readJson(rootPath, settings.fleet_runtime_drift_path),
        readJson(rootPath, settings.task_lifecycle_runtime_path),
        readJson(rootPath, settings.operator_runtime_status_path),
        readJson(rootPath, settings.human_augmentation_runtime_path),
        readJson(rootPath, settings.ceo_council_runtime_path),
        readJson(rootPath, 'data/ceo/autopilot.state.json'),
        readJson(rootPath, settings.autonomy_readiness_path),
        readJson(rootPath, settings.autonomy_daily_eval_path),
        readJson(rootPath, settings.autonomy_hold_reason_path),
        readJson(rootPath, 'core/state/setup_console_readiness.json'),
        readJson(rootPath, 'audit/onboarding-runs/latest-guided-session.json'),
        readJson(rootPath, 'audit/onboarding-runs/latest-private-config-stage.json'),
        readJson(rootPath, 'audit/onboarding-runs/latest-service-plan.json'),
        readJson(rootPath, 'core/state/hades_nightly_operations.json'),
        readJson(rootPath, 'core/state/chronos_runtime.json'),
        readJson(rootPath, 'core/state/warden_guardhouse.json'),
        readJson(rootPath, 'core/state/chronos_status.json'),
        readJson(rootPath, 'config/chronos_audit_tasks.json'),
        readJson(rootPath, 'core/state/provider_intelligence.json'),
        readJsonLines(rootPath, settings.professionalization_phase6_matrix_path),
        readJsonLines(rootPath, settings.professionalization_phase8_backlog_path),
        readJson(rootPath, settings.arda_source_map_path),
        readJson(rootPath, settings.core_plan_index_path),
        readJson(rootPath, 'core/state/provider_token_usage.json'),
        readJson(rootPath, settings.manwe_router_path),
        readJson(rootPath, settings.athena_runtime_path),
        readJsonLines(rootPath, settings.athena_digest_path),
        readJsonLines(rootPath, settings.athena_deep_graph_path),
        readJsonLines(rootPath, settings.athena_policy_readiness_path),
        readJson(rootPath, 'core/state/active_ruleset.json'),
        readJson(rootPath, 'core/state/permission_profiles.json'),
        readJsonLines(rootPath, settings.task_queue_path),
        derivePlanMap(rootPath),
        deriveHumanContext(rootPath),
        deriveBusinessRuntime(rootPath),
        derivePersonalRuntime(rootPath),
        readJsonLines(rootPath, 'core/state/knowledge_triage_registry.jsonl'),
        readJsonLines(rootPath, 'data/arandur/mission_queue_write_requests.jsonl'),
        readJsonLines(rootPath, 'data/arandur/recommendations.jsonl'),
        readJsonLines(rootPath, 'data/arandur/mission_approval_requests.jsonl'),
        readJsonLines(rootPath, 'data/hades/lifecycle_review_queue.jsonl'),
        readJson(rootPath, 'core/state/l3_readiness_projection.json'),
        readJson(rootPath, 'core/state/flywheel_packet_runtime.json'),
        readJsonLines(rootPath, 'data/hermes/messages.jsonl'),
        readJsonLines(rootPath, 'data/hermes/flywheel_dispatch_receipts.jsonl'),
        readJsonLines(rootPath, 'data/hermes/hermes_agent_gateway_receipts.jsonl'),
        readJsonLines(rootPath, 'data/council/agent_conversations.jsonl'),
        readJsonLines(rootPath, 'data/athena/scout_requests.jsonl'),
        readJsonLines(rootPath, 'data/athena/scout_findings.jsonl'),
        readJson(rootPath, 'core/state/scout_runtime.json'),
        readText(rootPath, 'data/prometheus/arda_presence_events.jsonl'),
      ])
      const safeLocalWorkCyclePreflight = await readJson(rootPath, 'data/prometheus/safe_local_work_cycle_preflight.json')
      const finalHumanContext = humanContext ?? derivedHumanContext
      const finalBusinessRuntime = businessRuntime ?? derivedBusinessRuntime
      const finalPersonalRuntime = personalRuntime ?? derivedPersonalRuntime
      const finalQueueSummary = queueSummary ?? deriveQueueSummaryFromActiveProjection(queueActiveProjection) ?? deriveQueueSummaryFromEntries(queueEntries)
      const finalRuntimeSettings = runtimeSettings ?? deriveRuntimeSettings(activeRuleset)
      const finalGovernanceRuntime = governanceRuntime ?? deriveGovernanceRuntime(activeRuleset, permissionProfiles)
      const finalOperationsFlow = operationsFlow ?? deriveOperationsFlow(finalQueueSummary)
      const finalStoragePressure = storagePressure
        ? { ...storagePressure, latest_apply: storageHygieneApply }
        : storageHygieneApply
          ? { status: 'apply_receipt_only', latest_apply: storageHygieneApply }
          : null
      const finalOperatorActions = operatorActions ?? deriveOperatorActions(finalQueueSummary)
      const finalOperatorRuntimeStatus = operatorRuntimeStatus ?? deriveOperatorRuntimeStatus(world, activeRuleset)
      const finalTaskLifecycleRuntime = taskLifecycleRuntime ?? deriveTaskLifecycleRuntime(finalQueueSummary)
      const finalHumanAugmentationRuntime = {
        ...(humanAugmentationRuntime ?? {}),
        arandur_queue_write_requests: deriveArandurQueueWriteRequests(arandurQueueWriteRequests, queueEntries),
      }
      const sections = deriveSections(sourceMap)
      const sceneZones = deriveSceneZones(sections)
      const sceneAnchors = deriveSceneAnchors(sections)
      const sceneSurfaces = deriveSceneSurfaces(sections)
      const workstationManifests = deriveWorkstationManifests(sections)
      const finalSourceMap = sourceMap ?? deriveSourceMap(rootPath, sections)
      const finalSnapshot = snapshot ?? deriveSnapshot(world, sections)
      const finalRemoteConfidenceSnapshot = normalizeRemoteConfidenceSnapshot(remoteConfidenceSnapshot)
      const finalSafeLocalWorkCyclePreflight = normalizeSafeLocalWorkCyclePreflight(safeLocalWorkCyclePreflight)
      const sourceProvenance = await deriveProvenanceRecords(rootPath, sections)
      const ledgerStates = await Promise.all([
        ledgerState(rootPath, settings.task_queue_path, 'Project task queue'),
        ledgerState(rootPath, 'data/hermes/hermes_agent_gateway_receipts.jsonl', 'Hermes gateway receipts'),
        ledgerState(rootPath, 'data/hermes/flywheel_dispatch_receipts.jsonl', 'Flywheel dispatch receipts'),
        ledgerState(rootPath, 'data/chronos/audit_receipts.jsonl', 'Chronos audit receipts'),
        ledgerState(rootPath, settings.athena_policy_readiness_path, 'Athena policy readiness'),
        ledgerState(rootPath, 'core/state/warden_guardhouse.json', 'Warden guardhouse projection'),
        ledgerState(rootPath, 'core/state/chronos_status.json', 'Chronos status projection'),
      ])
      const finalPlanMap = planMap ?? derivedPlanMap
      const automationStatus = deriveAutomationStatusSurface(autonomyDailyEval, autonomyHoldReason)
      const generatedAt = new Date().toISOString()
      const auditReadiness = deriveAuditReadiness(professionalizationPhase6Rows, professionalizationPhase8Rows, generatedAt)
      const presenceLedgerProjection = derivePresenceLedgerProjection(presenceEventLedgerText, generatedAt)
      const controlLoopTruth: ArdaControlLoopTruth = {
        wardenGuardhouse,
        chronosStatus,
        chronosAuditTasks,
        ledgerStates,
      }

      return {
        rootPath,
        generatedAt,
        settings: asRecord(settings),
        snapshot: finalSnapshot,
        remoteConfidenceSnapshot: finalRemoteConfidenceSnapshot,
        safeLocalWorkCyclePreflight: finalSafeLocalWorkCyclePreflight,
        l3ReadinessProjection,
        flywheelPacketRuntime,
        hermesMessages,
        flywheelDispatchReceipts,
        hermesAgentGatewayReceipts,
        agentConversations,
        scoutRequests,
        scoutFindings,
        scoutRuntime,
        world,
        humanContext: finalHumanContext,
        businessRuntime: finalBusinessRuntime,
        personalRuntime: finalPersonalRuntime,
        runtimeSettings: finalRuntimeSettings,
        configWalkthroughProfiles,
        governanceRuntime: finalGovernanceRuntime,
        operationsFlow: finalOperationsFlow,
        soterionRenderContract,
        paperclipAlignment,
        escalationRuntime,
        operatorActions: finalOperatorActions,
        outputTopology,
        outputAccounting,
        packageHealth,
        packageEnablement,
        packageRuntimeActivation,
        storagePressure: finalStoragePressure,
        queueSummary: finalQueueSummary,
        queueFederation,
        fleetRuntimeDrift,
        taskLifecycleRuntime: finalTaskLifecycleRuntime,
        operatorRuntimeStatus: finalOperatorRuntimeStatus,
        humanAugmentationRuntime: finalHumanAugmentationRuntime,
        ceoCouncilRuntime,
        ceoAutopilotState,
        autonomyReadiness,
        automationStatus,
        setupConsoleReadiness,
        onboardingGuidedSession,
        onboardingPrivateConfigStage,
        onboardingServicePlan,
        auditReadiness,
        hadesNightlyOperations,
        chronosRuntime,
        controlLoopTruth,
        providerIntelligence,
        sourceMap: finalSourceMap,
        planMap: finalPlanMap,
        providerTokenUsage,
        manweRouter,
        athenaRuntime,
        taskQueueEntries: queueEntries,
        athenaDigest,
        athenaDeepGraph,
        athenaPolicyReadiness,
        knowledgeTriage,
        arandurRecommendations,
        arandurMissionApprovalRequests,
        hadesLifecycleReviewQueue,
        agentPresenceState: presenceLedgerProjection.state,
        agentPresenceStatus: presenceLedgerProjection.status,
        sections,
        sceneZones,
        sceneAnchors,
        sceneSurfaces,
        workstationManifests,
        sourceProvenance,
      }
    },
  }
}
