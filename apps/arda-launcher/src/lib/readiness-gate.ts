import type { OnboardingSnapshot, RegistryStatusPayload } from './tauri-core-compat'

export interface ReadinessGateDecision {
  registry: 'pass' | 'warn' | 'fail'
  statusLabel: string
  isReady: boolean
}

export function evaluateReadinessGate(
  registry: RegistryStatusPayload,
  snapshot: OnboardingSnapshot,
): ReadinessGateDecision {
  const readiness = snapshot.readiness

  if (!registry.loaded) {
    return {
      registry: 'fail',
      statusLabel: registry.error || 'Registry unavailable',
      isReady: false,
    }
  }

  if (snapshot.compatibility.status !== 'supported') {
    return {
      registry: 'fail',
      statusLabel: `Unsupported profile: ${snapshot.compatibility.pretty_name}`,
      isReady: false,
    }
  }

  if (
    registry.gate_status === 'pass' &&
    snapshot.gate_status === 'pass' &&
    readiness.gate_status === 'pass' &&
    snapshot.can_start_workbench
  ) {
    return {
      registry: 'pass',
      statusLabel: `Ready: ${registry.track_count} tracks and ${readiness.summary.pass ?? 0} setup checks verified`,
      isReady: true,
    }
  }

  if (registry.gate_status === 'warn' || readiness.gate_status === 'warn') {
    return {
      registry: 'warn',
      statusLabel: `Readiness review: ${readiness.summary.warn ?? 0} warning(s)`,
      isReady: false,
    }
  }

  return {
    registry: 'fail',
    statusLabel: `Readiness blocked: ${readiness.summary.fail ?? 1} failed check(s)`,
    isReady: false,
  }
}
