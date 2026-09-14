// sigil: REPAIR
import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import OperatingSurfacePlanModule, {
  type CommandConsoleSurface,
  type LiveRuntimeChannelEvidence,
  type OperatingSurfaceLaneReport,
} from './OperatingSurfacePlanModule'
import { getSystemActionCapabilityStatuses, getSystemActionDescriptors } from '../../../lib/systemActionBus'

const liveRuntime: LiveRuntimeChannelEvidence = {
  channel: 'arda://hud-pulse',
  source: 'tauri',
  status: 'healthy',
  sequence: 7,
  lastEventIso: '2026-05-27T12:00:05.000Z',
  durableProjection: 'ArdaBundle projection still reloads from core/state/arda_snapshot.json',
}

const laneReports: OperatingSurfaceLaneReport[] = [
  {
    lane: 'Knowledge',
    current: 'ATHENA digest and Mnemosyne continuity are visible through reused surfaces.',
    gap: 'No dedicated knowledge lane split yet.',
    next: 'Split a focused knowledge evidence surface after provenance contracts stabilize.',
    evidence: ['data/athena/digest.jsonl', 'core/state/knowledge_triage_registry.jsonl'],
    status: 'partial',
  },
  {
    lane: 'Evidence',
    current: 'Source map and system evidence are visible through operating surface drilldowns.',
    gap: 'Evidence lane still needs a compact provenance/freshness rollup.',
    next: 'Expose lane-level evidence counts and source coverage on the first screen.',
    evidence: ['ARDA_CONTRACTS_MANIFEST.md'],
    status: 'gap',
  },
]

const commandConsole: CommandConsoleSurface = {
  metrics: [
    { label: 'packets', value: '2', tone: 'good' },
    { label: 'council', value: '1', tone: 'good' },
    { label: 'scout', value: '1', tone: 'good' },
  ],
  lanes: [
    { title: 'Flywheel', value: '1 ready', detail: '2 packets / 1 blocked', status: 'partial' },
    { title: 'Scout', value: '1 records', detail: 'runtime projection loaded', status: 'partial' },
  ],
  workItems: [
    {
      id: 'packet-001',
      title: 'ARDA command console projection',
      owner: 'prometheus',
      status: 'ready',
      priority: 'high',
      recordClass: 'proposal',
      laneSubclass: 'flywheel_packet',
      promotionReceiptRequired: 'flywheel_plan_packet_readiness_receipt',
    },
  ],
  messages: [
    {
      id: 'msg-001',
      source: 'data/hermes/messages.jsonl',
      actor: 'hermes',
      intent: 'operator_update',
      body: 'Command console evidence available locally.',
      timestamp: '2026-05-31T12:30:00Z',
    },
  ],
  receipts: [
    {
      id: 'receipt-001',
      source: 'hermes gateway',
      status: 'recorded',
      task: 'tsk_arda_console',
      summary: 'Local projection rendered without external sends.',
      timestamp: '2026-05-31T12:31:00Z',
    },
  ],
  conversations: [
    {
      id: 'conversation-001',
      topic: 'Agent conversation viewer',
      speaker: 'council',
      messageClass: 'proposal',
      summary: 'Expose deliberation records as operator-visible evidence.',
      risk: 'read_only',
      timestamp: '2026-05-31T12:32:00Z',
    },
  ],
  scoutItems: [
    {
      id: 'scout-001',
      kind: 'request',
      question: 'Which local surfaces should ARDA read?',
      requester: 'prometheus',
      status: 'requested',
      sourcePolicy: 'local_only',
      timestamp: '2026-05-31T12:33:00Z',
    },
  ],
  gaps: [
    {
      title: 'Scout research lane',
      detail: '1 requests and 0 findings loaded.',
    },
  ],
}

