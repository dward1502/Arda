// sigil: REPAIR
import type { SceneZoneDefinition } from '../systems/runtimeTypes'

export type WorldDistrictUrgencyState = 'nominal' | 'attention' | 'stale' | 'blocked' | 'critical' | 'unknown'

export type WorldActionGuard = 'safe' | 'gated' | 'draft_only'

export interface WorldDistrictAction {
  id: string
  label: string
  guard: WorldActionGuard
  target: string
}

export interface WorldDistrictContract {
  districtId: string
  title: string
  ownerAgent: string
  sceneZoneId: string
  domain: string
  sourcePaths: string[]
  provenanceDomainIds: string[]
  urgencyInputs: string[]
  alertTriggers: string[]
  workstationId: string | null
  sourceZoneId: string
  primaryActions: WorldDistrictAction[]
  safeActions: WorldDistrictAction[]
  gatedActions: WorldDistrictAction[]
  visualStateMapping: Record<WorldDistrictUrgencyState, string>
  acceptanceCriteria: string[]
}

const VISUAL_STATE_MAPPING: Record<WorldDistrictUrgencyState, string> = {
  nominal: 'steady cyan district emissive trace attached to district mesh',
  attention: 'gold pulse on district mesh and label',
  stale: 'violet freshness hatch on district mesh and label',
  blocked: 'amber guardhouse lock trace on district mesh and label',
  critical: 'red escalation pulse on district mesh and label',
  unknown: 'dim gray unresolved-source trace on district mesh and label',
}

