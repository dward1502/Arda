// sigil: REPAIR
import { primarySigilForSource } from '../../../../lib/soterionRender'

export interface QueueStatusSplit {
  ready: number
  pending: number
  inProgress: number
  blocked: number
}

export interface OperatorCockpitSurface {
  queue: {
    openTotal: number
    deliveredTotal?: number
    items: Array<{ id: string; title: string; owner: string; status: string; priority: string; recordClass?: string }>
    statusSplit: QueueStatusSplit
  }
  humanGates: {
    blockedTotal: number
    items: Array<{ id: string; title: string; status: string; decisionClass: string }>
  }
  warden: {
    effectiveAttention: number
    rawAttention: number
    repeatedNoise: number
    activeRepairFiles: number
    resolvedRepairFiles: number
  }
  chronos: {
    runnerStatus: string
    readyTaskCount: number
    scheduledTaskCount: number
    dueTasks: Array<{ id: string; name: string; cadence: string; owner: string }>
  }
  hermes: {
    gatewayReceiptCount: number
    dispatchReceiptCount: number
    latestReceipts: Array<{ id: string; status: string; task: string; source: string }>
  }
  athena: {
    policyReady: number
    referenceOnly: number
    implementationReady: number
    latest: Array<{ sourceId: string; readiness: string; confidence: string }>
  }
  charon: {
    providerCount: number
    availableProviderCount: number
    blockedProviderCount: number
    cooldownCount: number
    budgetPressureCount: number
    toolContextFloor: number
    warnings: Array<{ providerId: string; state: string; level: string; detail: string }>
  }
  autonomyGate: {
    decision: string
    cleanupPacketCount: number
    externalSourceBlockedCount: number
    reasons: string[]
  }
  storageHygiene: {
    status: string
    cleanupCandidateCount: number
    deletedBytes: number
    warnings: Array<{ label: string; value: string; detail: string }>
  }
  ledgerGaps: Array<{ label: string; path: string; status: string; detail: string }>
}

interface OperatorCockpitPanelProps {
  surface: OperatorCockpitSurface
}

function chipClass(status: string): string {
  if (status === 'ready' || status === 'healthy' || status === 'policy_ready') return 'systems-chip--good'
  if (status === 'missing' || status === 'empty' || status === 'blocked' || status === 'reference_only') return 'systems-chip--warn'
  return 'systems-chip--muted'
}

