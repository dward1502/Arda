import { safeTauriInvoke } from './tauriGuard'

export interface ProjectValidation {
  valid: boolean
  projectId: string | null
  root: string | null
  effectivePermissions: string[]
  providerPosture: string | null
  projectChecks: string[]
  errors: string[]
}

export interface TaskApproval {
  schema_version: 'arda.orome.task_approval.v1'
  proposal_id: string
  approval_id: string
  ledger_writes: string[]
  decision: 'policy_safe' | 'policy_blocked'
  created_at_utc: string
}

export interface MutationEnvelope {
  approval: TaskApproval
  idempotency_key: string
}

export interface AttachedProject {
  contract: Record<string, unknown>
  approval_id: string
  proposal_id: string
  idempotency_key: string
}

export type RunNodeState = 'pending' | 'ready' | 'blocked' | 'running' | 'succeeded' | 'failed' | 'cancelled' | 'superseded'
export type RunNodeKind = 'inspect' | 'retrieve' | 'research' | 'plan' | 'approval' | 'execute' | 'verify' | 'review' | 'compensate' | 'close'

export interface RunNode {
  id: string
  kind: RunNodeKind
  state: RunNodeState
  authority: 'read_only' | 'human_approval' | 'execute_with_approval' | 'verify' | 'compensate_with_approval'
  budget: { max_joules: number; max_cost_usd: number }
  retry: { max_attempts: number }
  timeout_ms: number
  idempotency_key: string
  input_digest: string | null
  output_digest: string | null
  parent_receipts: string[]
  checkpoint: { sequence: number; recovery_token: string | null; checkpoint_digest: string | null }
}

export interface RunGraph {
  schema_version: 'arda.run-graph.v1'
  run_id: string
  objective_id: string
  nodes: RunNode[]
  edges: Array<{ id: string; from: string; to: string; parent_receipt: string | null }>
  provenance: { project_contract_digest: string; created_by: string; parent_receipts: string[] }
}

export interface RunRecord {
  graph: RunGraph
  events: WorkbenchEvent[]
  review: RunReviewEvidence
  recovery_diagnostics?: RecoveryDiagnostics | null
}

export interface RecoveryDiagnostics {
  failure_owner: string
  failed_node_id: string
  failure_reason: string
  last_valid_state: { node_id: string; state: RunNodeState; receipt_digest: string | null } | null
  safe_recovery_action: string
  post_recovery_receipt: string | null
}

export interface ExecuteProviderNodeResponse {
  run: RunRecord
  receipt: Record<string, unknown>
}

export interface RunReviewEvidence {
  changes: ChangeRecord[]
  tests: TestRecord[]
  provider_receipt: ProviderReceiptRecord | null
}

export interface WorkbenchObjective {
  schemaVersion: 'arda.workbench.objective.v1'
  objectiveId: string
  text: string
  inputMode: 'text' | 'voice'
}

export interface WorkbenchEvent {
  schema_version?: string
  sequence?: number
  run_id?: string
  node_id?: string | null
  occurred_at_unix_ms?: number
  recorded_at_unix_ms?: number
  receipt_digest?: string | null
  idempotency_key?: string
  kind?: { type?: string; state?: RunNodeState; reason?: string; project_id?: string; approval_id?: string }
  event?: { type?: string; state?: RunNodeState; reason?: string; project_id?: string; approval_id?: string }
  type?: string
  state?: RunNodeState
  reason?: string
}

export interface ChangeRecord {
  path: string
  status: 'added' | 'modified' | 'deleted'
  additions: number
  deletions: number
  diff?: string
}

export interface TestRecord {
  name: string
  status: 'passed' | 'failed' | 'running' | 'not_run'
  durationMs?: number
  duration_ms?: number
  details?: string
}

export interface ProviderReceiptRecord {
  provider: string
  model: string
  adapter: string
  receipt_digest: string
  summary: string
}

