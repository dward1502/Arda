export const OPERATOR_PROJECTION_SCHEMA_VERSION = 'arda.operator-projection.v1' as const

export type ProjectionFreshness = 'fresh' | 'stale' | 'unknown'
export type DependencyHealth = 'not_configured' | 'unavailable' | 'degraded' | 'stale' | 'ready' | 'failed'
export type NodeState = 'pending' | 'ready' | 'blocked' | 'running' | 'succeeded' | 'failed' | 'cancelled' | 'superseded'

export interface ObjectiveProjection {
  objective_id: string
  project_id: string | null
  title: string
  status: 'pending' | 'active' | 'blocked' | 'succeeded' | 'failed' | 'cancelled'
}

export interface NodeProjection {
  node_id: string
  kind: 'inspect' | 'retrieve' | 'research' | 'plan' | 'approval' | 'execute' | 'verify' | 'review' | 'compensate' | 'close'
  state: NodeState
}

export interface WorkerProjection {
  node_id: string
  role: 'planner_proposer' | 'implementer' | 'independent_verifier' | 'security_privacy_critic' | 'implementation_risk_critic' | 'local_summary_classification' | 'adjudicator' | 'deterministic_tool' | 'human_approval'
  worker_id: string
  route_id: string
  state: NodeState
}

export interface RunProjection {
  run_id: string
  objective_id: string
  status: 'pending' | 'running' | 'blocked' | 'awaiting_approval' | 'succeeded' | 'failed' | 'cancelled'
  nodes: NodeProjection[]
  workers: WorkerProjection[]
}

export interface CapabilityProjection {
  capability_id: string
  version: string
  health: DependencyHealth
  selected: boolean
  optional: boolean
  selection_reasons: string[]
}

export interface PendingApprovalProjection {
  approval_id: string
  run_id: string
  node_id: string | null
  scope: string
  action_digest: string
  expires_at: string
  status: 'pending' | 'approved' | 'rejected' | 'expired' | 'consumed'
}

export interface CouncilProjection {
  council_id: string
  run_id: string
  state: string
  synthesis: string
  material_tensions: string[]
  non_approval: true
}

export interface ReminderProjection {
  reminder_id: string
  item_id: string
  status: 'pending' | 'delivered' | 'deferred' | 'acknowledged' | 'dismissed' | 'failed'
  next_due_at: string | null
}

export interface PersonalOperationsProjection {
  captures: number
  resumable_items: number
  reminders: ReminderProjection[]
}

export interface JouleWorkProjection {
  budget_joules: number
  consumed_joules: number
  remaining_joules: number
  source: 'observed' | 'estimated' | 'default_fallback' | 'synthetic_restoration' | 'unknown'
  source_confidence: number
}

export interface EvidenceProjection {
  evidence_id: string
  kind: string
  uri: string
  observed_at: string
  freshness: ProjectionFreshness
}

export interface CommunicationProjection {
  communication_id: string
  transport: string
  delivery: 'pending' | 'delivered' | 'failed' | 'unavailable'
  acknowledgement: 'not_required' | 'pending' | 'acknowledged' | 'deferred' | 'rejected'
  updated_at: string
}

export interface DependencyProjection {
  dependency_id: string
  health: DependencyHealth
  freshness: ProjectionFreshness
  detail: string
}

export interface OperatorProjection {
  schema_version: typeof OPERATOR_PROJECTION_SCHEMA_VERSION
  projection_id: string
  generated_at: string
  authority: 'read_only'
  freshness: ProjectionFreshness
  objectives: ObjectiveProjection[]
  runs: RunProjection[]
  capabilities: CapabilityProjection[]
  pending_approvals: PendingApprovalProjection[]
  councils: CouncilProjection[]
  personal_operations: PersonalOperationsProjection
  joulework: JouleWorkProjection
  evidence: EvidenceProjection[]
  communications: CommunicationProjection[]
  dependencies: DependencyProjection[]
}

