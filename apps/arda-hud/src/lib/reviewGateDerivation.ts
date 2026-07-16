import { type ArdaBundle, type JsonRecord} from "./ardaSource"
import { type ReviewGateItem, type CommandConsoleSurface, type ArandurQueueWriteRequest } from "../components/arda/types"
import { asRecord, asArray, getString, getNumber, getBoolean, getTimestamp } from "./ardaSurfaces"

export function getCommandConsoleSurface(bundle: ArdaBundle, reviewGateItems: ReviewGateItem[]): CommandConsoleSurface {
  const projectTasks = asRecord(bundle.queueSummary?.project_tasks)
  const recentTasks = asArray(projectTasks?.recent)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)
  const queueFederationSummary = asRecord(bundle.queueFederation?.summary)
  const queueFederationSources = asArray(bundle.queueFederation?.sources)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)
  const canonicalQueueContract = queueFederationSources.find((source) => getString(source.id, '') === 'canonical_project_tasks')
  const canonicalRecordClass = getString(canonicalQueueContract?.default_record_class, 'execution_attempt')
  const canonicalLaneSubclass = getString(canonicalQueueContract?.lane_subclass, 'canonical_task')
  const canonicalReceipt = getString(canonicalQueueContract?.promotion_receipt_required, 'core/projects/tasks/queue.jsonl append-only closeout')
  const flywheelSummary = asRecord(bundle.flywheelPacketRuntime?.summary)
  const flywheelPackets = asArray(bundle.flywheelPacketRuntime?.packets)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)
  const packetTotal = getNumber(flywheelSummary?.packet_total, flywheelPackets.length)
  const readyTotal = getNumber(flywheelSummary?.ready_total, 0)
  const blockedTotal = getNumber(flywheelSummary?.blocked_total, 0)
  const l3Status = asRecord(bundle.l3ReadinessProjection?.status)
  const l3Flywheel = asRecord(bundle.l3ReadinessProjection?.flywheel)
  const l3ProjectionPolicy = asRecord(bundle.l3ReadinessProjection?.projection_policy)
  const l3Level = getString(l3Status?.level, 'missing')
  const l3BoundedReady = getBoolean(l3Status?.bounded_mutation_ready, false)
  const l3BroadMutationAuthorized = getBoolean(l3Status?.broad_mutation_authorized, false)
  const l3NextReadyPacket = asRecord(l3Flywheel?.next_ready_packet)
  const completedTotal = getNumber(asRecord(projectTasks?.counts_by_status)?.completed, 0)
  const scoutTotal = bundle.scoutRequests.length + bundle.scoutFindings.length

  const workItems = (flywheelPackets.length > 0 ? flywheelPackets : recentTasks.slice(-8))
    .slice(-8)
    .reverse()
    .map((item, index) => {
      const meta = asRecord(item.meta)
      return {
        id: getString(item.id ?? item.task_id ?? meta?.packet_id, `work-${index + 1}`),
        title: getString(item.title ?? item.summary ?? meta?.plan, 'Untitled work item'),
        owner: getString(item.owner ?? meta?.owner, 'unknown'),
        status: getString(item.readiness ?? item.status ?? item.result, 'unknown'),
        priority: getString(item.priority ?? meta?.risk, 'normal'),
        recordClass: getString(item.record_class ?? meta?.record_class, canonicalRecordClass),
        laneSubclass: getString(item.lane_subclass ?? meta?.lane_subclass, canonicalLaneSubclass),
        promotionReceiptRequired: getString(item.promotion_receipt_required ?? meta?.promotion_receipt_required, canonicalReceipt),
      }
    })

  const messages = bundle.hermesMessages
    .slice(-6)
    .reverse()
    .map((message, index) => {
      const classification = asRecord(message.classification)
      const body = getBoolean(message.content_redacted, false)
        ? '[redacted]'
        : getString(message.content ?? message.summary, 'No message body recorded')
      return {
        id: getString(message.completion_id ?? message.receipt_id ?? message.received_at_utc, `message-${index + 1}`),
        source: getString(message.source ?? message.direction, 'hermes'),
        actor: getString(message.sender ?? message.agent ?? message.channel, 'unknown'),
        intent: getString(classification?.intent ?? message.status, 'message'),
        body,
        timestamp: getTimestamp(message),
      }
    })

  const receiptRecords = [
    ...bundle.flywheelDispatchReceipts.map((receipt) => ({ ...receipt, __source: 'flywheel dispatch' })),
    ...bundle.hermesAgentGatewayReceipts.map((receipt) => ({ ...receipt, __source: 'hermes gateway' })),
  ] as Array<JsonRecord & { __source: string }>
  const receipts = receiptRecords
    .sort((left, right) => getTimestamp(left).localeCompare(getTimestamp(right)))
    .slice(-6)
    .reverse()
    .map((receipt, index) => ({
      id: getString(receipt.receipt_id ?? receipt.task_id ?? receipt.ts_utc, `receipt-${index + 1}`),
      source: getString(receipt.__source, 'receipt'),
      status: getString(receipt.status ?? (getBoolean(receipt.dry_run, false) ? 'dry_run' : 'recorded'), 'recorded'),
      task: getString(receipt.task_ref ?? receipt.task_id ?? receipt.packet_id, 'unlinked task'),
      summary: getString(receipt.summary ?? receipt.next_action ?? receipt.prompt_sha1_12, 'Receipt recorded without summary text.'),
      timestamp: getTimestamp(receipt),
    }))

  const conversations = bundle.agentConversations
    .slice(-6)
    .reverse()
    .map((conversation, index) => ({
      id: getString(conversation.conversation_id ?? conversation.id ?? conversation.ts_utc, `conversation-${index + 1}`),
      topic: getString(conversation.topic ?? conversation.related_plan ?? conversation.related_task, 'Untitled conversation'),
      speaker: getString(conversation.speaker_agent ?? conversation.agent ?? conversation.speaker, 'unknown agent'),
      messageClass: getString(conversation.message_class ?? conversation.class ?? conversation.actionability, 'observation'),
      summary: getString(conversation.summary ?? conversation.message ?? conversation.content, 'No conversation summary recorded.'),
      risk: getString(conversation.risk_lane ?? conversation.risk, 'unknown risk'),
      timestamp: getTimestamp(conversation),
    }))

  const scoutRequests = bundle.scoutRequests.map((request, index) => ({
    id: getString(request.scout_request_id ?? request.request_id ?? request.id, `scout-request-${index + 1}`),
    kind: 'request',
    question: getString(request.question ?? request.topic ?? request.summary, 'Untitled scout request'),
    requester: getString(request.requester_agent ?? request.requester ?? request.owner, 'unknown requester'),
    status: getString(request.status ?? request.state, 'requested'),
    sourcePolicy: getString(request.allowed_sources ?? request.source_policy, 'source policy unknown'),
    timestamp: getTimestamp(request),
  }))
  const scoutFindings = bundle.scoutFindings.map((finding, index) => ({
    id: getString(finding.scout_finding_id ?? finding.finding_id ?? finding.id, `scout-finding-${index + 1}`),
    kind: 'finding',
    question: getString(finding.question ?? finding.title ?? finding.summary, 'Untitled scout finding'),
    requester: getString(finding.requester_agent ?? finding.source_agent ?? finding.owner, 'athena'),
    status: getString(finding.status ?? finding.result, 'found'),
    sourcePolicy: getString(finding.allowed_sources ?? finding.source_policy ?? finding.source_class, 'sources recorded in finding'),
    timestamp: getTimestamp(finding),
  }))
  const scoutItems = [...scoutRequests, ...scoutFindings]
    .sort((left, right) => left.timestamp.localeCompare(right.timestamp))
    .slice(-6)
    .reverse()

  const gaps = [
    {
      title: 'Agent conversation viewer',
      detail: bundle.agentConversations.length > 0
        ? `${bundle.agentConversations.length} council conversation records loaded.`
        : 'Missing data/council/agent_conversations.jsonl projection for active agent conversations.',
    },
    {
      title: 'Scout research lane',
      detail: scoutTotal > 0
        ? `${bundle.scoutRequests.length} requests and ${bundle.scoutFindings.length} findings loaded.`
        : 'Missing data/athena/scout_requests.jsonl, data/athena/scout_findings.jsonl, or core/state/scout_runtime.json projection.',
    },
    {
      title: 'Flywheel packet backlog',
      detail: packetTotal > 0
        ? `${packetTotal} packets projected with ${readyTotal} ready and ${blockedTotal} blocked.`
        : 'core/state/flywheel_packet_runtime.json is loaded, but it currently exposes no packets for the ARDA work lane.',
    },
    {
      title: 'L3 readiness projection',
      detail: bundle.l3ReadinessProjection
        ? `${l3Level}; bounded mutation ${l3BoundedReady ? 'ready' : 'not ready'}; broad mutation ${l3BroadMutationAuthorized ? 'authorized' : 'blocked'}.`
        : 'Missing core/state/l3_readiness_projection.json for ARDA/Hermes L3 operator context.',
    },
  ]

  return {
    metrics: [
      { label: 'L3', value: l3BoundedReady ? 'READY' : 'GATED', tone: l3BoundedReady ? 'good' : 'warn' },
      { label: 'packets', value: `${packetTotal}`, tone: packetTotal > 0 ? 'good' : 'warn' },
      { label: 'ready', value: `${readyTotal}`, tone: readyTotal > 0 ? 'good' : 'muted' },
      { label: 'reviews', value: `${reviewGateItems.length}`, tone: reviewGateItems.length > 0 ? 'warn' : 'muted' },
      { label: 'messages', value: `${bundle.hermesMessages.length}`, tone: bundle.hermesMessages.length > 0 ? 'good' : 'warn' },
      { label: 'receipts', value: `${receipts.length}`, tone: receipts.length > 0 ? 'good' : 'warn' },
      { label: 'council', value: `${conversations.length}`, tone: conversations.length > 0 ? 'good' : 'warn' },
      { label: 'scout', value: `${scoutItems.length}`, tone: scoutItems.length > 0 ? 'good' : 'warn' },
    ],
    lanes: [
      {
        title: 'Stage Contract',
        value: `${getNumber(queueFederationSummary?.sources_total, queueFederationSources.length)} lanes`,
        detail: `${getNumber(queueFederationSummary?.promotion_ready_total, 0)} promotion-ready / ${getNumber(queueFederationSummary?.blocked_total, 0)} blocked; canonical=${canonicalRecordClass}`,
        status: bundle.queueFederation ? 'partial' : 'gap',
      },
      {
        title: 'Flywheel',
        value: `${readyTotal} ready`,
        detail: `${packetTotal} packets / ${blockedTotal} blocked`,
        status: packetTotal > 0 ? 'partial' : 'gap',
      },
      {
        title: 'L3 Readiness',
        value: l3BoundedReady ? 'bounded ready' : 'gated',
        detail: l3NextReadyPacket
          ? `${getString(l3NextReadyPacket.packet_id, 'next')} ${getString(l3NextReadyPacket.readiness, 'ready')}; projection read-only=${getBoolean(l3ProjectionPolicy?.read_only, true) ? 'true' : 'false'}`
          : `${l3Level}; projection missing next packet`,
        status: bundle.l3ReadinessProjection ? 'partial' : 'gap',
      },
      {
        title: 'Queue',
        value: `${recentTasks.length} recent`,
        detail: `${completedTotal} completed in task summary`,
        status: recentTasks.length > 0 ? 'partial' : 'gap',
      },
      {
        title: 'Hermes',
        value: `${messages.length} visible`,
        detail: `${bundle.hermesAgentGatewayReceipts.length} gateway receipts loaded`,
        status: messages.length > 0 ? 'partial' : 'gap',
      },
      {
        title: 'Scout',
        value: `${scoutTotal} records`,
        detail: bundle.scoutRuntime ? 'runtime projection loaded' : 'runtime projection missing',
        status: scoutTotal > 0 || bundle.scoutRuntime ? 'partial' : 'gap',
      },
      {
        title: 'Decisions',
        value: `${reviewGateItems.length} gated`,
        detail: 'Arandur, HADES, and ATHENA review packets',
        status: reviewGateItems.length > 0 ? 'partial' : 'gap',
      },
      {
        title: 'Evidence',
        value: `${receipts.length} recent`,
        detail: 'Hermes gateway and Flywheel dispatch receipts',
        status: receipts.length > 0 ? 'partial' : 'gap',
      },
    ],
    workItems,
    messages,
    receipts,
    conversations,
    scoutItems,
    gaps,
  }
}

