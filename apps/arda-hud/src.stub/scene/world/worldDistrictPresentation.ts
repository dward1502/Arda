// sigil: REPAIR
import type { SceneZoneDefinition } from '../systems/runtimeTypes'
import type { WorldDistrictUrgencyState } from './worldDistrictContracts'
import type { WorldDistrictUrgency } from './worldDistrictUrgency'

export interface WorldDistrictPresentation {
  title: string
  badge: string
  detail: string
  actionLabel: string
  tone: WorldDistrictUrgencyState
  color: string
  openTargetZoneId: string
}

const URGENCY_COLORS: Record<WorldDistrictUrgencyState, string> = {
  nominal: '#55f0ff',
  attention: '#ffd166',
  stale: '#b98cff',
  blocked: '#ffb14a',
  critical: '#ff4d6d',
  unknown: '#7c879a',
}

function firstUsefulSignal(urgency: WorldDistrictUrgency | undefined): string {
  const signal = urgency?.topSignals.find((candidate) => candidate.trim().length > 0)
  return signal ?? 'No urgency projection available'
}

export function resolveWorldDistrictPresentation(
  zone: SceneZoneDefinition,
  urgency: WorldDistrictUrgency | undefined,
): WorldDistrictPresentation {
  const tone = urgency?.urgency ?? 'unknown'
  const detail = urgency?.summary ?? `${zone.title}: unknown; ${firstUsefulSignal(urgency)}.`
  return {
    title: zone.title,
    badge: tone,
    detail,
    actionLabel: urgency?.recommendedAction ?? 'Open district panel',
    tone,
    color: URGENCY_COLORS[tone],
    openTargetZoneId: urgency?.sourceZoneId ?? zone.id,
  }
}
