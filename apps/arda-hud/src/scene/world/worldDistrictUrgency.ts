// sigil: REPAIR
import type { ArdaBundle } from '../../lib/ardaSource'
import type { ArdaSourceProvenance } from '../../lib/ardaProvenance'
import type { SceneZoneDefinition } from '../systems/runtimeTypes'
import { contractForZone, type WorldDistrictContract, type WorldDistrictUrgencyState } from './worldDistrictContracts'

export interface WorldDistrictUrgency {
  districtId: string
  sourceZoneId: string
  urgency: WorldDistrictUrgencyState
  summary: string
  topSignals: string[]
  recommendedAction: string
}

interface JsonRecord {
  [key: string]: unknown
}

function asRecord(value: unknown): JsonRecord | null {
  return value && typeof value === 'object' && !Array.isArray(value) ? value as JsonRecord : null
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function textIncludes(value: unknown, needles: string[]): boolean {
  const text = JSON.stringify(value ?? '').toLowerCase()
  return needles.some((needle) => text.includes(needle))
}

function maxUrgency(current: WorldDistrictUrgencyState, next: WorldDistrictUrgencyState): WorldDistrictUrgencyState {
  const rank: Record<WorldDistrictUrgencyState, number> = {
    unknown: 0,
    nominal: 1,
    stale: 2,
    attention: 3,
    blocked: 4,
    critical: 5,
  }
  return rank[next] > rank[current] ? next : current
}

function provenanceForContract(contract: WorldDistrictContract, provenance: ArdaSourceProvenance[]): ArdaSourceProvenance[] {
  return provenance.filter((record) => (
    contract.provenanceDomainIds.includes(record.domainId)
    || record.sourcePaths.some((sourcePath) => contract.sourcePaths.some((contractPath) => sourcePath.startsWith(contractPath) || contractPath.startsWith(sourcePath)))
  ))
}

function sectionForContract(contract: WorldDistrictContract, bundle: Pick<ArdaBundle, 'sections'>): JsonRecord | null {
  return asRecord(bundle.sections.find((section) => section.id === contract.sourceZoneId || section.id === contract.sceneZoneId) ?? null)
}

export function calculateWorldDistrictUrgency(
  contract: WorldDistrictContract,
  bundle: Pick<ArdaBundle, 'sections' | 'sourceProvenance'>,
): WorldDistrictUrgency {
  const signals: string[] = []
  let urgency: WorldDistrictUrgencyState = 'nominal'
  const provenance = provenanceForContract(contract, bundle.sourceProvenance ?? [])
  const section = sectionForContract(contract, bundle)

  if (provenance.length === 0) {
    urgency = 'unknown'
    signals.push('No matching provenance record')
  } else if (provenance.some((record) => record.state === 'stale' || record.state === 'missing')) {
    urgency = maxUrgency(urgency, 'stale')
    signals.push('Source freshness is stale or missing')
  }

  const sectionSignals = [section, ...asArray(section?.signals), ...asArray(section?.highlights), ...asArray(section?.alerts)]
  if (sectionSignals.some((signal) => textIncludes(signal, ['critical', 'offline', 'failed', 'unhealthy']))) {
    urgency = maxUrgency(urgency, 'critical')
    signals.push('Critical/offline runtime signal')
  }
  if (sectionSignals.some((signal) => textIncludes(signal, ['blocked', 'policy', 'triad required', 'unsafe']))) {
    urgency = maxUrgency(urgency, 'blocked')
    signals.push('Blocked or gated governance signal')
  }
  if (sectionSignals.some((signal) => textIncludes(signal, ['pending', 'review', 'queue', 'drift', 'pressure', 'degraded', 'attention']))) {
    urgency = maxUrgency(urgency, 'attention')
    signals.push('Operational attention signal')
  }

  const queueCount = Number(asRecord(section?.counts)?.open_tasks_total ?? asRecord(section?.counts)?.blocked_tasks_total ?? 0)
  if (Number.isFinite(queueCount) && queueCount > 0) {
    urgency = maxUrgency(urgency, queueCount > 10 ? 'critical' : 'attention')
    signals.push(`Queue pressure: ${queueCount}`)
  }

  const topSignals = signals.length > 0 ? signals.slice(0, 3) : ['No elevated projection-backed signal']
  const recommendedAction = contract.primaryActions[0]?.label ?? 'Inspect district contract'
  return {
    districtId: contract.districtId,
    sourceZoneId: contract.sourceZoneId,
    urgency,
    summary: `${contract.title}: ${urgency}; ${topSignals[0]}.`,
    topSignals,
    recommendedAction,
  }
}

export function calculateWorldDistrictUrgencies(
  zones: SceneZoneDefinition[],
  bundle: Pick<ArdaBundle, 'sections' | 'sourceProvenance'>,
): Record<string, WorldDistrictUrgency> {
  return Object.fromEntries(zones.map((zone) => {
    const contract = contractForZone(zone)
    if (!contract) {
      return [zone.id, {
        districtId: zone.id,
        sourceZoneId: zone.id,
        urgency: 'unknown' as const,
        summary: `${zone.title}: unknown; no district contract.`,
        topSignals: ['No district contract'],
        recommendedAction: 'Define district contract',
      }]
    }
    return [zone.id, calculateWorldDistrictUrgency(contract, bundle)]
  }))
}