export function getPlanShelf(bundle: ArdaBundle): {
  humanPlanRoot: string
  corePlanRoot: string
  plans: Array<{ id: string; title: string; owner: string; openTaskCount: number; humanPlanPath: string; coreQuickRefPath: string }>
} {
  const planMap = asRecord(bundle.planMap)
  const plans = asArray(planMap?.plans)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)
    .map((entry) => ({
      id: getString(entry.id, 'unknown'),
      title: getString(entry.title, 'Untitled Plan'),
      owner: getString(entry.owner, 'unknown'),
      openTaskCount: getNumber(entry.openTaskCount ?? entry.open_task_count, 0),
      humanPlanPath: getString(entry.humanPlanPath ?? entry.human_plan_path, 'human/plans'),
      coreQuickRefPath: getString(entry.coreQuickRefPath ?? entry.core_quick_ref_path, 'core/projects/Plans'),
    }))

  return {
    humanPlanRoot: getString(planMap?.humanPlanRoot ?? planMap?.human_plan_root, 'human/plans'),
    corePlanRoot: getString(planMap?.corePlanRoot ?? planMap?.core_plan_root, 'core/projects/Plans'),
    plans,
  }
}

export function getHumanAugmentationRuntime(bundle: ArdaBundle): {
  summary: Array<{ label: string; value: string }>
  approvals: Array<{ id: string; decisionClass: string; approvers: string; status: string; note: string; commandSignature?: string | null }>
} {
  const runtime = asRecord(bundle.humanAugmentationRuntime)
  const summary = asRecord(runtime?.summary)
  const approvals = asArray(runtime?.approvals)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
    .map((item) => ({
      id: getString(item.approval_id, 'unknown'),
      decisionClass: getString(item.decision_class, 'unknown'),
      approvers: asArray(item.approvers).map((entry) => getString(entry)).filter(Boolean).join(', '),
      status: getString(item.status, 'unknown'),
      note: getString(item.note, 'n/a'),
      commandSignature: typeof item.command_signature === 'string' ? item.command_signature : null,
    }))

  return {
    summary: [
      { label: 'Approved', value: `${getNumber(summary?.approved_total, 0)}` },
      { label: 'Pending', value: `${getNumber(summary?.pending_total, 0)}` },
    ],
    approvals,
  }
}

