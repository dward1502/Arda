import type { ArdaSourceProvenance } from '../../lib/ardaProvenance'
import type { HudInstrumentSource } from './boardroomHudInstruments'

export type BoardroomHudSourceFamily =
  | 'fleet'
  | 'queue'
  | 'knowledge'
  | 'routing'
  | 'governance'
  | 'human'
  | 'daily-command'

const SOURCE_FAMILY_CONTRACTS: Record<BoardroomHudSourceFamily, { hints: string[]; fallbackPaths: string[] }> = {
  fleet: {
    hints: ['operator_runtime_status', 'fleet_runtime_drift'],
    fallbackPaths: ['core/state/operator_runtime_status.json', 'core/state/fleet_runtime_drift.json'],
  },
  queue: {
    hints: ['queue_summary', 'task_lifecycle_runtime', 'core/projects/tasks/queue'],
    fallbackPaths: ['core/state/queue_summary.json', 'core/projects/tasks/queue.jsonl'],
  },
  knowledge: {
    hints: ['athena_runtime', 'data/athena/digest', 'knowledge_triage', 'plan_map'],
    fallbackPaths: ['core/state/athena_runtime.json', 'data/athena/digest.jsonl'],
  },
  routing: {
    hints: ['manwe_router', 'provider_intelligence', 'lane_fitness'],
    fallbackPaths: ['core/state/manwe_router.json', 'core/state/provider_intelligence.json'],
  },
  governance: {
    hints: ['governance_runtime', 'warden_guardhouse', 'policy_readiness', 'approval'],
    fallbackPaths: ['core/state/governance_runtime.json', 'core/state/warden_guardhouse.json'],
  },
  human: {
    hints: ['human_context', 'business_runtime', 'personal_runtime'],
    fallbackPaths: ['core/state/human_context.json', 'core/state/business_runtime.json', 'core/state/personal_runtime.json'],
  },
  'daily-command': {
    hints: ['operations_flow', 'operator_actions', 'arda_snapshot'],
    fallbackPaths: ['core/state/operations_flow.json', 'core/state/operator_actions.json'],
  },
}

const FRESHNESS_PRIORITY = {
  blocked: 6,
  missing: 5,
  unknown: 4,
  stale: 3,
  derived: 2,
  fresh: 1,
} as const

function newestTimestamp(sources: ArdaSourceProvenance[]): string | null {
  const timestamps = sources
    .flatMap((source) => [source.observedAtUtc, source.generatedAtUtc])
    .filter((value): value is string => typeof value === 'string' && !Number.isNaN(Date.parse(value)))
    .sort((left, right) => Date.parse(right) - Date.parse(left))
  return timestamps[0] ?? null
}

function sourceTimestampMillis(source: ArdaSourceProvenance): number {
  const timestamp = newestTimestamp([source])
  return timestamp ? Date.parse(timestamp) : Number.NEGATIVE_INFINITY
}

export function adaptBoardroomHudSource(
  provenance: ArdaSourceProvenance[],
  family: BoardroomHudSourceFamily,
): HudInstrumentSource {
  const contract = SOURCE_FAMILY_CONTRACTS[family]
  const matches = provenance.filter((source) =>
    source.sourcePaths.some((path) => contract.hints.some((hint) => path.toLowerCase().includes(hint))),
  )
  if (matches.length === 0) {
    return {
      sourceId: family,
      sourceIds: [family],
      sourcePaths: contract.fallbackPaths,
      observedAtUtc: null,
      freshness: 'missing',
    }
  }

  const prioritizedMatches = [...matches].sort((left, right) => {
    return sourceTimestampMillis(right) - sourceTimestampMillis(left)
  })
  const sourcePaths = [...new Set(prioritizedMatches.flatMap((source) => source.sourcePaths))].slice(0, 8)
  const freshness = matches.reduce((worst, source) =>
    FRESHNESS_PRIORITY[source.state] > FRESHNESS_PRIORITY[worst] ? source.state : worst,
  'fresh' as ArdaSourceProvenance['state'])

  return {
    sourceId: prioritizedMatches[0].domainId,
    sourceIds: [...new Set(prioritizedMatches.map((source) => source.domainId))].slice(0, 8),
    sourcePaths,
    observedAtUtc: newestTimestamp(matches),
    freshness,
  }
}
