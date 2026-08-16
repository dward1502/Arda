// sigil: REPAIR
import { Activity, BookOpenText, Briefcase, FolderKanban, Settings, Shield } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import SourceCoverageBadge, { type SourceCoverageBadgeState } from './SourceCoverageBadge'
import type { SystemActionCapabilityStatus, SystemActionDescriptor, SystemActionId } from '../../../lib/systemActionBus'
import type { JsonRecord } from '../../../lib/ardaSource'

export type OperatingSurfaceLaneId =
  | 'Now'
  | 'Work'
  | 'Decisions'
  | 'Knowledge'
  | 'Health'
  | 'Business'
  | 'Evidence'
  | 'Settings'

export interface OperatingSurfaceLaneReport {
  lane: OperatingSurfaceLaneId
  current: string
  gap: string
  next: string
  evidence: string[]
  status: 'ready' | 'partial' | 'gap'
}

export interface LiveRuntimeChannelEvidence {
  channel: string
  source: string
  status: string
  sequence: number
  lastEventIso: string
  durableProjection: string
}

export interface CommandConsoleSurface {
  metrics: Array<{ label: string; value: string; tone: 'good' | 'warn' | 'muted' }>
  lanes: Array<{ title: string; value: string; detail: string; status: 'ready' | 'partial' | 'gap' }>
  workItems: Array<{ id: string; title: string; owner: string; status: string; priority: string; recordClass: string; laneSubclass: string; promotionReceiptRequired: string }>
  messages: Array<{ id: string; source: string; actor: string; intent: string; body: string; timestamp: string }>
  receipts: Array<{ id: string; source: string; status: string; task: string; summary: string; timestamp: string }>
  conversations: Array<{ id: string; topic: string; speaker: string; messageClass: string; summary: string; risk: string; timestamp: string }>
  scoutItems: Array<{ id: string; kind: string; question: string; requester: string; status: string; sourcePolicy: string; timestamp: string }>
  gaps: Array<{ title: string; detail: string }>
}

interface OperatingSurfacePlanModuleProps {
  reports: OperatingSurfaceLaneReport[]
  actionDescriptors: SystemActionDescriptor[]
  capabilityStatuses: SystemActionCapabilityStatus[]
  liveRuntime?: LiveRuntimeChannelEvidence | null
  remoteConfidenceSnapshot?: JsonRecord | null
  safeLocalWorkCyclePreflight?: JsonRecord | null
  commandConsole?: CommandConsoleSurface | null
  sourceCoverage?: SourceCoverageBadgeState
  tag?: string
  actionBusyId?: SystemActionId | null
  actionMessage?: string | null
  onRunRefreshAction?: (actionId: SystemActionId) => void
}

const laneIcons: Record<OperatingSurfaceLaneId, typeof Activity> = {
  Now: Activity,
  Work: FolderKanban,
  Decisions: Shield,
  Knowledge: BookOpenText,
  Health: Activity,
  Business: Briefcase,
  Evidence: BookOpenText,
  Settings,
}

const REFRESH_ACTION_IDS: SystemActionId[] = [
  'arda.chronos_run_provider_checks',
  'charon.refresh_provider_intelligence',
  'arda.queue_preview_cleanup',
  'arda.hades_run_nightly',
  'arda.hades_preview_organization_plan',
  'arda.hades_run_link_check',
  'arda.athena_refresh_digest',
  'arda.audit_run_system_audit',
  'arda.audit_run_repeated_audit',
  'arda.setup_run_readiness_check',
]

const asRecord = (value: unknown): JsonRecord | null => (
  value !== null && typeof value === 'object' && !Array.isArray(value) ? value as JsonRecord : null
)

const asString = (value: unknown, fallback: string): string => (
  typeof value === 'string' && value.length > 0 ? value : fallback
)

const asBoolean = (value: unknown): boolean => value === true

const asNumber = (value: unknown, fallback = 0): number => (
  typeof value === 'number' && Number.isFinite(value) ? value : fallback
)