export function getArandurQueueWriteRequests(bundle: ArdaBundle): ArandurQueueWriteRequest[] {
  const runtime = asRecord(bundle.humanAugmentationRuntime)
  return asArray(runtime?.arandur_queue_write_requests ?? runtime?.queue_write_requests)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
    .map((item) => {
      const requestedEntry = asRecord(item.requested_queue_entry)
      const sourceFingerprints = asRecord(item.source_fingerprints)
      const boundedOutput = asRecord(item.bounded_output)
      const mutationPolicyRecord = asRecord(item.mutation_policy)
      const mutationPolicy = Object.fromEntries(
        Object.entries(mutationPolicyRecord ?? {})
          .map(([key, value]) => [key, getString(value, `${value}`)]),
      )
      return {
        id: getString(item.queue_write_request_id, 'unknown'),
        missionCandidateId: getString(item.source_mission_candidate_id, 'unknown'),
        queueProposalId: getString(item.source_queue_proposal_id, 'unknown'),
        title: getString(requestedEntry?.title, 'Untitled Arandur queue write'),
        scope: getString(requestedEntry?.scope, 'arandur'),
        justification: getString(item.justification, 'No justification provided'),
        createdAtUtc: getString(item.created_at_utc, 'unknown'),
        canonicalQueueSha1: getString(sourceFingerprints?.canonical_queue_sha1, 'unknown'),
        proposalSha1: getString(sourceFingerprints?.mission_queue_proposal_sha1, 'unknown'),
        reviewRequired: getBoolean(item.review_required, true),
        reviewChecklist: asArray(item.operator_write_checklist).map((entry) => getString(entry)).filter(Boolean),
        requiresFutureHumanApproval: getBoolean(boundedOutput?.requires_future_human_approval, true),
        requiresSeparateFutureCanonicalQueueWrite: getBoolean(boundedOutput?.requires_separate_future_canonical_queue_write, true),
        mutationPolicy,
        writePending: getBoolean(item.write_pending, false),
        executionStatus: getString(item.execution_status, getBoolean(item.write_pending, false) ? 'write_pending' : 'legacy_review'),
        canonicalQueueTaskId: typeof item.canonical_queue_task_id === 'string' ? item.canonical_queue_task_id : null,
      }
    })
}

