import { safeTauriInvoke } from './tauriGuard'
import operatorStore, { type HermesRuntimeHealth, type LaunchHermesRuntimeResult } from './operatorStore'

export interface HermesRuntimeWindowResult {
  readonly windowLabel: string
  readonly url: string
  readonly port: number
  readonly launched: boolean
  readonly ready: boolean
  readonly runtimeIdentity: string | null
  readonly state: string
}

export interface HermesRuntimeHealthResult {
  readonly health: HermesRuntimeHealth
  readonly surfaceAvailable: boolean
}

export async function ensureHermesRuntimeSpots(): Promise<HermesRuntimeWindowResult> {
  return safeTauriInvoke<HermesRuntimeWindowResult>('ensure_hermes_runtime_surface')
}

export async function readHermesRuntimeHealth(): Promise<HermesRuntimeHealthResult> {
  const status = await safeTauriInvoke<HermesRuntimeHealth>('read_hermes_runtime_health')
  operatorStore.patch(status)

  return {
    health: operatorStore.current,
    surfaceAvailable: operatorStore.current.runtimeReady,
  }
}

export async function openHermesRuntimeWindow(): Promise<HermesRuntimeWindowResult> {
  return safeTauriInvoke<HermesRuntimeWindowResult>('open_hermes_runtime_window')
}

export function describeHermesRuntimeLaunch(result: LaunchHermesRuntimeResult): string {
  if (result.ready && result.url) return `Hermes runtime ready at ${result.url}`
  if (result.launched && result.port) return `Hermes runtime launched on port ${result.port}`
  if (result.failure) return `Hermes runtime launch failed: ${result.failure}`
  return 'Hermes runtime launch awaiting confirmation'
}
