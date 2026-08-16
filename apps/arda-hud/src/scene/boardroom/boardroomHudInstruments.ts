import type { ArdaFreshnessState, ArdaSourceProvenance } from '../../lib/ardaProvenance'
import type { BoardroomSlotAssignmentRecord } from '../../lib/boardroomSlotSettings'

export type HudTone = 'cyan' | 'violet' | 'gold' | 'mint' | 'rose'

export type HudInstrumentStatus = 'nominal' | 'watch' | 'external' | 'offline'

export type HudInstrumentTruthState = 'live' | 'snapshot' | 'projected' | 'stale' | 'unavailable' | 'missing'

export type HudInstrumentNodeState = 'good' | 'warn' | 'alert' | 'dim'

export type HudInstrumentPreset = 'topology' | 'routes' | 'lanes' | 'constellation' | 'pulse' | 'standby'

export interface HudInstrumentNode {
  id: string
  x: number
  y: number
  state: HudInstrumentNodeState
}

export interface HudInstrumentModel {
  title: string
  eyebrow: string
  tone: HudTone
  status: HudInstrumentStatus
  glyph: string
  preset: HudInstrumentPreset
  nodes: HudInstrumentNode[]
  links: Array<[number, number]>
  rings: number[]
  source?: HudInstrumentSource
}

export interface HudInstrumentSource {
  sourceId: string
  sourceLabel: string
  sourceIds?: string[]
  sourcePaths: string[]
  observedAtUtc: string | null
  freshness: ArdaFreshnessState
  sourceKind: ArdaSourceProvenance['sourceKind'] | null
  truthState: HudInstrumentTruthState
}

export interface HudInstrumentTruthPresentation {
  marker: '●' | '□' | '◇' | '!' | '×' | '?'
  label: 'LIVE' | 'SNAPSHOT' | 'PROJECTED' | 'STALE' | 'UNAVAILABLE' | 'MISSING'
}

export function resolveHudInstrumentTruthPresentation(
  truthState: HudInstrumentTruthState,
): HudInstrumentTruthPresentation {
  switch (truthState) {
    case 'live': return { marker: '●', label: 'LIVE' }
    case 'snapshot': return { marker: '□', label: 'SNAPSHOT' }
    case 'projected': return { marker: '◇', label: 'PROJECTED' }
    case 'stale': return { marker: '!', label: 'STALE' }
    case 'unavailable': return { marker: '×', label: 'UNAVAILABLE' }
    case 'missing': return { marker: '?', label: 'MISSING' }
  }
}

export type BoardroomHudInstrumentMap = Record<string, HudInstrumentModel>

export function previewPresetForSource(sourceZoneId?: string): HudInstrumentPreset {
  const source = sourceZoneId?.toLowerCase() ?? ''
  if (source.includes('routing') || source.includes('comms')) return 'routes'
  if (source.includes('planning') || source.includes('queue') || source.includes('operation')) return 'lanes'
  if (source.includes('memory') || source.includes('knowledge') || source.includes('reasoning')) return 'constellation'
  if (source.includes('sovereign') || source.includes('world') || source.includes('human') || source.includes('now_command')) return 'pulse'
  if (source.includes('system') || source.includes('fleet') || source.includes('health')) return 'topology'
  return 'standby'
}

export function previewTitleForSource(sourceZoneId?: string): string | undefined {
  if (!sourceZoneId) return undefined
  const service = sourceZoneId.startsWith('service_')
  const words = sourceZoneId
    .replace(/^service_/, '')
    .split('_')
    .filter((word) => word.length > 0 && word !== 'and')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
  return words.join(service ? ' ' : ' + ')
}

export function resolveBoardroomHudInstrument(
  instruments: BoardroomHudInstrumentMap,
  sceneZoneId: string,
  assignmentSlotId?: string,
): HudInstrumentModel | undefined {
  return instruments[sceneZoneId] ?? (assignmentSlotId ? instruments[assignmentSlotId] : undefined)
}

export interface FleetHudRuntimeDrift {
  driftedNodes: number
  totalNodes: number
}

export interface FleetHudInput {
  liveTargets: number
  totalTargets: number
  routableProviders: number
  unexpectedOffline: number
  intentionalOffline: number
  runtimeDrift?: FleetHudRuntimeDrift | null
  source?: HudInstrumentSource
}

export interface QueueHudInput {
  completed: number
  priorityBuckets: number
  ownerBuckets: number
  source?: HudInstrumentSource
}

export interface KnowledgeHudInput {
  documents: number
  plans: number
  source?: HudInstrumentSource
}