export function getReviewGateItems(bundle: ArdaBundle, queueWriteRequests: ArandurQueueWriteRequest[]): ReviewGateItem[] {
  const queueItems: ReviewGateItem[] = queueWriteRequests.map((request) => ({
    id: request.id,
    kind: 'queue_write',
    title: request.title,
    source: request.scope,
    status: request.executionStatus,
    decisionClass: 'arandur_queue_write',
    evidence: `${request.canonicalQueueSha1},${request.proposalSha1}`,
    summary: request.justification,
    checklist: request.reviewChecklist,
    createdAtUtc: request.createdAtUtc,
  }))

  const recommendationItems: ReviewGateItem[] = bundle.arandurRecommendations.map((entry) => {
    const candidate = asRecord(entry.candidate)
    const sources = asArray(candidate?.sources).map((source) => getString(source)).filter(Boolean)
    return {
      id: getString(entry.recommendation_id, 'unknown'),
      kind: 'recommendation',
      title: getString(candidate?.title, getString(entry.recommended_candidate_id, 'Untitled recommendation')),
      source: getString(entry.source, 'arandur'),
      status: getBoolean(entry.review_required, false) ? 'pending_review' : getString(candidate?.result ?? candidate?.status, 'recorded'),
      decisionClass: 'arandur_recommendation',
      evidence: getString(entry.source_packet, sources.slice(0, 2).join(',')),
      summary: getString(entry.recommended_action, 'Review Arandur recommendation before promotion.'),
      checklist: [
        'confirm source evidence still supports the recommendation',
        'confirm no canonical queue mutation is implied by approval alone',
        'confirm claims remain review-gated until implementation planning',
      ],
      createdAtUtc: getString(entry.ts_utc, ''),
    }
  })

  const missionItems: ReviewGateItem[] = bundle.arandurMissionApprovalRequests.map((entry) => {
    const boundedRecommendation = asRecord(entry.bounded_recommendation)
    const sourceFingerprints = asRecord(entry.source_fingerprints)
    return {
      id: getString(entry.approval_request_id, 'unknown'),
      kind: 'mission_approval',
      title: getString(boundedRecommendation?.title, getString(entry.approval_question, 'Untitled mission approval')),
      source: getString(boundedRecommendation?.scope, 'arandur'),
      status: getBoolean(entry.review_required, false) ? 'pending_review' : 'recorded',
      decisionClass: 'arandur_mission_approval',
      evidence: [
        getString(sourceFingerprints?.canonical_queue_sha1, ''),
        getString(sourceFingerprints?.mission_candidate_sha1, ''),
        getString(sourceFingerprints?.mission_review_sha1, ''),
      ].filter(Boolean).join(','),
      summary: getString(entry.approval_question, getString(entry.justification, 'Review mission approval request.')),
      checklist: asArray(entry.operator_approval_checklist).map((item) => getString(item)).filter(Boolean),
      createdAtUtc: getString(entry.created_at_utc, ''),
    }
  })

  const hadesItems: ReviewGateItem[] = bundle.hadesLifecycleReviewQueue.map((entry) => ({
    id: getString(entry.review_id, getString(entry.finding_id, 'unknown')),
    kind: 'hades_lifecycle',
    title: getString(entry.recommendation, 'HADES lifecycle review'),
    source: getString(entry.path, getString(entry.evidence_path, 'hades')),
    status: getBoolean(entry.review_required, false) ? 'pending_review' : getString(entry.classification, 'recorded'),
    decisionClass: 'hades_lifecycle_review',
    evidence: getString(entry.evidence_path, getString(entry.path, '')),
    summary: `${getString(entry.classification, 'review')} / ${getString(entry.severity, 'unknown')} severity`,
    checklist: asArray(entry.allowed_actions).map((item) => `allowed action: ${getString(item)}`).filter(Boolean),
    createdAtUtc: getString(entry.queued_at_utc, ''),
  }))

  const latestAthenaReadinessBySource = new Map<string, JsonRecord>()
  for (const entry of bundle.athenaPolicyReadiness) {
    const sourceId = getString(entry.source_id, '')
    if (!sourceId) continue
    const previous = latestAthenaReadinessBySource.get(sourceId)
    const previousTs = getString(previous?.ts_utc, '')
    const nextTs = getString(entry.ts_utc, '')
    if (!previous || nextTs.localeCompare(previousTs) >= 0) {
      latestAthenaReadinessBySource.set(sourceId, entry)
    }
  }

  const athenaItems: ReviewGateItem[] = [...latestAthenaReadinessBySource.values()]
    .filter((entry) => getString(entry.policy_readiness, 'reference_only') !== 'implementation_ready')
    .map((entry) => {
      const gate = asRecord(entry.gate)
      const observed = asRecord(gate?.observed)
      const blockers = asArray(gate?.blockers).map((item) => getString(item)).filter(Boolean)
      const sourceId = getString(entry.source_id, 'unknown')
      const confidence = getNumber(observed?.confidence, NaN)
      const confidenceLabel = Number.isFinite(confidence) ? `${Math.round(confidence * 100)}% confidence` : 'confidence unknown'
      const oppositionCoverage = getNumber(observed?.opposition_coverage, NaN)
      const oppositionLabel = Number.isFinite(oppositionCoverage) ? `${oppositionCoverage} opposition sources` : 'opposition coverage unknown'
      const triadPassed = getBoolean(observed?.triad_passed, false)

      return {
        id: sourceId,
        kind: 'athena_policy_readiness',
        title: `ATHENA source ${sourceId}`,
        source: 'athena',
        status: getString(entry.policy_readiness, 'reference_only'),
        decisionClass: 'athena_policy_readiness',
        evidence: blockers.length > 0 ? `${sourceId}:${blockers.join(',')}` : sourceId,
        summary: blockers.length > 0
          ? `Blocked by ${blockers.join(', ')}. Observed ${confidenceLabel}, ${oppositionLabel}, triad ${triadPassed ? 'passed' : 'not passed'}.`
          : `Ready for operator review. Observed ${confidenceLabel}, ${oppositionLabel}.`,
        checklist: [
          'confirm this source remains reference-only until blockers are resolved',
          ...blockers.map((blocker) => `resolve blocker: ${blocker}`),
          'confirm opposition or corroboration work exists before implementation promotion',
        ],
        createdAtUtc: getString(entry.ts_utc, ''),
      }
    })

  return [...queueItems, ...missionItems, ...recommendationItems, ...hadesItems, ...athenaItems]
    .filter((item) => item.id !== 'unknown')
    .sort((left, right) => (right.createdAtUtc ?? '').localeCompare(left.createdAtUtc ?? ''))
}

