// sigil: REPAIR
import { ClipboardCheck, FolderKanban, Play } from 'lucide-react'
import type { SystemActionCapabilityStatus, SystemActionDescriptor, SystemActionId } from '../../../lib/systemActionBus'

export interface PlanningTaskPivotPreview {
  title: string
  owner: string
  priority: string
  scope: string
  glyph: string
  origin: string
  status: string
  result: string
  command: string
}

interface PlanningActionContractPanelProps {
  actionDescriptors: SystemActionDescriptor[]
  capabilityStatuses: SystemActionCapabilityStatus[]
  busyActionId?: SystemActionId | null
  message?: string | null
  onRunAction?: (actionId: SystemActionId) => void
}

const QUEUE_PREVIEW_ACTION_ID: SystemActionId = 'arda.queue_preview_cleanup'
const TASK_PIVOT_ACTION_ID: SystemActionId = 'arda.queue_capture_pivot'

function shellQuote(value: string): string {
  return `'${value.replace(/'/g, "'\"'\"'")}'`
}

export function buildPlanningTaskPivotPreview(
  overrides: Partial<Omit<PlanningTaskPivotPreview, 'command'>> = {},
): PlanningTaskPivotPreview {
  const preview = {
    title: overrides.title ?? 'ARDA workstation action contract follow-up',
    owner: overrides.owner ?? 'prometheus',
    priority: overrides.priority ?? 'high',
    scope: overrides.scope ?? 'arda_hud',
    glyph: overrides.glyph ?? '↝',
    origin: overrides.origin ?? 'arda_planning_workstation',
    status: overrides.status ?? 'queued',
    result: overrides.result ?? 'pending',
  }

  return {
    ...preview,
    command: [
      'cargo run -p arda-cli -- utility task-pivot',
      shellQuote(preview.title),
      '--owner', preview.owner,
      '--priority', preview.priority,
      '--origin', preview.origin,
      '--scope', preview.scope,
      '--glyph', shellQuote(preview.glyph),
      '--status', preview.status,
      '--result', preview.result,
    ].join(' '),
  }
}

function descriptorById(descriptors: SystemActionDescriptor[], actionId: SystemActionId): SystemActionDescriptor | null {
  return descriptors.find((descriptor) => descriptor.id === actionId) ?? null
}

function capabilityById(statuses: SystemActionCapabilityStatus[], actionId: SystemActionId): SystemActionCapabilityStatus | null {
  return statuses.find((status) => status.id === actionId) ?? null
}

export default function PlanningActionContractPanel({
  actionDescriptors,
  capabilityStatuses,
  busyActionId,
  message,
  onRunAction,
}: PlanningActionContractPanelProps) {
  const queuePreviewDescriptor = descriptorById(actionDescriptors, QUEUE_PREVIEW_ACTION_ID)
  const queuePreviewStatus = capabilityById(capabilityStatuses, QUEUE_PREVIEW_ACTION_ID)
  const taskPivotDescriptor = descriptorById(actionDescriptors, TASK_PIVOT_ACTION_ID)
  const taskPivotStatus = capabilityById(capabilityStatuses, TASK_PIVOT_ACTION_ID)
  const taskPivotPreview = buildPlanningTaskPivotPreview()
  const canRunQueuePreview = Boolean(queuePreviewStatus?.manualRunAvailable && queuePreviewDescriptor?.riskLevel !== 'governed_mutation' && onRunAction)
  const queuePreviewBusy = busyActionId === QUEUE_PREVIEW_ACTION_ID

  return (
    <section className="planning-action-contract" aria-label="Planning workstation action contract">
      <div className="module-subtitle"><FolderKanban size={14} /> Planning Action Contract</div>
      <div className="split-stack">
        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>{queuePreviewDescriptor?.label ?? 'Preview Queue Cleanup'}</strong>
            <span>{queuePreviewDescriptor?.riskLevel ?? 'dry_run'}</span>
          </div>
          <p>{queuePreviewDescriptor?.purpose ?? 'Preview queue pressure and cleanup candidates before mutation.'}</p>
          <dl className="operating-surface-capability-contract">
            <div><dt>Executor</dt><dd>{queuePreviewDescriptor?.executor ?? 'not declared'}</dd></div>
            <div><dt>Gate</dt><dd>{queuePreviewDescriptor?.governanceGate ?? 'not declared'}</dd></div>
            <div><dt>Status</dt><dd>{queuePreviewStatus?.currentStatus ?? 'not observed'}</dd></div>
            <div><dt>Receipt</dt><dd>{queuePreviewStatus?.receiptPath ?? queuePreviewDescriptor?.receiptPath ?? 'not declared'}</dd></div>
          </dl>
          <button
            type="button"
            className={`refresh-button ${queuePreviewBusy ? 'refresh-button--active' : ''}`}
            disabled={!canRunQueuePreview || queuePreviewBusy}
            onClick={() => onRunAction?.(QUEUE_PREVIEW_ACTION_ID)}
          >
            <Play size={12} /> {queuePreviewBusy ? 'Running Preview...' : 'Run Queue Preview'}
          </button>
        </article>

        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>{taskPivotDescriptor?.label ?? 'Capture Task Pivot'}</strong>
            <span>{taskPivotDescriptor?.riskLevel ?? 'governed_mutation'}</span>
          </div>
          <p>{taskPivotDescriptor?.purpose ?? 'Record a meaningful project pivot into the durable task queue after operator intent is clear.'}</p>
          <dl className="operating-surface-capability-contract">
            <div><dt>Executor</dt><dd>{taskPivotDescriptor?.executor ?? 'arda-cli utility task-pivot'}</dd></div>
            <div><dt>Gate</dt><dd>{taskPivotDescriptor?.governanceGate ?? 'operator_intent_required'}</dd></div>
            <div><dt>Status</dt><dd>{taskPivotStatus?.currentStatus ?? 'blocked'}</dd></div>
            <div><dt>Receipt</dt><dd>{taskPivotStatus?.receiptPath ?? taskPivotDescriptor?.receiptPath ?? 'core/state/queue_summary.json'}</dd></div>
          </dl>
        </article>
      </div>

      <article className="document-list__item planning-action-contract__preview">
        <div className="document-list__title-row">
          <strong>Task Pivot Record Preview</strong>
          <span>operator-gated</span>
        </div>
        <dl className="operating-surface-capability-contract">
          <div><dt>title</dt><dd>{taskPivotPreview.title}</dd></div>
          <div><dt>owner</dt><dd>{taskPivotPreview.owner}</dd></div>
          <div><dt>priority</dt><dd>{taskPivotPreview.priority}</dd></div>
          <div><dt>scope</dt><dd>{taskPivotPreview.scope}</dd></div>
          <div><dt>origin</dt><dd>{taskPivotPreview.origin}</dd></div>
          <div><dt>glyph</dt><dd>{taskPivotPreview.glyph}</dd></div>
        </dl>
        <div className="planning-action-contract__command">
          <ClipboardCheck size={14} />
          <code>{taskPivotPreview.command}</code>
        </div>
        <p>ARDA shows the exact queue-write shape here, but the governed mutation remains operator-gated and is not mixed into the generic refresh lane.</p>
      </article>

      {message ? <p className="planning-action-contract__message">{message}</p> : null}
    </section>
  )
}