export const WORLD_DISTRICT_CONTRACTS: WorldDistrictContract[] = [
  {
    districtId: 'district_command',
    title: 'Command',
    ownerAgent: 'PROMETHEUS',
    sceneZoneId: 'sovereign_world',
    domain: 'CEO overview and autonomy readiness',
    sourcePaths: ['core/realm/agents.toml', 'core/state/arda_runtime.json'],
    provenanceDomainIds: ['world:core/realm/agents.toml', 'world:core/state/arda_runtime.json'],
    urgencyInputs: ['autonomy readiness', 'active review pressure', 'world runtime freshness'],
    alertTriggers: ['autonomy blocked', 'stale world projection', 'critical review gate'],
    workstationId: 'sovereign_world-workstation',
    sourceZoneId: 'sovereign_world',
    primaryActions: [{ id: 'open-command-workstation', label: 'Open Command workstation', guard: 'safe', target: 'sovereign_world' }],
    safeActions: [{ id: 'inspect-command-provenance', label: 'Inspect command provenance', guard: 'safe', target: 'source_trust' }],
    gatedActions: [{ id: 'draft-autonomy-packet', label: 'Draft autonomy action packet', guard: 'draft_only', target: 'planning_and_queue' }],
    visualStateMapping: VISUAL_STATE_MAPPING,
    acceptanceCriteria: ['operator can identify command owner', 'operator can inspect stale/blocked command signals'],
  },
  {
    districtId: 'district_knowledge',
    title: 'Knowledge',
    ownerAgent: 'ATHENA/MNEMOSYNE',
    sceneZoneId: 'knowledge_and_reasoning',
    domain: 'Research, memory, and knowledge triage',
    sourcePaths: ['core/state/knowledge_triage_registry.jsonl', 'data/athena'],
    provenanceDomainIds: ['knowledge:core/state/knowledge_triage_registry.jsonl'],
    urgencyInputs: ['policy blockers', 'triage freshness', 'memory continuity'],
    alertTriggers: ['blocked knowledge policy', 'stale triage registry', 'missing recall lane'],
    workstationId: 'knowledge_and_reasoning-workstation',
    sourceZoneId: 'knowledge_and_reasoning',
    primaryActions: [{ id: 'open-knowledge-workstation', label: 'Open Knowledge workstation', guard: 'safe', target: 'knowledge_and_reasoning' }],
    safeActions: [{ id: 'inspect-triage', label: 'Inspect triage registry', guard: 'safe', target: 'knowledge_and_reasoning' }],
    gatedActions: [{ id: 'draft-knowledge-refresh', label: 'Draft ATHENA refresh packet', guard: 'draft_only', target: 'knowledge_and_reasoning' }],
    visualStateMapping: VISUAL_STATE_MAPPING,
    acceptanceCriteria: ['policy blockers surface as blocked', 'triage freshness is visible'],
  },
  {
    districtId: 'district_operations',
    title: 'Operations',
    ownerAgent: 'HADES/APOLLO',
    sceneZoneId: 'planning_and_queue',
    domain: 'Queue, lifecycle execution, and operational cleanup',
    sourcePaths: ['core/projects/tasks/queue.jsonl', 'docs/plans'],
    provenanceDomainIds: ['planning:core/projects/tasks/queue.jsonl'],
    urgencyInputs: ['open queue depth', 'blocked tasks', 'review packets'],
    alertTriggers: ['blocked task count > 0', 'queue pressure high', 'review gate pending'],
    workstationId: 'planning_and_queue-workstation',
    sourceZoneId: 'planning_and_queue',
    primaryActions: [{ id: 'open-queue-workstation', label: 'Open Queue workstation', guard: 'safe', target: 'planning_and_queue' }],
    safeActions: [{ id: 'inspect-queue', label: 'Inspect queue state', guard: 'safe', target: 'planning_and_queue' }],
    gatedActions: [{ id: 'draft-task-pivot', label: 'Draft task pivot packet', guard: 'draft_only', target: 'planning_and_queue' }],
    visualStateMapping: VISUAL_STATE_MAPPING,
    acceptanceCriteria: ['queue pressure lights operations', 'no direct queue mutation from world click'],
  },
  {
    districtId: 'district_communications',
    title: 'Routing & Comms',
    ownerAgent: 'CHARON/HERMES',
    sceneZoneId: 'routing_and_comms',
    domain: 'Provider routing, Hermes delivery, and communication mesh',
    sourcePaths: ['config/charon_providers.toml', 'data/hermes'],
    provenanceDomainIds: ['routing:config/charon_providers.toml'],
    urgencyInputs: ['provider health', 'routing failures', 'delivery backlog'],
    alertTriggers: ['provider unhealthy', 'route unavailable', 'delivery failed'],
    workstationId: 'routing_and_comms-workstation',
    sourceZoneId: 'routing_and_comms',
    primaryActions: [{ id: 'open-routing-workstation', label: 'Open Routing workstation', guard: 'safe', target: 'routing_and_comms' }],
    safeActions: [{ id: 'inspect-routing-health', label: 'Inspect routing health', guard: 'safe', target: 'routing_health' }],
    gatedActions: [{ id: 'draft-provider-reroute', label: 'Draft provider reroute packet', guard: 'draft_only', target: 'routing_and_comms' }],
    visualStateMapping: VISUAL_STATE_MAPPING,
    acceptanceCriteria: ['unhealthy providers raise routing district pressure', 'routing actions remain draft or inspection only'],
  },
  {
    districtId: 'district_governance',
    title: 'Governance Guardhouse',
    ownerAgent: 'ORACLE',
    sceneZoneId: 'governance_guardhouse',
    domain: 'Governance, review gates, and autonomy guardrails',
    sourcePaths: ['core/realm/arda.toml', 'docs/SAFETY_MODEL.md'],
    provenanceDomainIds: ['governance:core/realm/arda.toml'],
    urgencyInputs: ['review gates', 'governance readiness', 'safety blockers'],
    alertTriggers: ['review gate blocked', 'triad required', 'autonomy unsafe'],
    workstationId: 'governance_guardhouse-workstation',
    sourceZoneId: 'governance_guardhouse',
    primaryActions: [{ id: 'open-governance-workstation', label: 'Open Governance workstation', guard: 'safe', target: 'governance_guardhouse' }],
    safeActions: [{ id: 'inspect-review-gates', label: 'Inspect review gates', guard: 'safe', target: 'governance_guardhouse' }],
    gatedActions: [{ id: 'draft-governance-decision', label: 'Draft governance decision packet', guard: 'draft_only', target: 'governance_guardhouse' }],
    visualStateMapping: VISUAL_STATE_MAPPING,
    acceptanceCriteria: ['guardhouse blocks are visibly distinct', 'gated decisions are never auto-executed'],
  },
  {
    districtId: 'district_monitoring',
    title: 'Fleet & Monitoring',
    ownerAgent: 'WARDEN',
    sceneZoneId: 'systems_health',
    domain: 'Fleet health, runtime drift, storage, and package state',
    sourcePaths: ['data/fleet', 'data/warden', 'core/state/fleet.json'],
    provenanceDomainIds: ['systems:data/fleet', 'systems:core/state/fleet.json'],
    urgencyInputs: ['runtime drift', 'storage pressure', 'package drift', 'service health'],
    alertTriggers: ['critical service offline', 'storage pressure high', 'drift detected'],
    workstationId: 'systems_health-workstation',
    sourceZoneId: 'systems_health',
    primaryActions: [{ id: 'open-fleet-workstation', label: 'Open Fleet workstation', guard: 'safe', target: 'systems_health' }],
    safeActions: [{ id: 'inspect-fleet-health', label: 'Inspect fleet health', guard: 'safe', target: 'systems_health' }],
    gatedActions: [{ id: 'draft-maintenance-packet', label: 'Draft maintenance packet', guard: 'draft_only', target: 'systems_health' }],
    visualStateMapping: VISUAL_STATE_MAPPING,
    acceptanceCriteria: ['fleet pressure is visible in world', 'maintenance remains draft-only from world'],
  },
  {
    districtId: 'district_human_business',
    title: 'Human & Business',
    ownerAgent: 'HUMAN',
    sceneZoneId: 'human_realm',
    domain: 'Human-facing business, personal, and realm context',
    sourcePaths: ['human/', 'core/state/business.json'],
    provenanceDomainIds: ['human:human/', 'business:core/state/business.json'],
    urgencyInputs: ['business state freshness', 'personal context freshness', 'human escalation'],
    alertTriggers: ['human escalation pending', 'business state stale', 'missing human context'],
    workstationId: 'human_realm-workstation',
    sourceZoneId: 'human_realm',
    primaryActions: [{ id: 'open-human-workstation', label: 'Open Human Realm workstation', guard: 'safe', target: 'human_realm' }],
    safeActions: [{ id: 'inspect-human-context', label: 'Inspect human/business context', guard: 'safe', target: 'human_realm' }],
    gatedActions: [{ id: 'draft-human-request', label: 'Draft human escalation packet', guard: 'draft_only', target: 'human_realm' }],
    visualStateMapping: VISUAL_STATE_MAPPING,
    acceptanceCriteria: ['human escalation is visible without auto-sending anything', 'business/personal placeholders remain local until sourced'],
  },
  {
    districtId: 'district_finance',
    title: 'Economy',
    ownerAgent: 'PLUTUS',
    sceneZoneId: 'lifecycle_execution_economics',
    domain: 'JouleWork accounting and output economics',
    sourcePaths: ['data/plutus', 'core/ledger'],
    provenanceDomainIds: ['economics:data/plutus'],
    urgencyInputs: ['joule accounting freshness', 'cost pressure', 'economic ledger availability'],
    alertTriggers: ['ledger unavailable', 'cost pressure high', 'economics projection stale'],
    workstationId: 'lifecycle_execution_economics-workstation',
    sourceZoneId: 'lifecycle_execution_economics',
    primaryActions: [{ id: 'open-economy-workstation', label: 'Open Economy workstation', guard: 'safe', target: 'lifecycle_execution_economics' }],
    safeActions: [{ id: 'inspect-joulework', label: 'Inspect JouleWork accounting', guard: 'safe', target: 'lifecycle_execution_economics' }],
    gatedActions: [{ id: 'draft-economics-review', label: 'Draft economics review packet', guard: 'draft_only', target: 'lifecycle_execution_economics' }],
    visualStateMapping: VISUAL_STATE_MAPPING,
    acceptanceCriteria: ['economy can be marked reference/future when data is unavailable', 'ledger absence maps to unknown or stale'],
  },
]