export function getCeoCouncilRuntime(bundle: ArdaBundle): {
  summary: Array<{ label: string; value: string }>
  sessions: Array<{ id: string; objective: string; loopClass: string; decisionClass: string; outcomeStatus: string }>
  validators: Array<{ label: string; value: string }>
  memoryLanes: Array<{ label: string; value: string }>
} {
  const runtime = asRecord(bundle.ceoCouncilRuntime)
  const summary = asRecord(runtime?.summary)
  const sessions = asArray(runtime?.sessions)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
    .map((item) => ({
      id: getString(item.session_id, 'unknown'),
      objective: getString(item.objective, 'Untitled council session'),
      loopClass: getString(item.loop_class, 'lightweight'),
      decisionClass: getString(item.decision_class, 'unknown'),
      outcomeStatus: getString(item.outcome_status, 'open'),
    }))
  const validatorCounts = asRecord(summary?.validator_invocation_counts)
  const memoryLaneUsage = asRecord(summary?.memory_lane_usage)

  return {
    summary: [
      { label: 'Sessions', value: `${getNumber(summary?.total_sessions, 0)}` },
      { label: 'Triad', value: `${getNumber(summary?.triad_sessions, 0)}` },
      { label: 'Lightweight', value: `${getNumber(summary?.lightweight_sessions, 0)}` },
      { label: 'Escalations', value: `${getNumber(summary?.human_escalations, 0)}` },
      { label: 'Promoted Private', value: `${getNumber(summary?.promoted_private_memory_total, 0)}` },
    ],
    sessions,
    validators: Object.entries(validatorCounts ?? {})
      .map(([label, value]) => ({ label, value: `${getNumber(value, 0)}` }))
      .sort((left, right) => Number(right.value) - Number(left.value))
      .slice(0, 6),
    memoryLanes: Object.entries(memoryLaneUsage ?? {})
      .map(([label, value]) => ({ label, value: `${getNumber(value, 0)}` }))
      .sort((left, right) => Number(right.value) - Number(left.value))
      .slice(0, 6),
  }
}