export interface RoutingHudInput {
  routableProviders: number
  activeConnections: number
  constrainedHeadroom: number | null
  source?: HudInstrumentSource
}

export interface GovernanceHudInput {
  reviewItems: number
  pendingItems: number
  incidentItems?: number
  source?: HudInstrumentSource
}

export interface HumanHudInput {
  documents: number
  notes: number
  businessItems?: number
  personalItems?: number
  missingReferences?: number
  source?: HudInstrumentSource
}

export interface DailyCommandHudInput {
  lanes: number
  attentionLanes: number
  source?: HudInstrumentSource
}

export interface BoardroomHudInstrumentInput {
  fleetHealth: FleetHudInput
  queue: QueueHudInput
  knowledge: KnowledgeHudInput
  routing: RoutingHudInput
  governance: GovernanceHudInput
  human: HumanHudInput
  dailyCommand: DailyCommandHudInput
}

const FLEET_NODE_POSITIONS = [
  [50, 13],
  [67, 22],
  [77, 39],
  [70, 60],
  [56, 74],
  [37, 72],
  [22, 58],
  [18, 38],
  [31, 21],
  [50, 43],
  [63, 44],
  [38, 45],
] as const

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value))
}

function buildLinks(count: number): Array<[number, number]> {
  const links: Array<[number, number]> = []
  for (let index = 0; index < count; index += 1) {
    links.push([index, (index + 1) % count])
    if (index % 2 === 0 && index + 3 < count) {
      links.push([index, index + 3])
    }
  }
  return links
}

function radialNodes({
  idPrefix,
  count,
  seed,
  hotCount = 0,
  warnCount = 0,
}: {
  idPrefix: string
  count: number
  seed: number
  hotCount?: number
  warnCount?: number
}): HudInstrumentNode[] {
  const nodeCount = clamp(count, 6, FLEET_NODE_POSITIONS.length)
  return FLEET_NODE_POSITIONS.slice(0, nodeCount).map(([fallbackX, fallbackY], index) => {
    const angle = (Math.PI * 2 * index) / nodeCount + seed * 0.07
    const radius = index % 3 === 0 ? 31 : index % 2 === 0 ? 42 : 52
    let state: HudInstrumentNodeState = 'good'
    if (index < hotCount) {
      state = 'alert'
    } else if (index < hotCount + warnCount) {
      state = 'warn'
    } else if (index > Math.max(3, nodeCount - 3) && seed % 2 === 0) {
      state = 'dim'
    }

    return {
      id: `${idPrefix}-${index + 1}`,
      x: Number((50 + Math.cos(angle) * radius).toFixed(2)) || fallbackX,
      y: Number((50 + Math.sin(angle) * radius * 0.68).toFixed(2)) || fallbackY,
      state,
    }
  })
}

function instrumentStatusFromPressure(pressure: number): HudInstrumentStatus {
  if (pressure <= 0) return 'offline'
  if (pressure >= 0.72) return 'watch'
  return 'nominal'
}

function instrumentStatusFromSource(
  status: HudInstrumentStatus,
  source?: HudInstrumentSource,
): HudInstrumentStatus {
  if (!source) return status
  if (source.truthState === 'missing' || source.truthState === 'unavailable') return 'offline'
  if (source.truthState === 'stale') return 'watch'
  if ((source.truthState === 'snapshot' || source.truthState === 'projected') && status === 'nominal') return 'external'
  return status
}

function commandInstrument({
  title,
  eyebrow,
  tone,
  glyph,
  preset,
  pressure,
  seed,
  hotCount = 0,
  warnCount = 0,
  source,
}: {
  title: string
  eyebrow: string
  tone: HudTone
  glyph: string
  preset: HudInstrumentPreset
  pressure: number
  seed: number
  hotCount?: number
  warnCount?: number
  source?: HudInstrumentSource
}): HudInstrumentModel {
  const nodeCount = clamp(Math.round(6 + pressure * 6), 6, FLEET_NODE_POSITIONS.length)
  const status = instrumentStatusFromSource(instrumentStatusFromPressure(pressure), source)
  return {
    title,
    eyebrow,
    tone: status === 'offline' ? 'rose' : tone,
    status,
    glyph,
    preset,
    nodes: radialNodes({ idPrefix: title.toLowerCase().replace(/[^a-z0-9]+/g, '-'), count: nodeCount, seed, hotCount, warnCount }),
    links: buildLinks(nodeCount),
    rings: status === 'watch' ? [21, 36, 51] : [18, 32, 47],
    source,
  }
}