export interface ProjectionMonitorSignals {
  authority: 'read_only'
  activeObjectives: number
  activeRuns: number
  runningWorkers: number
  pendingApprovals: number
  degradedDependencies: number
  unavailableOptionalCapabilities: number
}

const TOP_LEVEL_FIELDS = new Set<keyof OperatorProjection>([
  'schema_version', 'projection_id', 'generated_at', 'authority', 'freshness',
  'objectives', 'runs', 'capabilities', 'pending_approvals', 'councils',
  'personal_operations', 'joulework', 'evidence', 'communications', 'dependencies',
])
const FRESHNESS = new Set<ProjectionFreshness>(['fresh', 'stale', 'unknown'])
const HEALTH = new Set<DependencyHealth>(['not_configured', 'unavailable', 'degraded', 'stale', 'ready', 'failed'])

function record(value: unknown, lane: string): Record<string, unknown> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`${lane} must be an object`)
  }
  return value as Record<string, unknown>
}

function array(value: unknown, lane: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`${lane} must be an array`)
  return value
}

function text(value: unknown, lane: string): string {
  if (typeof value !== 'string' || value.trim() === '') throw new Error(`${lane} must be non-empty text`)
  return value
}

function finiteNumber(value: unknown, lane: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) {
    throw new Error(`${lane} must be a non-negative finite number`)
  }
  return value
}

function enumValue<T extends string>(value: unknown, allowed: ReadonlySet<T>, lane: string): T {
  if (typeof value !== 'string' || !allowed.has(value as T)) throw new Error(`${lane} has an unsupported value`)
  return value as T
}

function assertIsoDate(value: unknown, lane: string): void {
  const candidate = text(value, lane)
  if (Number.isNaN(Date.parse(candidate))) throw new Error(`${lane} must be an ISO timestamp`)
}

function assertUnique(items: Record<string, unknown>[], field: string, lane: string): void {
  const seen = new Set<string>()
  for (const item of items) {
    const id = text(item[field], `${lane}.${field}`)
    if (seen.has(id)) throw new Error(`duplicate ${lane} identifier: ${id}`)
    seen.add(id)
  }
}

