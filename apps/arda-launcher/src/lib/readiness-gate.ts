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

  if (registry.startup_gate_status !== 'pass') {
    return { registry: 'fail', isReady: false, statusLabel: `Registry startup blocked: ${registry.startup_blockers?.join('; ') || 'startup requirements unavailable'}` }
  }

  if (
    registry.startup_gate_status === 'pass' &&
    snapshot.gate_status === 'pass' &&
    readiness.gate_status === 'pass' &&
    snapshot.can_start_workbench
  ) {
    return {
      registry: 'pass',
      statusLabel: `Registry startup requirements verified; ${readiness.summary.pass ?? 0} setup checks passed`,
      isReady: true,
    }
  }

  if (snapshot.gate_status !== 'fail' && readiness.gate_status !== 'fail' &&
      (snapshot.gate_status === 'warn' || readiness.gate_status === 'warn')) {
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