export function deriveFleetHudInstrument(input: FleetHudInput): HudInstrumentModel {
  const totalTargets = Math.max(0, input.totalTargets)
  const liveTargets = clamp(input.liveTargets, 0, totalTargets || input.liveTargets)
  const unexpectedOffline = Math.max(0, input.unexpectedOffline)
  const intentionalOffline = Math.max(0, input.intentionalOffline)
  const driftedNodes = Math.max(0, input.runtimeDrift?.driftedNodes ?? 0)
  const nodeCount = clamp(Math.max(totalTargets, 6), 6, FLEET_NODE_POSITIONS.length)
  const offlineStart = clamp(liveTargets, 0, nodeCount)
  const intentionalStart = clamp(offlineStart + unexpectedOffline, 0, nodeCount)
  const runtimeStatus: HudInstrumentStatus =
    totalTargets > 0 && liveTargets === 0
      ? 'offline'
      : unexpectedOffline > 0 || driftedNodes > 0
        ? 'watch'
        : 'nominal'
  const status = instrumentStatusFromSource(runtimeStatus, input.source)
  const tone: HudTone = status === 'offline' ? 'rose' : status === 'watch' ? 'gold' : 'cyan'

  const nodes = FLEET_NODE_POSITIONS.slice(0, nodeCount).map(([x, y], index) => {
    let state: HudInstrumentNodeState = index < liveTargets ? 'good' : 'dim'
    if (index >= offlineStart && index < offlineStart + unexpectedOffline) {
      state = 'alert'
    } else if (index >= intentionalStart && index < intentionalStart + intentionalOffline) {
      state = 'warn'
    }
    return { id: `fleet-${index + 1}`, x, y, state }
  })

  return {
    title: 'Fleet',
    eyebrow: 'LIVE SYSTEM MAP',
    tone,
    status,
    glyph: `${liveTargets}/${totalTargets || nodeCount}`,
    preset: 'topology',
    nodes,
    links: buildLinks(nodeCount),
    rings: status === 'nominal' ? [18, 32, 47] : [21, 36, 51],
    source: input.source,
  }
}

export function deriveQueueHudInstrument(input: QueueHudInput): HudInstrumentModel {
  const completed = Math.max(0, input.completed)
  const priorityBuckets = Math.max(0, input.priorityBuckets)
  const ownerBuckets = Math.max(0, input.ownerBuckets)
  const pressure = clamp((priorityBuckets + ownerBuckets) / 12, 0, 1)

  return commandInstrument({
    title: 'Operations',
    eyebrow: 'TASK FLOW',
    tone: 'gold',
    glyph: `${completed}`,
    preset: 'lanes',
    pressure: Math.max(0.2, pressure),
    seed: completed + priorityBuckets * 3 + ownerBuckets * 5,
    warnCount: priorityBuckets > 3 ? 2 : 1,
    source: input.source,
  })
}

export function deriveKnowledgeHudInstrument(input: KnowledgeHudInput): HudInstrumentModel {
  const documents = Math.max(0, input.documents)
  const plans = Math.max(0, input.plans)
  const total = documents + plans
  const pressure = clamp(total / 48, 0, 1)

  return commandInstrument({
    title: 'Knowledge',
    eyebrow: 'PLANS + MEMORY',
    tone: 'mint',
    glyph: `${documents}/${plans}`,
    preset: 'constellation',
    pressure: Math.max(0.24, pressure),
    seed: documents * 2 + plans * 7,
    warnCount: plans > documents ? 1 : 0,
    source: input.source,
  })
}

export function deriveRoutingHudInstrument(input: RoutingHudInput): HudInstrumentModel {
  const routableProviders = Math.max(0, input.routableProviders)
  const activeConnections = Math.max(0, input.activeConnections)
  const constrainedHeadroom = input.constrainedHeadroom === null ? 1 : clamp(input.constrainedHeadroom, 0, 1)
  const pressure = routableProviders === 0 ? 0 : clamp(activeConnections / Math.max(1, routableProviders), 0.18, 1)

  return commandInstrument({
    title: 'Routing',
    eyebrow: 'PROVIDER MESH',
    tone: constrainedHeadroom < 0.25 ? 'gold' : 'cyan',
    glyph: `${routableProviders}`,
    preset: 'routes',
    pressure,
    seed: routableProviders * 11 + activeConnections,
    hotCount: constrainedHeadroom < 0.15 ? 1 : 0,
    warnCount: constrainedHeadroom < 0.35 ? 2 : 0,
    source: input.source,
  })
}