export function getTaskLifecycleRuntime(bundle: ArdaBundle): {
  summary: Array<{ label: string; value: string }>
  pipeline: string
  disposalCandidates: Array<{ id: string; title: string; owner: string; marker: string; nextPhase: string }>
} {
  const runtime = asRecord(bundle.taskLifecycleRuntime)
  const summary = asRecord(runtime?.summary)
  const contract = asRecord(runtime?.contract)
  const disposalCandidates = asArray(runtime?.disposal_candidates)
    .map((item) => asRecord(item))
    .filter((item): item is JsonRecord => item !== null)
    .map((item) => ({
      id: getString(item.id, 'unknown'),
      title: getString(item.title, 'Untitled task'),
      owner: getString(item.owner, 'unknown'),
      marker: getString(item.disposal_marker, '↝'),
      nextPhase: getString(item.next_phase, 'hades_disposal_review'),
    }))

  return {
    summary: [
      { label: 'Queued', value: `${getNumber(summary?.queued_total, 0)}` },
      { label: 'Active', value: `${getNumber(summary?.active_total, 0)}` },
      { label: 'Completed', value: `${getNumber(summary?.completed_total, 0)}` },
      { label: 'Disposal Review', value: `${getNumber(summary?.disposal_review_total, 0)}` },
      { label: 'Archive Ready', value: `${getNumber(summary?.archive_ready_total, 0)}` },
    ],
    pipeline: getString(contract?.pipeline, 'plan -> task_emission -> task_retrieval -> bounded_execution -> completion_evidence -> hades_disposal_review -> archive_or_retention'),
    disposalCandidates,
  }
}

