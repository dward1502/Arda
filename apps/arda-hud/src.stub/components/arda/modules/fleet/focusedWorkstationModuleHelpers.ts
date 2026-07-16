export type WorkstationModuleRole = 'default' | 'focused'

export const WORKSTATION_FOCUSED_ZONES = new Set([
  'systems_health',
  'routing_health',
  'sovereign_world',
])

export function buildWorkstationModuleRole(zoneId: string | null): WorkstationModuleRole {
  if (!zoneId) return 'default'
  return WORKSTATION_FOCUSED_ZONES.has(zoneId) ? 'focused' : 'default'
}