export function deriveGovernanceHudInstrument(input: GovernanceHudInput): HudInstrumentModel {
  const pendingItems = Math.max(0, input.pendingItems)
  const incidentItems = Math.max(0, input.incidentItems ?? 0)
  const pressure = clamp((pendingItems * 2 + incidentItems * 3) / 12, 0, 1)
  return commandInstrument({
    title: 'Governance',
    eyebrow: 'DECISION PRESSURE',
    tone: 'gold',
    glyph: `${pendingItems}/${incidentItems}`,
    preset: 'lanes',
    pressure: Math.max(0.12, pressure),
    seed: pendingItems * 11 + incidentItems * 17,
    warnCount: Math.min(3, pendingItems + incidentItems),
    source: input.source,
  })
}

export function deriveHumanHudInstrument(input: HumanHudInput): HudInstrumentModel {
  const documents = Math.max(0, input.documents)
  const notes = Math.max(0, input.notes)
  const businessItems = Math.max(0, input.businessItems ?? 0)
  const personalItems = Math.max(0, input.personalItems ?? 0)
  const missingReferences = Math.max(0, input.missingReferences ?? 0)
  const total = documents + notes + businessItems + personalItems
  return commandInstrument({
    title: 'Continuity',
    eyebrow: 'HUMAN · BUSINESS · PERSONAL',
    tone: 'mint',
    glyph: `${total}/${missingReferences}`,
    preset: 'pulse',
    pressure: Math.max(0.2, clamp((total + missingReferences * 4) / 48, 0, 1)),
    seed: documents * 3 + notes * 7 + businessItems * 11 + personalItems * 13,
    warnCount: Math.min(3, missingReferences),
    source: input.source,
  })
}

export function deriveDailyCommandHudInstrument(input: DailyCommandHudInput): HudInstrumentModel {
  const lanes = Math.max(0, input.lanes)
  const attentionLanes = Math.max(0, input.attentionLanes)
  return commandInstrument({
    title: 'Daily Command',
    eyebrow: 'OPERATING SURFACE',
    tone: 'violet',
    glyph: `${attentionLanes}/${lanes}`,
    preset: 'pulse',
    pressure: Math.max(0.2, clamp(lanes / 8, 0, 1)),
    seed: lanes * 7 + attentionLanes * 13,
    warnCount: attentionLanes > 0 ? Math.min(2, attentionLanes) : 0,
    source: input.source,
  })
}

function instrumentForWidgetBindings(
  input: BoardroomHudInstrumentInput,
  assignment: BoardroomSlotAssignmentRecord,
): HudInstrumentModel {
  const bindings = assignment.surface_layout.preview.widgets
    .map((widget) => widget.data_binding.toLowerCase())
    .join(' ')
  if (bindings.includes('governance')) return deriveGovernanceHudInstrument(input.governance)
  if (bindings.includes('routing') || bindings.includes('comms') || bindings.includes('manwe')) return deriveRoutingHudInstrument(input.routing)
  if (bindings.includes('human') || bindings.includes('business') || bindings.includes('personal')) return deriveHumanHudInstrument(input.human)
  if (bindings.includes('fleet') || bindings.includes('system') || bindings.includes('backbone')) return deriveFleetHudInstrument(input.fleetHealth)
  if (bindings.includes('queue') || bindings.includes('planning')) return deriveQueueHudInstrument(input.queue)
  if (bindings.includes('knowledge') || bindings.includes('memory')) return deriveKnowledgeHudInstrument(input.knowledge)
  return deriveDailyCommandHudInstrument(input.dailyCommand)
}

export function deriveBoardroomHudInstruments(
  input: BoardroomHudInstrumentInput,
  assignments: BoardroomSlotAssignmentRecord[] = [],
): BoardroomHudInstrumentMap {
  const instruments: BoardroomHudInstrumentMap = {
    monitor_1: deriveFleetHudInstrument(input.fleetHealth),
    monitor_2: deriveRoutingHudInstrument(input.routing),
    monitor_3: deriveKnowledgeHudInstrument(input.knowledge),
    monitor_4: deriveQueueHudInstrument(input.queue),
    monitor_5: deriveHumanHudInstrument(input.human),
    command_core: deriveDailyCommandHudInstrument(input.dailyCommand),
  }
  for (const assignment of assignments) {
    if (!assignment.slot_id.startsWith('view_desk_')) continue
    instruments[assignment.slot_id] = instrumentForWidgetBindings(input, assignment)
  }
  return instruments
}