export function getWorldDistrictContract(districtIdOrZoneId: string): WorldDistrictContract | null {
  return WORLD_DISTRICT_CONTRACTS.find((contract) => contract.districtId === districtIdOrZoneId || contract.sceneZoneId === districtIdOrZoneId) ?? null
}

export function contractForZone(zone: SceneZoneDefinition): WorldDistrictContract | null {
  return getWorldDistrictContract(zone.id) ?? getWorldDistrictContract(zone.sourceIds[0] ?? zone.id)
}

export function assertWorldDistrictContractsComplete(contracts: WorldDistrictContract[] = WORLD_DISTRICT_CONTRACTS): string[] {
  const errors: string[] = []
  contracts.forEach((contract) => {
    if (!contract.districtId) errors.push(`${contract.title}: missing district_id`)
    if (!contract.ownerAgent) errors.push(`${contract.districtId}: missing owner_agent`)
    if (contract.sourcePaths.length === 0) errors.push(`${contract.districtId}: missing source_paths`)
    if (contract.urgencyInputs.length === 0) errors.push(`${contract.districtId}: missing urgency_inputs`)
    if (contract.alertTriggers.length === 0) errors.push(`${contract.districtId}: missing alert_triggers`)
    if (!contract.workstationId && contract.primaryActions.length === 0) errors.push(`${contract.districtId}: missing workstation/action target`)
    if (contract.acceptanceCriteria.length === 0) errors.push(`${contract.districtId}: missing acceptance criteria`)
  })
  return errors
}
