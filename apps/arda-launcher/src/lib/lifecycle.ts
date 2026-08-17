import { invoke } from '@tauri-apps/api/core'

export type AggregateState = 'stopped' | 'starting' | 'healthy' | 'degraded' | 'failed' | 'stopping' | 'unknown'
export type Freshness = 'fresh' | 'stale' | 'unknown'
export interface Observation { source: string; source_id: string; observed_at: string; freshness: Freshness }
export interface Observed<T> { value: T; observation: Observation }
export interface ComponentObservation {
  component_id: string
  class: 'required' | 'optional'
  unit: { owning_unit: string; enablement: Observed<string>; active_state: Observed<string>; sub_state: Observed<string> }
  protocol_health: Observed<string>
  diagnostic: { code: string; message: string } | null
  recovery_action: string | null
}
export interface LifecycleSnapshot {
  schema_version: 'arda.system-lifecycle.v1'
  observed_at: string
  aggregate_state: AggregateState
  components: ComponentObservation[]
  hud_native: { availability: Observed<string>; running: Observed<string> }
  hermes_gateway: { availability: Observed<string>; protocol_health: Observed<string> }
}
export type PrimaryAction = { kind: 'start' | 'open_hud' | 'retry' | 'inspect'; label: string }

export function lifecyclePrimaryAction(snapshot: LifecycleSnapshot): PrimaryAction {
  const stale = snapshot.components.some(component =>
    component.class === 'required' &&
    (component.unit.active_state.observation.freshness !== 'fresh' || component.protocol_health.observation.freshness !== 'fresh'))
  if (stale) return { kind: 'retry', label: 'RETRY OBSERVATION' }
  switch (snapshot.aggregate_state) {
    case 'stopped': return { kind: 'start', label: 'START ARDA' }
    case 'healthy': return { kind: 'open_hud', label: 'OPEN HUD' }
    case 'degraded': return { kind: 'retry', label: 'RETRY RECOVERY' }
    default: return { kind: 'inspect', label: 'INSPECT FAILURE' }
  }
}

export function lifecycleRows(snapshot: LifecycleSnapshot) {
  return snapshot.components.filter(component => component.class === 'required').map(component => ({
    id: component.component_id,
    process: component.unit.active_state.value,
    health: component.protocol_health.value,
    freshness: component.protocol_health.observation.freshness,
    recovery: component.recovery_action,
  }))
}

export const invokeLifecycleStatus = () => invoke<LifecycleSnapshot>('lifecycle_status')
export const invokeStartSession = () => invoke<string>('start_arda_session')
export const invokeRecoverComponent = (actionId: string) => invoke<string>('recover_component', { actionId })
export const invokeLaunchNativeHud = () => invoke<string>('launch_native_hud')
