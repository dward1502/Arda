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
  type RunRecord,
  type RunReviewEvidence,
  type WorkbenchEvent,
  type WorkbenchObjective,
} from '../../../lib/workbench'

function messageOf(error: unknown): string { return error instanceof Error ? error.message : String(error) }

const LAST_RUN_STORAGE_KEY = 'arda.workbench.last-run-id'
const objectiveStorageKey = (runId: string) => `arda.workbench.objective.${runId}`

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
    setEvents(result.events); setMessage(`Run ${result.graph.run_id} planned. Execution remains approval-gated.`)
  })
  const approve = (nodeId: string) => void act(async () => {
    if (!run) throw new Error('No planned run is available for approval')
    const result = await approveWorkbenchRun(run.graph.run_id, nodeId, envelope(`approve-${nodeId}`))
    setRun(result); setGraph(result.graph)
    setEvents(result.events)
    setMessage(`Approval ${nodeId} recorded by the typed harness endpoint.`)
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
        <div className="workbench-first-screen"><RunGraphView graph={graph} selectedNodeId={selectedNodeId} onSelectNode={(node) => setSelectedNodeId(node.id)} /><ApprovalPanel approvals={approvals} busy={busy} onApprove={approve} /><ChangeReview changes={run?.review?.changes ?? []} tests={run?.review?.tests ?? []} providerReceipt={run?.review?.provider_receipt} selectedNode={selectedNode} events={events} onComplete={complete} onExecuteProvider={executeProvider} busy={busy} /><RunTimeline events={events} graph={graph} /></div>
      </div>
    </ModuleCard>
  )
}
