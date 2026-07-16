export type HudTone = 'cyan' | 'violet' | 'gold' | 'mint' | 'rose'

export type HudInstrumentStatus = 'nominal' | 'watch' | 'external' | 'offline'

export type HudInstrumentNodeState = 'good' | 'warn' | 'alert' | 'dim'

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
  nodes: HudInstrumentNode[]
  links: Array<[number, number]>
  rings: number[]
}

export type BoardroomHudInstrumentMap = Record<string, HudInstrumentModel>

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
}

export interface QueueHudInput {
  completed: number
  priorityBuckets: number
  ownerBuckets: number
}

export interface KnowledgeHudInput {
  documents: number
  plans: number
}

export interface RoutingHudInput {
  routableProviders: number
  activeConnections: number
  constrainedHeadroom: number | null
}

export interface BoardroomHudInstrumentInput {
  fleetHealth: FleetHudInput
  queue: QueueHudInput
  knowledge: KnowledgeHudInput
  routing: RoutingHudInput
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

function commandInstrument({
  title,
  eyebrow,
  tone,
  glyph,
  pressure,
  seed,
  hotCount = 0,
  warnCount = 0,
}: {
  title: string
  eyebrow: string
  tone: HudTone
  glyph: string
  pressure: number
  seed: number
  hotCount?: number
  warnCount?: number
}): HudInstrumentModel {
  const nodeCount = clamp(Math.round(6 + pressure * 6), 6, FLEET_NODE_POSITIONS.length)
  const status = instrumentStatusFromPressure(pressure)
  return {
    title,
    eyebrow,
    tone: status === 'offline' ? 'rose' : tone,
    status,
    glyph,
    nodes: radialNodes({ idPrefix: title.toLowerCase().replace(/[^a-z0-9]+/g, '-'), count: nodeCount, seed, hotCount, warnCount }),
    links: buildLinks(nodeCount),
    rings: status === 'watch' ? [21, 36, 51] : [18, 32, 47],
  }
}

export function deriveFleetHudInstrument(input: FleetHudInput): HudInstrumentModel {
  const totalTargets = Math.max(0, input.totalTargets)
  const liveTargets = clamp(input.liveTargets, 0, totalTargets || input.liveTargets)
  const unexpectedOffline = Math.max(0, input.unexpectedOffline)
  const intentionalOffline = Math.max(0, input.intentionalOffline)
  const routableProviders = Math.max(0, input.routableProviders)
  const driftedNodes = Math.max(0, input.runtimeDrift?.driftedNodes ?? 0)
  const nodeCount = clamp(Math.max(totalTargets, routableProviders, 6), 6, FLEET_NODE_POSITIONS.length)
  const offlineStart = clamp(liveTargets, 0, nodeCount)
  const intentionalStart = clamp(offlineStart + unexpectedOffline, 0, nodeCount)
  const status: HudInstrumentStatus =
    totalTargets > 0 && liveTargets === 0
      ? 'offline'
      : unexpectedOffline > 0 || driftedNodes > 0
        ? 'watch'
        : 'nominal'
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
    nodes,
    links: buildLinks(nodeCount),
    rings: status === 'nominal' ? [18, 32, 47] : [21, 36, 51],
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
    pressure: Math.max(0.2, pressure),
    seed: completed + priorityBuckets * 3 + ownerBuckets * 5,
    warnCount: priorityBuckets > 3 ? 2 : 1,
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
    pressure: Math.max(0.24, pressure),
    seed: documents * 2 + plans * 7,
    warnCount: plans > documents ? 1 : 0,
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
    pressure,
    seed: routableProviders * 11 + activeConnections,
    hotCount: constrainedHeadroom < 0.15 ? 1 : 0,
    warnCount: constrainedHeadroom < 0.35 ? 2 : 0,
  })
}

export function deriveBoardroomHudInstruments(input: BoardroomHudInstrumentInput): BoardroomHudInstrumentMap {
  return {
    'boardroom.lower.left_wrap': deriveFleetHudInstrument(input.fleetHealth),
    view_desk_l: deriveQueueHudInstrument(input.queue),
    view_desk_r: deriveKnowledgeHudInstrument(input.knowledge),
    monitor_left_2: deriveRoutingHudInstrument(input.routing),
  }
}