function remoteConfidenceContract(snapshot: JsonRecord | null | undefined) {
  const policy = asRecord(snapshot?.side_effect_policy)
  const ardaHud = asRecord(snapshot?.arda_hud)

  return {
    status: asString(snapshot?.status, 'unknown'),
    mode: asString(snapshot?.mode, 'unpublished'),
    generatedAt: asString(snapshot?.generated_at_utc, 'not recorded'),
    targetStatePath: asString(ardaHud?.target_state_path, 'core/state/remote_confidence_snapshot.json'),
    projectionMode: asString(ardaHud?.projection_mode, 'local_runtime_state_file'),
    externalMessagesSent: asBoolean(policy?.external_messages_sent),
    serviceRestart: asBoolean(policy?.service_restart),
    credentialChange: asBoolean(policy?.credential_change),
  }
}

function safeLocalWorkCycleContract(preflight: JsonRecord | null | undefined) {
  const candidateSummary = asRecord(preflight?.candidate_summary)
  const policy = asRecord(preflight?.side_effect_policy)
  const ardaHud = asRecord(preflight?.arda_hud)

  return {
    checkedAt: asString(preflight?.checked_at_utc, 'not recorded'),
    projectionMode: asString(ardaHud?.projection_mode, 'local_report_file'),
    totalCount: asNumber(candidateSummary?.total_count),
    safeLocalCount: asNumber(candidateSummary?.safe_local_count),
    humanGatedCount: asNumber(candidateSummary?.human_gated_count),
    externalMessagesSent: asBoolean(policy?.external_messages_sent),
    mutatesTaskStatus: asBoolean(policy?.mutates_task_status),
    newRailRequired: asBoolean(ardaHud?.new_rail_required),
    forksAutonomyLogic: asBoolean(ardaHud?.forks_autonomy_logic),
    liveDiscordValidation: asString(policy?.live_discord_validation, 'human_gated_separate'),
  }
}