export default function OperatorCockpitPanel({ surface }: OperatorCockpitPanelProps) {
  const marker = primarySigilForSource('arda')
  const attentionClear = surface.warden.effectiveAttention === 0
  const gapCount = surface.ledgerGaps.length
  const charonWarn = surface.charon.cooldownCount + surface.charon.budgetPressureCount
  const storageWarn = surface.storageHygiene.cleanupCandidateCount > 0 || surface.storageHygiene.status === 'warn'
  const autonomyWarn = surface.autonomyGate.cleanupPacketCount + surface.autonomyGate.externalSourceBlockedCount

  return (
    <section className="systems-panel systems-panel--health">
      <header className="systems-panel__header">
        <div className="systems-panel__eyebrow">{marker} Operator Cockpit</div>
        <h3 className="systems-panel__title">Control Loop Truth</h3>
      </header>
      <div className="systems-kpi-grid">
        <article className={`systems-kpi ${surface.queue.openTotal > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Open Queue</span>
          <strong className="systems-kpi__value">{surface.queue.openTotal}</strong>
        </article>
        <article className={`systems-kpi ${surface.humanGates.blockedTotal > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Human Gates</span>
          <strong className="systems-kpi__value">{surface.humanGates.blockedTotal}</strong>
        </article>
        <article className={`systems-kpi ${attentionClear ? 'systems-kpi--good' : 'systems-kpi--warn'}`}>
          <span className="systems-kpi__label">Warden Pressure</span>
          <strong className="systems-kpi__value">{surface.warden.effectiveAttention}</strong>
        </article>
        <article className={`systems-kpi ${gapCount > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Ledger Gaps</span>
          <strong className="systems-kpi__value">{gapCount}</strong>
        </article>
        <article className={`systems-kpi ${charonWarn > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Charon Pressure</span>
          <strong className="systems-kpi__value">{charonWarn}</strong>
          <span className="systems-kpi__note">floor {surface.charon.toolContextFloor}</span>
        </article>
        <article className={`systems-kpi ${storageWarn ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">HADES Storage</span>
          <strong className="systems-kpi__value">{surface.storageHygiene.cleanupCandidateCount}</strong>
          <span className="systems-kpi__note">{surface.storageHygiene.status}</span>
        </article>
        <article className={`systems-kpi ${autonomyWarn > 0 ? 'systems-kpi--warn' : 'systems-kpi--good'}`}>
          <span className="systems-kpi__label">Autonomy Gate</span>
          <strong className="systems-kpi__value">{autonomyWarn}</strong>
          <span className="systems-kpi__note">{surface.autonomyGate.decision}</span>
        </article>
      </div>

      <div className="systems-chip-cloud">
        <span className={`systems-chip ${surface.queue.statusSplit.ready > 0 ? 'systems-chip--good' : 'systems-chip--warn'}`}>
          queue ready {surface.queue.statusSplit.ready}
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
        <span className={`systems-chip ${attentionClear ? 'systems-chip--good' : 'systems-chip--warn'}`}>
          warden effective {surface.warden.effectiveAttention}
        </span>
        <span className="systems-chip systems-chip--muted">raw {surface.warden.rawAttention}</span>
        <span className="systems-chip systems-chip--muted">noise {surface.warden.repeatedNoise}</span>
        <span className="systems-chip systems-chip--muted">chronos {surface.chronos.runnerStatus}</span>
        <span className="systems-chip systems-chip--muted">hermes receipts {surface.hermes.gatewayReceiptCount}</span>
        <span className="systems-chip systems-chip--muted">athena policy {surface.athena.policyReady}</span>
        <span className="systems-chip systems-chip--muted">athena reference {surface.athena.referenceOnly}</span>
        <span className={`systems-chip ${charonWarn > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          charon available {surface.charon.availableProviderCount}/{surface.charon.providerCount}
        </span>
        <span className={`systems-chip ${surface.charon.blockedProviderCount > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          charon blocked {surface.charon.blockedProviderCount}
        </span>
        <span className={`systems-chip ${storageWarn ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          storage {surface.storageHygiene.status}
        </span>
        <span className={`systems-chip ${autonomyWarn > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>
          autonomy blockers {autonomyWarn}
        </span>
      </div>

      <div className="document-list compact">
        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>Open work</strong>
            <span className="systems-chip systems-chip--muted">{surface.queue.openTotal}</span>
          </div>
          {surface.queue.items.length > 0 ? surface.queue.items.map((item) => (
            <p key={item.id}>{item.title} | {item.owner} | {item.status} | {item.priority}</p>
          )) : <p>No open queue items loaded from the canonical task ledger.</p>}
        </article>

        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>Human-gated decisions</strong>
            <span className="systems-chip systems-chip--warn">{surface.humanGates.blockedTotal}</span>
          </div>
          {surface.humanGates.items.length > 0 ? surface.humanGates.items.map((item) => (
            <p key={item.id}>{item.title} | {item.decisionClass} | {item.status}</p>
          )) : <p>No blocked or pending human gates in the loaded review surfaces.</p>}
        </article>

        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>Chronos scheduled audits</strong>
            <span className="systems-chip systems-chip--muted">{surface.chronos.readyTaskCount}/{surface.chronos.scheduledTaskCount}</span>
          </div>
          {surface.chronos.dueTasks.length > 0 ? surface.chronos.dueTasks.map((task) => (
            <p key={task.id}>{task.name} | {task.owner} | {task.cadence}</p>
          )) : <p>No due Chronos audit tasks are present in the loaded status projection.</p>}
        </article>

        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>Hermes receipts</strong>
            <span className="systems-chip systems-chip--muted">{surface.hermes.gatewayReceiptCount + surface.hermes.dispatchReceiptCount}</span>
          </div>
          {surface.hermes.latestReceipts.length > 0 ? surface.hermes.latestReceipts.map((receipt) => (
            <p key={receipt.id}>{receipt.source} | {receipt.status} | {receipt.task}</p>
          )) : <p>No Hermes gateway or dispatch receipts are loaded.</p>}
        </article>

        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>Charon routing pressure</strong>
            <span className={`systems-chip ${charonWarn > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>
              {surface.charon.availableProviderCount}/{surface.charon.providerCount}
            </span>
          </div>
          {surface.charon.warnings.length > 0 ? surface.charon.warnings.map((warning) => (
            <p key={`${warning.providerId}-${warning.state}`}>{warning.providerId} | {warning.state} | {warning.level} | {warning.detail}</p>
          )) : <p>No Charon cooldown or budget pressure warnings in the loaded router projection.</p>}
        </article>

        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>Autonomy gate</strong>
            <span className={`systems-chip ${autonomyWarn > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>
              {surface.autonomyGate.decision}
            </span>
          </div>
          <p>HADES packets {surface.autonomyGate.cleanupPacketCount} | external sources {surface.autonomyGate.externalSourceBlockedCount}</p>
          {surface.autonomyGate.reasons.length > 0 ? surface.autonomyGate.reasons.map((reason) => (
            <p key={reason}>{reason}</p>
          )) : <p>No autonomy gate blockers are present in the loaded autopilot state.</p>}
        </article>

        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>HADES storage hygiene</strong>
            <span className={`systems-chip ${storageWarn ? 'systems-chip--warn' : 'systems-chip--good'}`}>
              {surface.storageHygiene.status}
            </span>
          </div>
          {surface.storageHygiene.warnings.length > 0 ? surface.storageHygiene.warnings.map((warning) => (
            <p key={warning.label}>{warning.label} | {warning.value} | {warning.detail}</p>
          )) : <p>No HADES storage warnings in the loaded hygiene projection.</p>}
        </article>

        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>Athena readiness</strong>
            <span className="systems-chip systems-chip--muted">{surface.athena.policyReady}/{surface.athena.referenceOnly}</span>
          </div>
          {surface.athena.latest.length > 0 ? surface.athena.latest.map((item) => (
            <p key={item.sourceId}>{item.sourceId} | {item.readiness} | confidence {item.confidence}</p>
          )) : <p>No ATHENA policy-readiness records are loaded.</p>}
        </article>

        <article className="document-list__item">
          <div className="document-list__title-row">
            <strong>Ledger gaps</strong>
            <span className={`systems-chip ${gapCount > 0 ? 'systems-chip--warn' : 'systems-chip--good'}`}>{gapCount}</span>
          </div>
          {gapCount > 0 ? surface.ledgerGaps.map((gap) => (
            <p key={gap.path}>
              {gap.label} | <span className={`systems-chip ${chipClass(gap.status)}`}>{gap.status}</span> | {gap.detail}
            </p>
          )) : <p>Required control-loop ledgers are present and non-empty.</p>}
        </article>
      </div>
    </section>
  )
}
