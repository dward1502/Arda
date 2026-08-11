import { FormEvent, useEffect, useState } from 'react'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import ModuleCard from '../ModuleCard'
import ApprovalPanel from '../../workbench/ApprovalPanel'
import ChangeReview from '../../workbench/ChangeReview'
import RunGraphView from '../../workbench/RunGraphView'
import RunTimeline from '../../workbench/RunTimeline'
import {
  approveWorkbenchRun,
  attachProjectContract,
  buildRunGraph,
  cancelWorkbenchRun,
  completeWorkbenchRunNode,
  createObjective,
  executeWorkbenchProviderNode,
  getWorkbenchRun,
  planWorkbenchRun,
  startWorkbenchRunEventStream,
  validateProjectContract,
  type AttachedProject,
  type MutationEnvelope,
  type ProjectValidation,
  type RunGraph,
  type RunNode,
  type RunRecord,
  type RunReviewEvidence,
  type WorkbenchEvent,
  type WorkbenchObjective,
} from '../../../lib/workbench'

function messageOf(error: unknown): string { return error instanceof Error ? error.message : String(error) }

const LAST_RUN_STORAGE_KEY = 'arda.workbench.last-run-id'
const objectiveStorageKey = (runId: string) => `arda.workbench.objective.${runId}`
const proposalStorageKey = (runId: string) => `arda.workbench.proposal.${runId}`
const approvalStorageKey = (runId: string) => `arda.workbench.approval.${runId}`

export interface OperatorSummary {
  whatHappened: string
  why: string
  whatCanAct: string
  evidenceQuality: string
  nextAction: string
}

function authorityLabel(node: RunNode | null): string {
  if (!node) return 'Arda may inspect and validate only. Only the operator can authorize a project mutation.'
  switch (node.authority) {
    case 'human_approval': return 'Only the operator can approve or reject this step.'
    case 'execute_with_approval': return 'The execution provider may act only after operator approval.'
    case 'verify': return 'The verifier can inspect recorded evidence; it cannot approve mutations.'
    case 'compensate_with_approval': return 'Recovery can act only after explicit operator approval.'
    default: return 'The runtime may inspect data without changing project state.'
  }
}

export function summarizeOperatorState(input: {
  graph: RunGraph | null
  events: WorkbenchEvent[]
  error: string | null
  message: string
  validationValid: boolean
  attached: boolean
  objectivePresent: boolean
  runPresent: boolean
}): OperatorSummary {
  const active = input.graph?.nodes.find((node) => ['failed', 'cancelled', 'running', 'blocked', 'ready', 'pending'].includes(node.state)) ?? null
  const latestReason = [...input.events].reverse().map((event) => event.kind?.reason ?? event.event?.reason ?? event.reason).find(Boolean)
  const completed = Boolean(input.graph?.nodes.length) && input.graph!.nodes.every((node) => node.state === 'succeeded')
  const failed = input.graph?.nodes.some((node) => node.state === 'failed') ?? false
  const whatHappened = input.error
    ? `The latest operation did not complete: ${input.error}`
    : completed
      ? 'The governed run completed and its recorded evidence is ready for review.'
      : active
        ? `The run is at the ${active.kind} step, which is ${active.state}.`
        : input.message
  const why = latestReason
    ? `Recorded reason: ${latestReason}`
    : active?.state === 'blocked'
      ? 'The step is blocked until its declared authority or dependency is satisfied.'
      : active
        ? `The run follows its governed sequence and ${active.kind} is the next incomplete step.`
        : 'No run decision has been made yet.'
  const evidenceQuality = input.error
    ? 'An application error is visible. Do not treat this operation as successful.'
    : !input.graph
      ? 'No execution receipt or project verification evidence exists yet.'
      : failed
        ? 'Failure evidence is recorded. Project success is not proven; inspect the failed boundary and its receipt before recovery.'
        : completed
          ? 'The graph reports all steps succeeded. Confirm the changed files, exact project check, provider provenance, and final receipt before closeout.'
          : 'Evidence is partial while the run is incomplete. A green step or provider statement alone is not proof of project success.'
  let nextAction = 'Validate a project contract before attaching it.'
  if (input.validationValid && !input.attached) nextAction = 'Review the validated permissions, then attach the project with approved proposal and approval IDs.'
  else if (input.attached && !input.objectivePresent) nextAction = 'Describe and capture the objective for this project.'
  else if (input.attached && input.objectivePresent && !input.runPresent) nextAction = 'Review the run graph, then plan the governed run.'
  else if (active?.kind === 'approval') nextAction = 'Review the requested authority and approve or reject the step.'
  else if (active?.state === 'failed' || active?.state === 'cancelled') nextAction = 'Inspect the recorded reason and receipts, then revise or recover the run.'
  else if (active) nextAction = `Select the ${active.kind} step and complete its displayed evidence or authority requirement.`
  else if (completed) nextAction = 'Review changed files, checks, provider receipt, and the final timeline before closeout.'
  return { whatHappened, why, whatCanAct: authorityLabel(active), evidenceQuality, nextAction }
}