/** Parse untrusted transport data into the canonical, read-only P9.1 contract. */
export function parseOperatorProjection(value: unknown): OperatorProjection {
  const root = record(value, 'operator projection')
  for (const key of Object.keys(root)) {
    if (!TOP_LEVEL_FIELDS.has(key as keyof OperatorProjection)) {
      throw new Error(`unknown operator projection field: ${key}`)
    }
  }
  if (root.schema_version !== OPERATOR_PROJECTION_SCHEMA_VERSION) throw new Error('unsupported operator projection schema version')
  if (root.authority !== 'read_only') throw new Error('operator projection authority must be read_only')
  text(root.projection_id, 'projection_id')
  assertIsoDate(root.generated_at, 'generated_at')
  enumValue(root.freshness, FRESHNESS, 'freshness')

  const objectives = array(root.objectives, 'objectives').map((item) => record(item, 'objective'))
  const runs = array(root.runs, 'runs').map((item) => record(item, 'run'))
  const capabilities = array(root.capabilities, 'capabilities').map((item) => record(item, 'capability'))
  const approvals = array(root.pending_approvals, 'pending_approvals').map((item) => record(item, 'approval'))
  const councils = array(root.councils, 'councils').map((item) => record(item, 'council'))
  const evidence = array(root.evidence, 'evidence').map((item) => record(item, 'evidence'))
  const communications = array(root.communications, 'communications').map((item) => record(item, 'communication'))
  const dependencies = array(root.dependencies, 'dependencies').map((item) => record(item, 'dependency'))
  assertUnique(objectives, 'objective_id', 'objective')
  assertUnique(runs, 'run_id', 'run')
  assertUnique(capabilities, 'capability_id', 'capability')
  assertUnique(approvals, 'approval_id', 'approval')
  assertUnique(councils, 'council_id', 'council')
  assertUnique(evidence, 'evidence_id', 'evidence')
  assertUnique(communications, 'communication_id', 'communication')
  assertUnique(dependencies, 'dependency_id', 'dependency')

  const objectiveIds = new Set(objectives.map((item) => text(item.objective_id, 'objective_id')))
  const runIds = new Set(runs.map((item) => text(item.run_id, 'run_id')))
  for (const run of runs) {
    if (!objectiveIds.has(text(run.objective_id, 'run.objective_id'))) throw new Error('run references missing objective')
    const nodes = array(run.nodes, 'run.nodes').map((item) => record(item, 'node'))
    const nodeIds = new Set(nodes.map((node) => text(node.node_id, 'node.node_id')))
    for (const workerValue of array(run.workers, 'run.workers')) {
      const worker = record(workerValue, 'worker')
      if (!nodeIds.has(text(worker.node_id, 'worker.node_id'))) throw new Error('worker references missing node')
      text(worker.worker_id, 'worker.worker_id')
      text(worker.route_id, 'worker.route_id')
    }
  }
  for (const capability of capabilities) {
    enumValue(capability.health, HEALTH, 'capability.health')
    if (capability.selected === true && array(capability.selection_reasons, 'selection_reasons').length === 0) {
      throw new Error('selected capability must explain selection')
    }
  }
  for (const approval of approvals) {
    if (!runIds.has(text(approval.run_id, 'approval.run_id'))) throw new Error('approval references missing run')
    assertIsoDate(approval.expires_at, 'approval.expires_at')
  }
  for (const council of councils) {
    if (!runIds.has(text(council.run_id, 'council.run_id'))) throw new Error('council references missing run')
    if (council.non_approval !== true) throw new Error('council projection cannot claim approval authority')
  }

  const personal = record(root.personal_operations, 'personal_operations')
  finiteNumber(personal.captures, 'personal_operations.captures')
  finiteNumber(personal.resumable_items, 'personal_operations.resumable_items')
  array(personal.reminders, 'personal_operations.reminders')

  const joulework = record(root.joulework, 'joulework')
  const budget = finiteNumber(joulework.budget_joules, 'joulework.budget_joules')
  const consumed = finiteNumber(joulework.consumed_joules, 'joulework.consumed_joules')
  const remaining = finiteNumber(joulework.remaining_joules, 'joulework.remaining_joules')
  const confidence = finiteNumber(joulework.source_confidence, 'joulework.source_confidence')
  if (confidence > 1) throw new Error('joulework.source_confidence must be at most 1')
  if (Math.max(0, budget - consumed) !== remaining) throw new Error('joulework budget balance is inconsistent')

  for (const dependency of dependencies) {
    const health = enumValue(dependency.health, HEALTH, 'dependency.health')
    const freshness = enumValue(dependency.freshness, FRESHNESS, 'dependency.freshness')
    if (health === 'stale' && freshness === 'fresh') throw new Error('stale dependency cannot be fresh')
  }

  return root as unknown as OperatorProjection
}

export function projectionMonitorSignals(projection: OperatorProjection): ProjectionMonitorSignals {
  return {
    authority: projection.authority,
    activeObjectives: projection.objectives.filter((item) => item.status === 'active').length,
    activeRuns: projection.runs.filter((item) => ['running', 'blocked', 'awaiting_approval'].includes(item.status)).length,
    runningWorkers: projection.runs.flatMap((run) => run.workers).filter((worker) => worker.state === 'running').length,
    pendingApprovals: projection.pending_approvals.filter((item) => item.status === 'pending').length,
    degradedDependencies: projection.dependencies.filter((item) => ['degraded', 'stale', 'failed'].includes(item.health)).length,
    unavailableOptionalCapabilities: projection.capabilities.filter((item) => item.optional && item.health === 'unavailable').length,
  }
}
