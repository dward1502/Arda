import { parseOperatorProjection } from '../../lib/operatorProjection'

export interface OperatorProjectionCanvasModel {
  ok: true
  projectionId: string
  authority: 'read_only'
  freshness: string
  title: string
  rows: string[]
}

export interface OperatorProjectionCanvasFailure {
  ok: false
  reason: 'operator projection unavailable'
}

export function resolveOperatorProjectionCanvasModel(
  value: unknown,
): OperatorProjectionCanvasModel | OperatorProjectionCanvasFailure {
  try {
    const projection = parseOperatorProjection(value)
    const objective = projection.objectives.find((item) => item.status === 'active')
      ?? projection.objectives[0]
    const run = projection.runs.find((item) => item.status === 'running')
      ?? projection.runs[0]
    const approval = projection.pending_approvals.find((item) => item.status === 'pending')
      ?? projection.pending_approvals[0]
    const rows = [
      objective ? `OBJECTIVE  ${objective.objective_id}  ${objective.status.toUpperCase()}` : 'OBJECTIVE  NONE',
      run ? `RUN        ${run.run_id}  ${run.status.toUpperCase()}` : 'RUN        NONE',
      approval ? `APPROVAL   ${approval.approval_id}  ${approval.status.toUpperCase()}` : 'APPROVAL   NONE',
      ...projection.dependencies.map((dependency) => (
        `DEPENDENCY ${dependency.dependency_id}  ${dependency.health.toUpperCase()} / ${dependency.freshness.toUpperCase()}`
      )),
    ]
    return {
      ok: true,
      projectionId: projection.projection_id,
      authority: projection.authority,
      freshness: projection.freshness,
      title: objective?.title ?? 'No active objective',
      rows,
    }
  } catch {
    return { ok: false, reason: 'operator projection unavailable' }
  }
}