export default function WorkbenchModule() {
  const [path, setPath] = useState('')
  const [validation, setValidation] = useState<ProjectValidation | null>(null)
  const [attached, setAttached] = useState<AttachedProject | null>(null)
  const [objectiveText, setObjectiveText] = useState('')
  const [objective, setObjective] = useState<WorkbenchObjective | null>(null)
  const [graph, setGraph] = useState<RunGraph | null>(null)
  const [run, setRun] = useState<RunRecord | null>(null)
  const [events, setEvents] = useState<WorkbenchEvent[]>([])
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null)
  const [proposalId, setProposalId] = useState('')
  const [approvalId, setApprovalId] = useState('')
  const [receiptDigest, setReceiptDigest] = useState('')
  const [evidenceJson, setEvidenceJson] = useState('')
  const [resumeRunId, setResumeRunId] = useState('')
  const [busy, setBusy] = useState(false)
  const [streamStatus, setStreamStatus] = useState<'idle' | 'connecting' | 'live' | 'error'>('idle')
  const [message, setMessage] = useState('Validate a typed project contract before attachment.')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const runId = window.localStorage.getItem(LAST_RUN_STORAGE_KEY)
    if (!runId) return
    let cancelled = false
    setBusy(true)
    void getWorkbenchRun(runId).then((record) => {
      if (cancelled) return
      setRun(record)
      setGraph(record.graph)
      setEvents(record.events)
      const storedObjective = window.localStorage.getItem(objectiveStorageKey(record.graph.run_id))
      if (storedObjective) {
        setObjectiveText(storedObjective)
        setObjective({ schemaVersion: 'arda.workbench.objective.v1', objectiveId: record.graph.objective_id, text: storedObjective, inputMode: 'text' })
      }
      setProposalId(window.localStorage.getItem(proposalStorageKey(record.graph.run_id)) ?? '')
      setApprovalId(window.localStorage.getItem(approvalStorageKey(record.graph.run_id)) ?? '')
      setSelectedNodeId(record.graph.nodes.find((node) => node.state !== 'succeeded')?.id ?? record.graph.nodes.at(-1)?.id ?? null)
      setMessage(`Resumed run ${record.graph.run_id} from the durable harness.`)
    }).catch((caught) => {
      if (cancelled) return
      window.localStorage.removeItem(LAST_RUN_STORAGE_KEY)
      setError(`Run resume failed: ${messageOf(caught)}`)
    }).finally(() => {
      if (!cancelled) setBusy(false)
    })
    return () => { cancelled = true }
  }, [])

  useEffect(() => {
    const runId = run?.graph.run_id
    if (runId) window.localStorage.setItem(LAST_RUN_STORAGE_KEY, runId)
  }, [run?.graph.run_id])

  useEffect(() => {
    const runId = run?.graph.run_id
    if (runId && objective?.text) window.localStorage.setItem(objectiveStorageKey(runId), objective.text)
  }, [run?.graph.run_id, objective?.text])

  useEffect(() => {
    const runId = run?.graph.run_id
    if (!runId) return
    let cancelled = false
    let unlisteners: UnlistenFn[] = []
    setStreamStatus('connecting')
    void Promise.all([
      listen<WorkbenchEvent>('arda://workbench-run-event', ({ payload }) => {
        if (cancelled || payload.run_id !== runId) return
        setStreamStatus('live')
        setEvents((current) => {
          if (payload.sequence !== undefined && current.some((event) => event.sequence === payload.sequence)) return current
          return [...current, payload].sort((left, right) => (left.sequence ?? 0) - (right.sequence ?? 0))
        })
        void getWorkbenchRun(runId).then((record) => {
          if (!cancelled) {
            setRun(record)
            setGraph(record.graph)
          }
        })
      }),
      listen<{ runId: string; error: string }>('arda://workbench-stream-error', ({ payload }) => {
        if (cancelled || payload.runId !== runId) return
        setStreamStatus('error')
        setError(`Run event stream failed: ${payload.error}`)
      }),
    ]).then(async (registered) => {
      if (cancelled) {
        registered.forEach((unlisten) => unlisten())
        return
      }
      unlisteners = registered
      await startWorkbenchRunEventStream(runId)
    }).catch((caught) => {
      if (!cancelled) {
        setStreamStatus('error')
        setError(`Run event stream unavailable: ${messageOf(caught)}`)
      }
    })
    return () => {
      cancelled = true
      unlisteners.forEach((unlisten) => unlisten())
    }
  }, [run?.graph.run_id])

  const envelope = (action: string): MutationEnvelope => {
    if (!proposalId.trim() || !approvalId.trim()) throw new Error('Proposal ID and approval ID are required for mutations')
    const stamp = Date.now()
    return { approval: { schema_version: 'arda.orome.task_approval.v1', proposal_id: proposalId.trim(), approval_id: approvalId.trim(), ledger_writes: [], decision: 'policy_safe', created_at_utc: new Date(stamp).toISOString() }, idempotency_key: `${action}-${stamp}` }
  }
  const act = async (operation: () => Promise<void>) => {
    setBusy(true); setError(null)
    try { await operation() } catch (caught) { setError(messageOf(caught)) } finally { setBusy(false) }
  }
  const resume = () => void act(async () => {
    const runId = resumeRunId.trim()
    if (!runId) throw new Error('Run ID is required for resume')
    const record = await getWorkbenchRun(runId)
    setRun(record); setGraph(record.graph); setEvents(record.events)
    const storedObjective = window.localStorage.getItem(objectiveStorageKey(record.graph.run_id))
    if (storedObjective) {
      setObjectiveText(storedObjective)
      setObjective({ schemaVersion: 'arda.workbench.objective.v1', objectiveId: record.graph.objective_id, text: storedObjective, inputMode: 'text' })
    }
    setProposalId(window.localStorage.getItem(proposalStorageKey(record.graph.run_id)) ?? '')
    setApprovalId(window.localStorage.getItem(approvalStorageKey(record.graph.run_id)) ?? '')
    setSelectedNodeId(record.graph.nodes.find((node) => node.state !== 'succeeded')?.id ?? record.graph.nodes.at(-1)?.id ?? null)
    setMessage(`Resumed run ${record.graph.run_id} from the durable harness.`)
  })
  const validate = () => void act(async () => {
    if (!path.startsWith('/')) throw new Error('Project contract path must be an absolute path')
    const result = await validateProjectContract(path)
    setValidation(result); setAttached(null)
    setMessage(result.valid ? 'Validation passed. Review permissions, posture, and checks before attachment.' : 'Validation failed. Project was not attached.')
  })
  const attach = () => void act(async () => {
    if (!validation?.valid) throw new Error('A passing validation is required before attachment')
    const result = await attachProjectContract(path, envelope('attach'))
    setAttached(result); setMessage(`Attached project ${validation.projectId}.`)
  })
  const capture = (event: FormEvent) => {
    event.preventDefault(); setError(null)
    try {
      const next = createObjective(objectiveText, 'text')
      setObjective(next)
      const nextGraph = buildRunGraph(next, validation?.projectId ?? 'unattached')
      setGraph(nextGraph); setSelectedNodeId(nextGraph.nodes[0]?.id ?? null); setMessage('Objective captured through the text contract. Voice will feed the same contract later.')
    } catch (caught) { setError(messageOf(caught)) }
  }
  const plan = () => void act(async () => {
    if (!attached || !validation?.projectId || !graph) throw new Error('Attach the project and capture an objective before planning')
    const result = await planWorkbenchRun(validation.projectId, graph, envelope('plan'))
    setRun(result); setGraph(result.graph)
    window.localStorage.setItem(proposalStorageKey(result.graph.run_id), proposalId.trim())
    window.localStorage.setItem(approvalStorageKey(result.graph.run_id), approvalId.trim())
    setEvents(result.events); setMessage(`Run ${result.graph.run_id} planned. Execution remains approval-gated.`)
  })
  const approve = (nodeId: string) => void act(async () => {
    if (!run) throw new Error('No planned run is available for approval')
    const result = await approveWorkbenchRun(run.graph.run_id, nodeId, envelope(`approve-${nodeId}`))
    setRun(result); setGraph(result.graph)
    setEvents(result.events)
    setMessage(`Approval ${nodeId} recorded by the typed harness endpoint.`)
  })
  const reject = (nodeId: string) => void act(async () => {
    if (!run) throw new Error('No planned run is available for rejection')
    const result = await cancelWorkbenchRun(
      run.graph.run_id,
      `Approval ${nodeId} rejected; revise the objective before planning a new run.`,
      envelope(`reject-${nodeId}`),
    )
    setRun(result); setGraph(result.graph); setEvents(result.events)
    setMessage(`Approval ${nodeId} rejected. Revise the objective and capture a replacement run.`)
  })
  const complete = () => void act(async () => {
    if (!run || !selectedNode) throw new Error('Select a run node before recording a receipt')
    if (!['execute', 'verify', 'review', 'close'].includes(selectedNode.kind)) throw new Error('The selected node does not accept an operator receipt')
    if (!receiptDigest.trim()) throw new Error('Receipt digest is required')
    let evidence: RunReviewEvidence | undefined
    if (evidenceJson.trim()) {
      const parsed: unknown = JSON.parse(evidenceJson)
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('Review evidence must be a JSON object')
      evidence = parsed as RunReviewEvidence
    }
    const result = await completeWorkbenchRunNode(run.graph.run_id, selectedNode.id, receiptDigest.trim(), envelope(`complete-${selectedNode.id}`), evidence)
    setRun(result); setGraph(result.graph); setEvents(result.events)
    setMessage(`${selectedNode.kind} node ${selectedNode.id} completed with a typed receipt.`)
  })
  const executeProvider = () => void act(async () => {
    if (!run || !selectedNode || selectedNode.kind !== 'execute') throw new Error('Select the execute node before invoking the provider')
    if (!objective?.text) throw new Error('The captured objective is required for provider execution')
    const result = await executeWorkbenchProviderNode(run.graph.run_id, selectedNode.id, objective.text, envelope(`provider-${selectedNode.id}`))
    setRun(result.run); setGraph(result.run.graph); setEvents(result.run.events)
    setMessage(`Live provider receipt ${String(result.receipt.receipt_digest ?? 'recorded')} correlated to run ${run.graph.run_id}.`)
  })

  const approvals = graph?.nodes.filter((node) => node.kind === 'approval') ?? []
  const selectedNode = graph?.nodes.find((node) => node.id === selectedNodeId) ?? null
  const operatorSummary = summarizeOperatorState({
    graph,
    events,
    error,
    message,
    validationValid: validation?.valid === true,
    attached: attached !== null,
    objectivePresent: objective !== null,
    runPresent: run !== null,
  })
  return (
    <ModuleCard title="Workbench" eyebrow="Governed operator surface" accent="gold">
      <div className="workbench" aria-label="ARDA Workbench">
        <section className="workbench-panel" aria-labelledby="project-attachment-title">
          <header><h3 id="project-attachment-title">Project attachment</h3><span>{attached ? 'attached' : validation?.valid ? 'ready' : 'not attached'}</span></header>
          <label>Project contract path<input value={path} onChange={(event) => { setPath(event.target.value); setValidation(null); setAttached(null) }} placeholder="/absolute/path/project.json" /></label>
          <div className="workbench-actions"><button type="button" disabled={busy} onClick={validate}>Validate project contract</button><button type="button" disabled={busy || !validation?.valid} onClick={attach}>Attach project</button></div>
          {validation ? <div className="workbench-contract" aria-label="Validated project contract">
            <strong>{validation.valid ? 'Validation passed' : 'Validation failed'}</strong>
            <dl><div><dt>Project</dt><dd>{validation.projectId ?? 'missing'}</dd></div><div><dt>Root</dt><dd>{validation.root ?? 'missing'}</dd></div><div><dt>Provider posture</dt><dd>{validation.providerPosture ?? 'not declared'}</dd></div><div><dt>Effective permissions</dt><dd>{validation.effectivePermissions.join(', ') || 'none declared'}</dd></div><div><dt>Project checks</dt><dd>{validation.projectChecks.join(', ') || 'none declared'}</dd></div></dl>
            {validation.errors.length ? <ul>{validation.errors.map((item) => <li key={item}>{item}</li>)}</ul> : null}
          </div> : null}
          <div className="workbench-approval-fields"><label>Proposal ID<input value={proposalId} onChange={(event) => setProposalId(event.target.value)} /></label><label>Approval ID<input value={approvalId} onChange={(event) => setApprovalId(event.target.value)} /></label></div>
          <label>Receipt digest<input value={receiptDigest} onChange={(event) => setReceiptDigest(event.target.value)} placeholder="sha256:..." /></label>
          <label>Review evidence JSON<textarea value={evidenceJson} onChange={(event) => setEvidenceJson(event.target.value)} rows={4} placeholder='{"changes":[],"tests":[],"provider_receipt":null}' /></label>
          <div className="workbench-approval-fields"><label>Resume run ID<input value={resumeRunId} onChange={(event) => setResumeRunId(event.target.value)} /></label><button type="button" disabled={busy || !resumeRunId.trim()} onClick={resume}>Resume durable run</button></div>
        </section>
        <form className="workbench-panel" onSubmit={capture}>
          <header><h3>Objective</h3><span>{objective?.inputMode ?? 'text first'}</span></header>
          <label>Objective<textarea value={objectiveText} onChange={(event) => setObjectiveText(event.target.value)} rows={3} /></label>
          <div className="workbench-actions"><button type="submit">Capture objective</button><button type="button" disabled={busy || !attached || !graph} onClick={plan}>Plan governed run</button></div>
          <small>Voice input will produce the same arda.workbench.objective.v1 contract; it does not bypass this boundary.</small>
        </form>
        {error ? <p role="alert" className="workbench-error">{error}</p> : null}<p role="status" aria-live="polite">{message} Run events: {streamStatus}.</p>
        <section className="workbench-panel workbench-operator-summary" aria-labelledby="workbench-operator-summary-title">
          <header><h3 id="workbench-operator-summary-title">Operator summary</h3><span>plain-language run state</span></header>
          <dl>
            <div><dt>What happened?</dt><dd>{operatorSummary.whatHappened}</dd></div>
            <div><dt>Why?</dt><dd>{operatorSummary.why}</dd></div>
            <div><dt>What can act?</dt><dd>{operatorSummary.whatCanAct}</dd></div>
            <div><dt>What evidence is available?</dt><dd>{operatorSummary.evidenceQuality}</dd></div>
            <div><dt>What should I do next?</dt><dd>{operatorSummary.nextAction}</dd></div>
          </dl>
        </section>
        <div className="workbench-first-screen"><RunGraphView graph={graph} selectedNodeId={selectedNodeId} onSelectNode={(node) => setSelectedNodeId(node.id)} /><ApprovalPanel approvals={approvals} busy={busy} onApprove={approve} onReject={reject} /><ChangeReview changes={run?.review?.changes ?? []} tests={run?.review?.tests ?? []} providerReceipt={run?.review?.provider_receipt} selectedNode={selectedNode} events={events} onComplete={complete} onExecuteProvider={executeProvider} busy={busy} /><RunTimeline events={events} graph={graph} streamStatus={streamStatus} /></div>
      </div>
    </ModuleCard>
  )
}
