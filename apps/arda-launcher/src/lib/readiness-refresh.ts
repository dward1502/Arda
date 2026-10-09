import { evaluateReadinessGate } from './readiness-gate'
import type { OnboardingSnapshot, RegistryStatusPayload } from './tauri-core-compat'

export function createReadinessRequests() {
  let generation = 0
  return {
    invalidate() { generation += 1 },
    async run(
      readRegistry: () => Promise<RegistryStatusPayload>,
      readSnapshot: () => Promise<OnboardingSnapshot>,
    ) {
      const request = ++generation
      const result = await refreshReadiness(readRegistry, readSnapshot)
      return request === generation ? result : null
    },
  }
}

// Startup and operator refresh must use the same current evidence and failure policy.
export async function refreshReadiness(
  readRegistry: () => Promise<RegistryStatusPayload>,
  readSnapshot: () => Promise<OnboardingSnapshot>,
) {
  try {
    const [registry, snapshot] = await Promise.all([readRegistry(), readSnapshot()])
    return { snapshot, decision: evaluateReadinessGate(registry, snapshot), error: null }
  } catch (cause) {
    const error = `Readiness diagnostics failed: ${cause}`
    return {
      snapshot: null,
      decision: { registry: 'fail' as const, statusLabel: error, isReady: false },
      error,
    }
  }
}
