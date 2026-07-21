import { safeTauriInvoke } from './tauriGuard'
import operatorStore, { type HermesRuntimeHealth, type LaunchHermesRuntimeResult } from './operatorStore'

export interface HermesRuntimeWindowResult {
  readonly window_label: string
  readonly url: string | null
  readonly port: number | null
  readonly launched: boolean
  readonly ready: boolean
  readonly identity: string | null
  readonly spotCount: number
  readonly failure: string | null
}

export interface HermesRuntimeHealthResult {
  readonly health: HermesRuntimeHealth
  readonly surfaceAvailable: boolean
}

export async function ensureHermesRuntimeSpots(): Promise<HermesRuntimeWindowResult> {
  return safeTauriInvoke<HermesRuntimeWindowResult>('ensure_hermes_runtime_surface')
}

export async function readHermesRuntimeHealth(): Promise<HermesRuntimeHealthResult> {
  const status = await safeTauriInvoke<{
    runtimeAvailable: boolean
    runtimeIdentity: string | null
    runtimeReady: boolean
    runtimeLaunched: boolean
    runtimeVersion: string | null
    sessionDirectory: string | null
    spotsCount: number
    spotsActive: number
    url: string | null
    port: number | null
    probes: {
      port: boolean
      identity: boolean
      version: boolean
      sessionDirectory: boolean
      atLeastOneSpot: boolean
    }
    failure: string | null
  }>('read_hermes_runtime_health')

  operatorStore.patch({
    runtimeAvailable: status.runtimeAvailable,
    runtimeIdentity: status.runtimeIdentity,
    runtimeLaunched: status.runtimeLaunched,
    runtimeReady: status.runtimeReady,
    runtimeVersion: status.runtimeVersion,
    sessionDirectory: status.sessionDirectory,
    spotsCount: status.spotsCount,
    spotsActive: status.spotsActive,
    probes: status.probes,
  })

  return {
    health: operatorStore.current,
    surfaceAvailable: true,
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