export function getRuntimeDrift(bundle: ArdaBundle) {
  const runtimeDrift = asRecord(bundle.fleetRuntimeDrift)
  const items = asArray(runtimeDrift?.items)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)
    .map((entry) => {
      const drift = asRecord(entry.drift)
      return {
        nodeId: getString(entry.node_id, 'unknown'),
        displayName: getString(entry.display_name, getString(entry.node_id, 'unknown')),
        providerId: getString(entry.provider_id, ''),
        declaredModel: getString(entry.declared_model, ''),
        declaredContextWindow: Number.isFinite(getNumber(entry.declared_context_window, NaN))
          ? getNumber(entry.declared_context_window, NaN)
          : null,
        charonContextWindow: Number.isFinite(getNumber(entry.charon_context_window, NaN))
          ? getNumber(entry.charon_context_window, NaN)
          : null,
        actualProcessContextWindow: Number.isFinite(getNumber(entry.actual_process_context_window, NaN))
          ? getNumber(entry.actual_process_context_window, NaN)
          : null,
        declaredVsCharon: getBoolean(drift?.declared_vs_charon, false),
        declaredVsLocalProcess: getBoolean(drift?.declared_vs_local_process, false),
        localRuntimeStatus: getString(entry.local_runtime_status, ''),
      }
    })

  return {
    totalNodes: items.length,
    driftedNodes: items.filter((item) => item.declaredVsCharon || item.declaredVsLocalProcess).length,
    items,
  }
}
export function getOperatorRuntimeSurface(bundle: ArdaBundle): JsonRecord | null {
  return asRecord(bundle.operatorRuntimeStatus)
}

