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

export interface MutationIntent {
  approvalReference: string
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
  objectiveId?: string
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
  return {
    schemaVersion: 'arda.workbench.objective.v1',
    text: trimmed,
    inputMode,
  }
}

export const validateProjectContract = (path: string) =>
  safeTauriInvoke<ProjectValidation>('validate_project_contract', { path })

export const attachProjectContract = (path: string, intent: MutationIntent) =>
  safeTauriInvoke<AttachedProject>('attach_project_contract', { path, intent })

export const planWorkbenchRun = (projectId: string, objective: WorkbenchObjective, intent: MutationIntent) =>
  safeTauriInvoke<RunRecord>('plan_workbench_run', { request: { project_id: projectId, objective: { text: objective.text, input_mode: objective.inputMode }, intent } })

export const approveWorkbenchRun = (runId: string, nodeId: string, intent: MutationIntent) =>
  safeTauriInvoke<RunRecord>('approve_workbench_run', { request: { run_id: runId, node_id: nodeId, intent } })

export const completeWorkbenchRunNode = (runId: string, nodeId: string, receiptDigest: string, intent: MutationIntent, evidence?: RunReviewEvidence) =>
  safeTauriInvoke<RunRecord>('complete_workbench_run_node', { request: { run_id: runId, node_id: nodeId, receipt_digest: receiptDigest, intent, evidence } })

export const executeWorkbenchProviderNode = (runId: string, nodeId: string, objective: string, intent: MutationIntent) =>
  safeTauriInvoke<ExecuteProviderNodeResponse>('execute_workbench_provider_node', { request: { run_id: runId, node_id: nodeId, objective, intent } })

export const cancelWorkbenchRun = (runId: string, reason: string, intent: MutationIntent) =>
  safeTauriInvoke<RunRecord>('cancel_workbench_run', { request: { run_id: runId, reason, intent } })

export const getWorkbenchRun = (runId: string) =>
  safeTauriInvoke<RunRecord>('get_workbench_run', { runId })

export const getWorkbenchRunEvents = (runId: string) =>
  safeTauriInvoke<{ events: WorkbenchEvent[] }>('get_workbench_run_events', { runId })

export const startWorkbenchRunEventStream = (runId: string) =>
  safeTauriInvoke<void>('start_workbench_run_event_stream', { runId })