export function createObjective(text: string, inputMode: WorkbenchObjective['inputMode'] = 'text'): WorkbenchObjective {
  const trimmed = text.trim()
  if (!trimmed) throw new Error('Objective text is required')
  const suffix = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}`
  return {
    schemaVersion: 'arda.workbench.objective.v1',
    objectiveId: `objective-${suffix}`,
    text: trimmed,
    inputMode,
  }
}

export function buildRunGraph(objective: WorkbenchObjective, projectId: string): RunGraph {
  const runId = `run-${objective.objectiveId.replace(/^objective-/, '')}`
  const checkpoint = { sequence: 0, recovery_token: null, checkpoint_digest: null }
  const node = (
    id: string,
    kind: RunNodeKind,
    authority: RunNode['authority'],
    cost: number,
    receipts: string[] = [],
  ): RunNode => ({
    id,
    kind,
    state: 'pending',
    authority,
    budget: { max_joules: kind === 'execute' ? 250 : 25, max_cost_usd: cost },
    retry: { max_attempts: 1 },
    timeout_ms: kind === 'execute' ? 900_000 : 60_000,
    idempotency_key: `${runId}-${id}`,
    input_digest: `objective:${objective.objectiveId}`,
    output_digest: null,
    parent_receipts: receipts,
    checkpoint: { ...checkpoint },
  })
  return {
    schema_version: 'arda.run-graph.v1',
    run_id: runId,
    objective_id: objective.objectiveId,
    nodes: [
      node('plan', 'plan', 'read_only', 0),
      node('approval', 'approval', 'human_approval', 0, ['receipt:plan']),
      node('execute', 'execute', 'execute_with_approval', 2, ['receipt:approval']),
      node('verify', 'verify', 'verify', 0, ['receipt:execute']),
      node('review', 'review', 'verify', 0, ['receipt:verify']),
      node('close', 'close', 'read_only', 0, ['receipt:review']),
    ],
    edges: [
      { id: 'plan-to-approval', from: 'plan', to: 'approval', parent_receipt: 'receipt:plan' },
      { id: 'approval-to-execute', from: 'approval', to: 'execute', parent_receipt: 'receipt:approval' },
      { id: 'execute-to-verify', from: 'execute', to: 'verify', parent_receipt: 'receipt:execute' },
      { id: 'verify-to-review', from: 'verify', to: 'review', parent_receipt: 'receipt:verify' },
      { id: 'review-to-close', from: 'review', to: 'close', parent_receipt: 'receipt:review' },
    ],
    provenance: {
      project_contract_digest: `project:${projectId}`,
      created_by: 'arda-hud-workbench',
      parent_receipts: [],
    },
  }
}

export const validateProjectContract = (path: string) =>
  safeTauriInvoke<ProjectValidation>('validate_project_contract', { path })

export const attachProjectContract = (path: string, envelope: MutationEnvelope) =>
  safeTauriInvoke<AttachedProject>('attach_project_contract', { path, envelope })

export const planWorkbenchRun = (projectId: string, graph: RunGraph, envelope: MutationEnvelope) =>
  safeTauriInvoke<RunRecord>('plan_workbench_run', { request: { project_id: projectId, graph, envelope } })

export const approveWorkbenchRun = (runId: string, nodeId: string, envelope: MutationEnvelope) =>
  safeTauriInvoke<RunRecord>('approve_workbench_run', { request: { run_id: runId, node_id: nodeId, envelope } })

export const completeWorkbenchRunNode = (runId: string, nodeId: string, receiptDigest: string, envelope: MutationEnvelope, evidence?: RunReviewEvidence) =>
  safeTauriInvoke<RunRecord>('complete_workbench_run_node', { request: { run_id: runId, node_id: nodeId, receipt_digest: receiptDigest, envelope, evidence } })

export const executeWorkbenchProviderNode = (runId: string, nodeId: string, objective: string, envelope: MutationEnvelope) =>
  safeTauriInvoke<ExecuteProviderNodeResponse>('execute_workbench_provider_node', { request: { run_id: runId, node_id: nodeId, objective, envelope } })

export const cancelWorkbenchRun = (runId: string, reason: string, envelope: MutationEnvelope) =>
  safeTauriInvoke<RunRecord>('cancel_workbench_run', { request: { run_id: runId, reason, envelope } })

export const getWorkbenchRun = (runId: string) =>
  safeTauriInvoke<RunRecord>('get_workbench_run', { runId })

export const getWorkbenchRunEvents = (runId: string) =>
  safeTauriInvoke<{ events: WorkbenchEvent[] }>('get_workbench_run_events', { runId })

export const startWorkbenchRunEventStream = (runId: string) =>
  safeTauriInvoke<void>('start_workbench_run_event_stream', { runId })