export default function OperatingSurfacePlanModule({
  reports,
  actionDescriptors,
  capabilityStatuses,
  liveRuntime,
  remoteConfidenceSnapshot,
  safeLocalWorkCyclePreflight,
  commandConsole,
  sourceCoverage,
  tag,
  actionBusyId,
  actionMessage,
  onRunRefreshAction,
}: OperatingSurfacePlanModuleProps) {
  const scheduled = actionDescriptors.filter((action) => action.automationEligible)
  const manual = actionDescriptors.filter((action) => action.userTriggerEligible)
  const governed = actionDescriptors.filter((action) => action.riskLevel === 'governed_mutation')
  const persistedStatusCount = capabilityStatuses.filter((action) => action.lastRun !== 'not observed in this HUD profile').length
  const capabilityStatusSummary = capabilityStatuses.reduce(
    (summary, action) => ({
      ...summary,
      [action.currentStatus]: (summary[action.currentStatus] ?? 0) + 1,
    }),
    {} as Partial<Record<SystemActionCapabilityStatus['currentStatus'], number>>,
  )
  const now = reports.find((report) => report.lane === 'Now')
  const work = reports.find((report) => report.lane === 'Work')
  const decisions = reports.find((report) => report.lane === 'Decisions')
  const health = reports.find((report) => report.lane === 'Health')
  const firstScreen = [
    { label: 'Current state', report: now },
    { label: 'Human attention', report: decisions },
    { label: 'Active work', report: work },
    { label: 'System health', report: health },
  ]
  const remoteConfidence = remoteConfidenceContract(remoteConfidenceSnapshot)
  const safeLocalPreflight = safeLocalWorkCycleContract(safeLocalWorkCyclePreflight)
  const descriptorsById = new Map(actionDescriptors.map((action) => [action.id, action]))
  const refreshActions = capabilityStatuses
    .map((status) => ({ status, descriptor: descriptorsById.get(status.id) }))
    .filter(({ status, descriptor }) => (
      REFRESH_ACTION_IDS.includes(status.id) &&
      status.manualRunAvailable &&
      descriptor &&
      descriptor.riskLevel !== 'governed_mutation'
    ))

  return (
    <ModuleCard
      title="Operating Surface Review"
      eyebrow="ARDA plan alignment"
      accent="cyan"
      tag={tag}
      actions={<SourceCoverageBadge coverage={sourceCoverage} />}
    >
      <div className="operating-surface-summary">
        <article className="systems-kpi systems-kpi--good">
          <span className="systems-kpi__label">Scheduled Capabilities</span>
          <strong className="systems-kpi__value">{scheduled.length}</strong>
          <span className="systems-kpi__note">descriptor-backed</span>
        </article>
        <article className="systems-kpi systems-kpi--good">
          <span className="systems-kpi__label">Manual Calls</span>
          <strong className="systems-kpi__value">{manual.length}</strong>
          <span className="systems-kpi__note">adapter-routed</span>
        </article>
        <article className="systems-kpi systems-kpi--warn">
          <span className="systems-kpi__label">Governed Mutations</span>
          <strong className="systems-kpi__value">{governed.length}</strong>
          <span className="systems-kpi__note">review required</span>
        </article>
        <article className="systems-kpi systems-kpi--good">
          <span className="systems-kpi__label">Durable Status</span>
          <strong className="systems-kpi__value">{persistedStatusCount}/{capabilityStatuses.length}</strong>
          <span className="systems-kpi__note">last-run persisted</span>
        </article>
        <article className={`systems-kpi systems-kpi--${liveRuntime ? 'good' : 'warn'}`}>
          <span className="systems-kpi__label">Live Runtime</span>
          <strong className="systems-kpi__value">{liveRuntime ? `seq ${liveRuntime.sequence}` : 'offline'}</strong>
          <span className="systems-kpi__note">{liveRuntime ? `${liveRuntime.source} / ${liveRuntime.status}` : 'no event observed'}</span>
        </article>
      </div>

      {liveRuntime ? (
        <article className="document-list__item operating-surface-live-runtime" aria-label="Live runtime channel evidence">
          <div className="document-list__title-row">
            <strong>Live Runtime</strong>
            <span>{liveRuntime.channel}</span>
          </div>
          <dl className="operating-surface-capability-contract">
            <div><dt>Transport</dt><dd>{liveRuntime.source} / {liveRuntime.status}</dd></div>
            <div><dt>Last event</dt><dd>{liveRuntime.lastEventIso}</dd></div>
            <div><dt>Sequence</dt><dd>seq {liveRuntime.sequence}</dd></div>
            <div><dt>Durable projection</dt><dd>{liveRuntime.durableProjection}</dd></div>
          </dl>
        </article>
      ) : null}

      {remoteConfidenceSnapshot ? (
        <article className="document-list__item operating-surface-live-runtime" aria-label="Remote confidence local projection">
          <div className="document-list__title-row">
            <strong>Remote Confidence</strong>
            <span>{remoteConfidence.status}</span>
          </div>
          <dl className="operating-surface-capability-contract">
            <div><dt>Mode</dt><dd>{remoteConfidence.mode}</dd></div>
            <div><dt>Generated</dt><dd>{remoteConfidence.generatedAt}</dd></div>
            <div><dt>Projection</dt><dd>{remoteConfidence.projectionMode}</dd></div>
            <div><dt>State file</dt><dd>{remoteConfidence.targetStatePath}</dd></div>
            <div><dt>Discord</dt><dd>{remoteConfidence.externalMessagesSent ? 'external Discord send observed' : 'no external Discord send'}</dd></div>
            <div><dt>Services</dt><dd>{remoteConfidence.serviceRestart ? 'service restart observed' : 'no service restart'}</dd></div>
            <div><dt>Credentials</dt><dd>{remoteConfidence.credentialChange ? 'credential change observed' : 'no credential change'}</dd></div>
          </dl>
        </article>
      ) : null}

      {safeLocalWorkCyclePreflight ? (
        <article className="document-list__item operating-surface-live-runtime" aria-label="Safe-local work-cycle preflight">
          <div className="document-list__title-row">
            <strong>Safe-Local Work Cycle</strong>
            <span>{safeLocalPreflight.safeLocalCount} safe / {safeLocalPreflight.totalCount} total</span>
          </div>
          <dl className="operating-surface-capability-contract">
            <div><dt>Human gate</dt><dd>{safeLocalPreflight.humanGatedCount} human-gated</dd></div>
            <div><dt>Checked</dt><dd>{safeLocalPreflight.checkedAt}</dd></div>
            <div><dt>Projection</dt><dd>{safeLocalPreflight.projectionMode}</dd></div>
            <div><dt>Discord</dt><dd>{safeLocalPreflight.externalMessagesSent ? 'external Discord send observed' : 'no external Discord send'}</dd></div>
            <div><dt>Task status</dt><dd>{safeLocalPreflight.mutatesTaskStatus ? 'mutates task status' : 'does not mutate task status'}</dd></div>
            <div><dt>Rail</dt><dd>{safeLocalPreflight.newRailRequired ? 'new rail required' : 'no new rail required'}</dd></div>
            <div><dt>Autonomy</dt><dd>{safeLocalPreflight.forksAutonomyLogic ? 'forks autonomy logic' : 'does not fork autonomy logic'}</dd></div>
            <div><dt>Live validation</dt><dd>{safeLocalPreflight.liveDiscordValidation === 'human_gated_separate' ? 'live Discord validation human-gated separately' : safeLocalPreflight.liveDiscordValidation}</dd></div>
          </dl>
        </article>
      ) : null}

      <div className="operating-surface-first-screen" aria-label="First screen command summary">
        {firstScreen.map((item) => (
          <article className="operating-surface-first-screen__item" key={item.label}>
            <span className="operating-surface-first-screen__label">{item.label}</span>
            <strong>{item.report?.lane ?? 'Loading'}</strong>
            <p>{item.report?.current ?? 'Waiting for projected runtime state.'}</p>
            <span className={`systems-chip systems-chip--${item.report?.status === 'ready' ? 'good' : item.report?.status === 'gap' ? 'warn' : 'muted'}`}>
              {item.report?.status ?? 'loading'}
            </span>
          </article>
        ))}
      </div>

      {commandConsole ? (
        <section className="arda-command-console" aria-label="ARDA command console">
          <header className="arda-command-console__header">
            <div>
              <span>Primary Console</span>
              <strong>Work, messages, receipts, and gaps</strong>
            </div>
            <div className="arda-command-console__metrics">
              {commandConsole.metrics.map((metric) => (
                <span className={`systems-chip systems-chip--${metric.tone}`} key={metric.label}>
                  {metric.label}: {metric.value}
                </span>
              ))}
            </div>
          </header>

          <div className="arda-command-console__lane-grid">
            {commandConsole.lanes.map((lane) => (
              <article className={`arda-command-console__lane arda-command-console__lane--${lane.status}`} key={lane.title}>
                <span>{lane.title}</span>
                <strong>{lane.value}</strong>
                <p>{lane.detail}</p>
              </article>
            ))}
          </div>

          <div className="arda-command-console__body">
            <section className="arda-command-console__panel" aria-label="Active work lane">
              <div className="module-subtitle"><FolderKanban size={14} /> Work Lane</div>
              <div className="document-list compact">
                {commandConsole.workItems.length > 0 ? commandConsole.workItems.map((item) => (
                  <article className="document-list__item" key={item.id}>
                    <div className="document-list__title-row">
                      <strong>{item.title}</strong>
                      <span>{item.recordClass}</span>
                    </div>
                    <p>{item.owner} / {item.priority} / {item.status} / {item.id}</p>
                    <span>stage: {item.recordClass}; lane: {item.laneSubclass}; receipt: {item.promotionReceiptRequired}</span>
                  </article>
                )) : (
                  <article className="document-list__item">
                    <strong>No active work projected</strong>
                    <p>ARDA loaded the queue surfaces but found no current packet or recent task records for this lane.</p>
                  </article>
                )}
              </div>
            </section>

            <section className="arda-command-console__panel" aria-label="Hermes message stream">
              <div className="module-subtitle"><Activity size={14} /> Hermes Stream</div>
              <div className="document-list compact">
                {commandConsole.messages.length > 0 ? commandConsole.messages.map((message) => (
                  <article className="document-list__item" key={message.id}>
                    <div className="document-list__title-row">
                      <strong>{message.actor}</strong>
                      <span>{message.intent}</span>
                    </div>
                    <p>{message.body}</p>
                    <span>{message.source} / {message.timestamp}</span>
                  </article>
                )) : (
                  <article className="document-list__item">
                    <strong>No Hermes messages loaded</strong>
                    <p>Expected `data/hermes/messages.jsonl` for operator-visible communication history.</p>
                  </article>
                )}
              </div>
            </section>

            <section className="arda-command-console__panel" aria-label="Receipt and evidence stream">
              <div className="module-subtitle"><BookOpenText size={14} /> Receipt Stream</div>
              <div className="document-list compact">
                {commandConsole.receipts.length > 0 ? commandConsole.receipts.map((receipt) => (
                  <article className="document-list__item" key={receipt.id}>
                    <div className="document-list__title-row">
                      <strong>{receipt.task}</strong>
                      <span>{receipt.status}</span>
                    </div>
                    <p>{receipt.summary}</p>
                    <span>{receipt.source} / {receipt.timestamp}</span>
                  </article>
                )) : (
                  <article className="document-list__item">
                    <strong>No receipts loaded</strong>
                    <p>Expected Hermes gateway or Flywheel dispatch receipts for evidence-backed closeout.</p>
                  </article>
                )}
              </div>
            </section>

            <section className="arda-command-console__panel" aria-label="Agent conversation viewer">
              <div className="module-subtitle"><Shield size={14} /> Agent Conversations</div>
              <div className="document-list compact">
                {commandConsole.conversations.length > 0 ? commandConsole.conversations.map((conversation) => (
                  <article className="document-list__item" key={conversation.id}>
                    <div className="document-list__title-row">
                      <strong>{conversation.topic}</strong>
                      <span>{conversation.messageClass}</span>
                    </div>
                    <p>{conversation.summary}</p>
                    <span>{conversation.speaker} / {conversation.risk} / {conversation.timestamp}</span>
                  </article>
                )) : (
                  <article className="document-list__item">
                    <strong>No agent conversations loaded</strong>
                    <p>Expected `data/council/agent_conversations.jsonl` for watchable deliberation, decisions, receipts, and questions.</p>
                  </article>
                )}
              </div>
            </section>

            <section className="arda-command-console__panel" aria-label="Scout request and finding lane">
              <div className="module-subtitle"><BookOpenText size={14} /> Scout Lane</div>
              <div className="document-list compact">
                {commandConsole.scoutItems.length > 0 ? commandConsole.scoutItems.map((item) => (
                  <article className="document-list__item" key={item.id}>
                    <div className="document-list__title-row">
                      <strong>{item.question}</strong>
                      <span>{item.status}</span>
                    </div>
                    <p>{item.kind} / {item.sourcePolicy}</p>
                    <span>{item.requester} / {item.timestamp}</span>
                  </article>
                )) : (
                  <article className="document-list__item">
                    <strong>No scout records loaded</strong>
                    <p>Expected scout requests, scout findings, or scout runtime projection from ATHENA.</p>
                  </article>
                )}
              </div>
            </section>

            <section className="arda-command-console__panel" aria-label="Plan implementation gaps">
              <div className="module-subtitle"><Shield size={14} /> Plan Gaps</div>
              <div className="document-list compact">
                {commandConsole.gaps.map((gap) => (
                  <article className="document-list__item" key={gap.title}>
                    <strong>{gap.title}</strong>
                    <p>{gap.detail}</p>
                  </article>
                ))}
              </div>
            </section>
          </div>
        </section>
      ) : null}

      {refreshActions.length > 0 ? (
        <section className="document-list compact operating-surface-refresh-actions" aria-label="Refresh action flow">
          <header className="document-list__section-heading">
            <strong>Refresh Action Flow</strong>
            <span>{refreshActions.length} action-bus backed refreshes</span>
          </header>
          {refreshActions.map(({ status, descriptor }) => (
            <article className="document-list__item" key={`refresh-${status.id}`}>
              <div className="document-list__title-row">
                <strong>{status.name}</strong>
                <span>{descriptor?.riskLevel ?? 'unknown'} / {status.currentStatus}</span>
              </div>
              <p>{status.purpose}</p>
              <dl className="operating-surface-capability-contract">
                <div><dt>Gate</dt><dd>{status.governanceGate}</dd></div>
                <div><dt>Receipt</dt><dd>{status.receiptPath}</dd></div>
                <div><dt>Last run</dt><dd>{status.lastRun}</dd></div>
              </dl>
              <button
                type="button"
                className="refresh-button"
                disabled={!onRunRefreshAction || actionBusyId === status.id}
                onClick={() => onRunRefreshAction?.(status.id)}
              >
                {actionBusyId === status.id ? 'Running...' : `Run ${status.name}`}
              </button>
            </article>
          ))}
          {actionMessage ? <div className="text-[11px] text-[#b8c4d4]">{actionMessage}</div> : null}
        </section>
      ) : null}

      <div className="operating-surface-grid">
        {reports.map((report) => {
          const Icon = laneIcons[report.lane]
          return (
            <article className={`operating-surface-lane operating-surface-lane--${report.status}`} key={report.lane}>
              <header className="operating-surface-lane__header">
                <div className="module-subtitle">
                  <Icon size={14} />
                  {report.lane}
                </div>
                <span className={`systems-chip systems-chip--${report.status === 'ready' ? 'good' : report.status === 'partial' ? 'muted' : 'warn'}`}>
                  {report.status}
                </span>
              </header>
              <p><strong>Current:</strong> {report.current}</p>
              <p><strong>Gap:</strong> {report.gap}</p>
              <p><strong>Next:</strong> {report.next}</p>
              <div className="token-cloud" aria-label={`${report.lane} evidence`}>
                {report.evidence.map((item) => (
                  <span className="token-chip" key={`${report.lane}-${item}`}>{item}</span>
                ))}
              </div>
            </article>
          )
        })}
      </div>

      <section className="document-list compact operating-surface-provenance" aria-label="Lane provenance matrix">
        <header className="document-list__section-heading">
          <strong>Lane Provenance Matrix</strong>
          <span>{sourceCoverage?.label ?? 'source map status unknown'}</span>
        </header>
        {reports.map((report) => (
          <article className="document-list__item" key={`provenance-${report.lane}`}>
            <div className="document-list__title-row">
              <strong>{report.lane} / {report.status}</strong>
              <span>{report.evidence.length} evidence {report.evidence.length === 1 ? 'link' : 'links'}</span>
            </div>
            <p>{report.next}</p>
          </article>
        ))}
      </section>

      <div className="document-list compact operating-surface-actions" aria-label="Capability execution contract">
        <header className="document-list__section-heading">
          <strong>Capability Execution Contract</strong>
          <span>
            {capabilityStatuses.length} capabilities / {capabilityStatusSummary.succeeded ?? 0} succeeded / {capabilityStatusSummary.blocked ?? 0} gated
          </span>
        </header>
        {capabilityStatuses.map((action) => (
          <article className="document-list__item" key={action.id}>
            <div className="document-list__title-row">
              <strong>{action.name}</strong>
              <span>{action.ownerSystem} / {action.currentStatus}</span>
            </div>
            <p>{action.purpose}</p>
            <dl className="operating-surface-capability-contract">
              <div><dt>Executor</dt><dd>{action.executor}</dd></div>
              <div><dt>Schedule</dt><dd>{action.scheduleState}</dd></div>
              <div><dt>Last run</dt><dd>{action.lastRun}</dd></div>
              <div><dt>Next run</dt><dd>{action.nextRun}</dd></div>
              <div><dt>Manual</dt><dd>{action.manualRunAvailable ? 'available' : 'not available'}</dd></div>
              <div><dt>Dry run</dt><dd>{action.dryRunAvailable ? 'available' : 'not available'}</dd></div>
              <div><dt>Gate</dt><dd>{action.governanceGate}</dd></div>
              <div><dt>Result</dt><dd>{action.resultPath}</dd></div>
              <div><dt>Receipt</dt><dd>{action.receiptPath}</dd></div>
              <div><dt>Failure</dt><dd>{action.failureReason}</dd></div>
            </dl>
            <div className="token-cloud" aria-label={`${action.name} related evidence`}>
              {action.relatedEvidence.map((item) => (
                <span className="token-chip" key={`${action.id}-${item}`}>{item}</span>
              ))}
            </div>
          </article>
        ))}
      </div>
    </ModuleCard>
  )
}
