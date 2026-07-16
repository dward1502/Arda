// sigil: REPAIR
import { useMemo, useState } from 'react'
import { CheckCircle2, FileText, XCircle } from 'lucide-react'
import { openSourcePath } from '../../../../lib/weathertop'
import { primarySigilForSource } from '../../../../lib/soterionRender'
import type { OperatorCockpitSurface } from './OperatorCockpitPanel'

type QueueTask = OperatorCockpitSurface['queue']['items'][number]
type ListItem = OperatorCockpitSurface['queue']['items'][number]
interface TaskListViewerProps {
  surface: OperatorCockpitSurface
  onApprove?: (taskId: string) => void
  onDiscard?: (taskId: string) => void
  listItems?: ListItem[] | null
}

function slugFromTitle(title: string): string {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

function deriveTaskSourcePath(task: QueueTask): string {
  const implementedPlan = task.title.match(/^Implement plan:\s*(.+)$/i)
  if (implementedPlan?.[1]) return `docs/plans/${implementedPlan[1]}.md`

  const implementedContract = task.title.match(/^Implement contract:\s*(.+)$/i)
  if (implementedContract?.[1]) return `docs/contracts/${implementedContract[1]}.md`

  const idStem = task.id.replace(/^tsk_\d+_/, '')
  return `docs/plans/${idStem}_plan.md`
}

function previewLines(task: QueueTask, sourcePath: string): string[] {
  return [
    `id: ${task.id}`,
    `title: ${task.title}`,
    `owner: ${task.owner}`,
    `status: ${task.status}`,
    `priority: ${task.priority}`,
    `source: ${sourcePath}`,
  ]
}

export default function TaskListViewer({ surface, onApprove, onDiscard }: TaskListViewerProps) {
  const marker = primarySigilForSource('queue')
  const tasks = surface.queue.items
  const [selectedId, setSelectedId] = useState(tasks[0]?.id ?? '')
  const [message, setMessage] = useState<string | null>(null)
  const selectedTask = tasks.find((task) => task.id === selectedId) ?? tasks[0] ?? null
  const sourcePath = selectedTask ? deriveTaskSourcePath(selectedTask) : ''
  const detailPreview = useMemo(
    () => (selectedTask ? previewLines(selectedTask, sourcePath) : []),
    [selectedTask, sourcePath],
  )

  const openSelectedSource = async () => {
    if (!selectedTask || !sourcePath) return
    try {
      const result = await openSourcePath(sourcePath)
      setMessage(result.ok ? `Opened ${result.sourcePath}` : result.message)
    } catch (error) {
      setMessage(error instanceof Error ? error.message : 'Native source opener is unavailable.')
    }
  }

  const approveSelected = () => {
    if (!selectedTask) return
    onApprove?.(selectedTask.id)
    setMessage(`Approve requested for ${selectedTask.id}`)
  }

  const discardSelected = () => {
    if (!selectedTask) return
    onDiscard?.(selectedTask.id)
    setMessage(`Discard requested for ${selectedTask.id}`)
  }

  return (
    <section className="systems-panel task-list-viewer" aria-label="ARDA task list viewer">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">{marker} Task Queue</div>
        <h3 className="systems-panel__title">Open Work Viewer</h3>
      </header>

      <div className="systems-kpi-grid">
        <article className={`systems-kpi ${surface.queue.openTotal > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Open Total</span>
          <strong className="systems-kpi__value">{surface.queue.openTotal}</strong>
          <span className="systems-kpi__note">in-flight metadata only</span>
        </article>
      </div>

      <div className="systems-chip-cloud">
        <span className={`systems-chip ${surface.queue.statusSplit.ready > 0 ? 'systems-chip--good' : 'systems-chip--warn'}`}>
          ready {surface.queue.statusSplit.ready}
        </span>
        <span className={`systems-chip ${surface.queue.statusSplit.pending > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          pending {surface.queue.statusSplit.pending}
        </span>
        <span className={`systems-chip ${surface.queue.statusSplit.inProgress > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          in progress {surface.queue.statusSplit.inProgress}
        </span>
        <span className={`systems-chip ${surface.queue.statusSplit.blocked > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          blocked {surface.queue.statusSplit.blocked}
        </span>
      </div>

      <div className="task-list-viewer__layout">
        <div className="task-list-viewer__items" role="list" aria-label="Open queue tasks">
          {tasks.length > 0 ? tasks.map((task) => (
            <button
              key={task.id}
              type="button"
              className={task.id === selectedTask?.id ? 'task-list-viewer__item is-active' : 'task-list-viewer__item'}
              onClick={() => setSelectedId(task.id)}
            >
              <span className="task-list-viewer__item-title">{task.title}</span>
              <span className="task-list-viewer__item-meta">
                {task.owner} | {task.status} | {task.priority}
              </span>
            </button>
          )) : (
            <article className="document-list__item">
              <p>No open queue items loaded from `core/state/queue_summary.json`.</p>
            </article>
          )}
        </div>

        <article className="document-list__item task-list-viewer__detail">
          <div className="document-list__title-row">
            <strong>{selectedTask?.title ?? 'No task selected'}</strong>
            {selectedTask ? <span className="systems-chip systems-chip--muted">{selectedTask.status}</span> : null}
          </div>
          {detailPreview.map((line) => (
            <p key={line}>{line}</p>
          ))}
          {sourcePath ? <p className="task-list-viewer__source">Likely source: {sourcePath}</p> : null}
          <div className="task-list-viewer__actions">
            <button type="button" className="refresh-button" onClick={openSelectedSource} disabled={!selectedTask}>
              <FileText size={14} aria-hidden="true" />
              Open Source
            </button>
            <button type="button" className="refresh-button refresh-button--active" onClick={approveSelected} disabled={!selectedTask}>
              <CheckCircle2 size={14} aria-hidden="true" />
              Approve
            </button>
            <button type="button" className="refresh-button task-list-viewer__discard" onClick={discardSelected} disabled={!selectedTask}>
              <XCircle size={14} aria-hidden="true" />
              Discard
            </button>
          </div>
          {message ? <p className="systems-panel__note">{message}</p> : null}
        </article>
      </div>
    </section>
  )
}

export { deriveTaskSourcePath, slugFromTitle }