describe('OperatingSurfacePlanModule', () => {
  it('surfaces live runtime channel evidence without replacing durable projection evidence', () => {
    render(
      <OperatingSurfacePlanModule
        reports={[]}
        actionDescriptors={[]}
        capabilityStatuses={[]}
        liveRuntime={liveRuntime}
      />,
    )

    expect(screen.getAllByText('Live Runtime')).toHaveLength(2)
    expect(screen.getAllByText('tauri / healthy')).toHaveLength(2)
    expect(screen.getAllByText('seq 7')).toHaveLength(2)
    expect(screen.getByText('2026-05-27T12:00:05.000Z')).toBeTruthy()
    expect(screen.getByText('ArdaBundle projection still reloads from core/state/arda_snapshot.json')).toBeTruthy()
  })

  it('renders a compact lane provenance matrix for knowledge and evidence gaps', () => {
    render(
      <OperatingSurfacePlanModule
        reports={laneReports}
        actionDescriptors={[]}
        capabilityStatuses={[]}
        liveRuntime={null}
        sourceCoverage={{ status: 'partial', label: 'source map partial', missingCount: 2 }}
      />,
    )

    const provenance = screen.getByLabelText('Lane provenance matrix')

    expect(within(provenance).getByText('Lane Provenance Matrix')).toBeTruthy()
    expect(within(provenance).getByText('source map partial')).toBeTruthy()
    expect(within(provenance).getByText(/Knowledge\s*\/\s*partial/)).toBeTruthy()
    expect(within(provenance).getByText(/Evidence\s*\/\s*gap/)).toBeTruthy()
    expect(within(provenance).getByText(/2\s+evidence\s+links/)).toBeTruthy()
    expect(within(provenance).getByText(/1\s+evidence\s+link/)).toBeTruthy()
    expect(within(provenance).getByText('Split a focused knowledge evidence surface after provenance contracts stabilize.')).toBeTruthy()
  })

  it('renders remote confidence as a safe local runtime projection without Discord side effects', () => {
    render(
      <OperatingSurfacePlanModule
        reports={[]}
        actionDescriptors={[]}
        capabilityStatuses={[]}
        remoteConfidenceSnapshot={{
          schema_version: 'arda.remote_confidence_snapshot.v1',
          generated_at_utc: '2026-05-30T00:00:00.000Z',
          mode: 'local_runtime_published',
          status: 'attention_required',
          side_effect_policy: {
            writes_generated_state: true,
            external_messages_sent: false,
            service_restart: false,
            credential_change: false,
          },
          arda_hud: {
            projection_mode: 'local_runtime_state_file',
            target_state_path: 'core/state/remote_confidence_snapshot.json',
          },
        }}
      />,
    )

    const remoteConfidence = screen.getByLabelText('Remote confidence local projection')

    expect(within(remoteConfidence).getByText('Remote Confidence')).toBeTruthy()
    expect(within(remoteConfidence).getByText('attention_required')).toBeTruthy()
    expect(within(remoteConfidence).getByText('local_runtime_published')).toBeTruthy()
    expect(within(remoteConfidence).getByText('core/state/remote_confidence_snapshot.json')).toBeTruthy()
    expect(within(remoteConfidence).getByText('no external Discord send')).toBeTruthy()
    expect(within(remoteConfidence).getByText('no service restart')).toBeTruthy()
    expect(within(remoteConfidence).getByText('no credential change')).toBeTruthy()
  })

  it('renders safe-local work-cycle preflight without adding a rail or forking autonomy logic', () => {
    render(
      <OperatingSurfacePlanModule
        reports={[]}
        actionDescriptors={[]}
        capabilityStatuses={[]}
        safeLocalWorkCyclePreflight={{
          schema_version: 'arda.safe_local_work_cycle_preflight.v1',
          checked_at_utc: '2026-05-31T09:49:35Z',
          mode: 'safe_local_preflight_report',
          candidate_summary: {
            total_count: 12,
            safe_local_count: 9,
            human_gated_count: 1,
          },
          side_effect_policy: {
            writes_local_report: true,
            read_only_intake: true,
            external_messages_sent: false,
            service_restart: false,
            credential_change: false,
            destructive_operations: false,
            mutates_task_status: false,
            live_discord_validation: 'human_gated_separate',
          },
          arda_hud: {
            projection_mode: 'local_report_file',
            new_rail_required: false,
            forks_autonomy_logic: false,
          },
        }}
      />,
    )

    const preflight = screen.getByLabelText('Safe-local work-cycle preflight')

    expect(within(preflight).getByText('Safe-Local Work Cycle')).toBeTruthy()
    expect(within(preflight).getByText('9 safe / 12 total')).toBeTruthy()
    expect(within(preflight).getByText('1 human-gated')).toBeTruthy()
    expect(within(preflight).getByText('local_report_file')).toBeTruthy()
    expect(within(preflight).getByText('no external Discord send')).toBeTruthy()
    expect(within(preflight).getByText('does not mutate task status')).toBeTruthy()
    expect(within(preflight).getByText('no new rail required')).toBeTruthy()
    expect(within(preflight).getByText('does not fork autonomy logic')).toBeTruthy()
    expect(within(preflight).getByText('live Discord validation human-gated separately')).toBeTruthy()
  })

  it('renders the primary command console with work, message, receipt, conversation, scout, and gap lanes', () => {
    render(
      <OperatingSurfacePlanModule
        reports={[]}
        actionDescriptors={[]}
        capabilityStatuses={[]}
        commandConsole={commandConsole}
      />,
    )

    const consoleSurface = screen.getByLabelText('ARDA command console')

    expect(within(consoleSurface).getByText('Primary Console')).toBeTruthy()
    expect(within(consoleSurface).getByText('Work, messages, receipts, and gaps')).toBeTruthy()
    expect(within(consoleSurface).getByText('packets: 2')).toBeTruthy()
    expect(within(consoleSurface).getByText('council: 1')).toBeTruthy()
    expect(within(consoleSurface).getByText('scout: 1')).toBeTruthy()
    expect(within(consoleSurface).getByText('ARDA command console projection')).toBeTruthy()
    expect(within(consoleSurface).getByText('prometheus / high / ready / packet-001')).toBeTruthy()
    expect(within(consoleSurface).getByText('stage: proposal; lane: flywheel_packet; receipt: flywheel_plan_packet_readiness_receipt')).toBeTruthy()
    expect(within(consoleSurface).getByText('Command console evidence available locally.')).toBeTruthy()
    expect(within(consoleSurface).getByText('Local projection rendered without external sends.')).toBeTruthy()
    expect(within(consoleSurface).getByText('Expose deliberation records as operator-visible evidence.')).toBeTruthy()
    expect(within(consoleSurface).getByText('Which local surfaces should ARDA read?')).toBeTruthy()
    expect(within(consoleSurface).getByText('1 requests and 0 findings loaded.')).toBeTruthy()
  })

  it('renders the full callable capability contract instead of truncating after the first registered actions', () => {
    const capabilityStatuses = getSystemActionCapabilityStatuses({
      chronosRuntime: {
        generated_at_utc: '2026-05-27T12:00:00Z',
        status: 'baseline_active',
      },
      setupConsoleReadiness: {
        generated_at_utc: '2026-05-27T12:07:00Z',
        gate_status: 'pass',
      },
      athenaRuntime: {
        generated_at_utc: '2026-05-27T12:08:00Z',
        status: 'ready',
      },
    })

    render(
      <OperatingSurfacePlanModule
        reports={[]}
        actionDescriptors={[]}
        capabilityStatuses={capabilityStatuses}
      />,
    )

    const capabilities = screen.getByLabelText('Capability execution contract')

    expect(within(capabilities).getByText('Capability Execution Contract')).toBeTruthy()
    expect(within(capabilities).getByText(new RegExp(`${capabilityStatuses.length}\\s+capabilities`))).toBeTruthy()
    expect(within(capabilities).getByText('Run CHRONOS Provider Checks')).toBeTruthy()
    expect(within(capabilities).getByText('Run Setup Repair Flow')).toBeTruthy()
    expect(within(capabilities).getByText('Promote Policy-Ready Knowledge')).toBeTruthy()
  })

  it('exposes only read-only or dry-run refreshes as explicit action-flow buttons', () => {
    const onRunRefreshAction = vi.fn()
    const actionDescriptors = getSystemActionDescriptors()
    const capabilityStatuses = getSystemActionCapabilityStatuses({
      providerIntelligence: {
        generated_at_utc: '2026-05-27T12:00:00Z',
        providers_total: 4,
      },
      queueSummary: {
        generated_at_utc: '2026-05-27T12:01:00Z',
      },
    })

    render(
      <OperatingSurfacePlanModule
        reports={[]}
        actionDescriptors={actionDescriptors}
        capabilityStatuses={capabilityStatuses}
        onRunRefreshAction={onRunRefreshAction}
      />,
    )

    const refreshFlow = screen.getByLabelText('Refresh action flow')

    expect(within(refreshFlow).getByText('Refresh Action Flow')).toBeTruthy()
    expect(within(refreshFlow).getByRole('button', { name: 'Run Refresh Provider Intelligence' })).toBeTruthy()
    expect(within(refreshFlow).getByRole('button', { name: 'Run Preview Queue Cleanup' })).toBeTruthy()
    expect(within(refreshFlow).queryByRole('button', { name: 'Run Promote Policy-Ready Knowledge' })).toBeNull()

    fireEvent.click(within(refreshFlow).getByRole('button', { name: 'Run Refresh Provider Intelligence' }))
    expect(onRunRefreshAction).toHaveBeenCalledWith('charon.refresh_provider_intelligence')
  })
})
