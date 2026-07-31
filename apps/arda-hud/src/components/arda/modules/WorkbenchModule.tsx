import { FormEvent, useState } from 'react'
import ModuleCard from '../ModuleCard'
import ApprovalPanel from '../../workbench/ApprovalPanel'
import ChangeReview from '../../workbench/ChangeReview'
import RunGraphView from '../../workbench/RunGraphView'
import RunTimeline from '../../workbench/RunTimeline'
import {
  approveWorkbenchRun,
  attachProjectContract,
  buildRunGraph,
  createObjective,
  planWorkbenchRun,
  validateProjectContract,
  type AttachedProject,
  type MutationEnvelope,
  type ProjectValidation,
  type RunGraph,
  type RunRecord,
  type WorkbenchEvent,
  type WorkbenchObjective,
} from '../../../lib/workbench'

function messageOf(error: unknown): string { return error instanceof Error ? error.message : String(error) }

export default function WorkbenchModule() {
  const [path, setPath] = useState('')
  const [validation, setValidation] = useState<ProjectValidation | null>(null)
  const [attached, setAttached] = useState<AttachedProject | null>(null)
  const [objectiveText, setObjectiveText] = useState('')
  const [objective, setObjective] = useState<WorkbenchObjective | null>(null)
  const [graph, setGraph] = useState<RunGraph | null>(null)
  const [run, setRun] = useState<RunRecord | null>(null)
  const [events, setEvents] = useState<WorkbenchEvent[]>([])
  const [proposalId, setProposalId] = useState('')
  const [approvalId, setApprovalId] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('Validate a typed project contract before attachment.')
  const [error, setError] = useState<string | null>(null)

  const envelope = (action: string): MutationEnvelope => {
    if (!proposalId.trim() || !approvalId.trim()) throw new Error('Proposal ID and approval ID are required for mutations')
    const stamp = Date.now()
    return { approval: { schema_version: 'arda.orome.task_approval.v1', proposal_id: proposalId.trim(), approval_id: approvalId.trim(), ledger_writes: [], decision: 'policy_safe', created_at_utc: new Date(stamp).toISOString() }, idempotency_key: `${action}-${stamp}` }
  }
  const act = async (operation: () => Promise<void>) => {
    setBusy(true); setError(null)
    try { await operation() } catch (caught) { setError(messageOf(caught)) } finally { setBusy(false) }
  }
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
      setGraph(nextGraph); setMessage('Objective captured through the text contract. Voice will feed the same contract later.')
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

  const approvals = graph?.nodes.filter((node) => node.kind === 'approval') ?? []
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
        </section>
        <form className="workbench-panel" onSubmit={capture}>
          <header><h3>Objective</h3><span>{objective?.inputMode ?? 'text first'}</span></header>
          <label>Objective<textarea value={objectiveText} onChange={(event) => setObjectiveText(event.target.value)} rows={3} /></label>
          <div className="workbench-actions"><button type="submit">Capture objective</button><button type="button" disabled={busy || !attached || !graph} onClick={plan}>Plan governed run</button></div>
          <small>Voice input will produce the same arda.workbench.objective.v1 contract; it does not bypass this boundary.</small>
        </form>
        {error ? <p role="alert" className="workbench-error">{error}</p> : null}<p role="status" aria-live="polite">{message}</p>
        <div className="workbench-first-screen"><RunGraphView graph={graph} /><ApprovalPanel approvals={approvals} busy={busy} onApprove={approve} /><ChangeReview changes={[]} tests={[]} /><RunTimeline events={events} graph={graph} /></div>
      </div>
    </ModuleCard>
  )
}
