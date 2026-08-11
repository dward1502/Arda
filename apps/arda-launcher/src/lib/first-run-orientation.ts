import type { OnboardingSnapshot } from './tauri-core-compat'

export interface FirstRunOrientation {
  systemState: string
  canActNow: string
  approvalAuthority: string
  evidenceQuality: string
  executionBlockers: string
  nextAction: string
}

function count(summary: Record<string, number>, key: string): number {
  return summary[key] ?? 0
}

export function summarizeFirstRunOrientation(snapshot: OnboardingSnapshot): FirstRunOrientation {
  const failedChecks = snapshot.readiness.checks.filter(check => check.status === 'fail')
  const warningChecks = snapshot.readiness.checks.filter(check => check.status === 'warn')
  const firstActionable = failedChecks[0] ?? warningChecks[0]
  const passCount = count(snapshot.readiness.summary, 'pass')
  const warnCount = count(snapshot.readiness.summary, 'warn')
  const failCount = count(snapshot.readiness.summary, 'fail')

  let systemState: string
  let executionBlockers: string
  let nextAction: string

  if (snapshot.compatibility.status !== 'supported') {
    systemState = 'Unsupported environment — installation and Workbench startup are blocked.'
    executionBlockers = `${snapshot.compatibility.pretty_name} is outside the supported ${snapshot.compatibility.supported_profile} profile.`
    nextAction = `Move to the supported profile ${snapshot.compatibility.supported_profile}; do not continue installation here.`
  } else if (snapshot.gate_status === 'pass' && snapshot.can_start_workbench) {
    systemState = 'Ready — required startup checks passed and Workbench can start.'
    executionBlockers = 'No current readiness blocker.'
    nextAction = snapshot.guided.next_actions[0]
      ?? snapshot.guided.steps.find(step => step.status !== 'complete')?.next_action
      ?? 'Open Workbench and attach a project.'
  } else if (snapshot.gate_status === 'fail' || failedChecks.length > 0) {
    systemState = 'Blocked — Workbench cannot start until failed readiness checks are resolved.'
    executionBlockers = firstActionable
      ? `${firstActionable.title}: ${firstActionable.recommendation}`
      : 'A required readiness check failed; review actionable diagnostics.'
    nextAction = firstActionable?.recommendation ?? 'Review the failed readiness checks before continuing.'
  } else {
    systemState = 'Needs attention — Workbench remains locked while readiness warnings are reviewed.'
    executionBlockers = firstActionable
      ? `${firstActionable.title}: ${firstActionable.recommendation}`
      : 'Readiness has unresolved warnings.'
    nextAction = firstActionable?.recommendation ?? 'Review readiness warnings before continuing.'
  }

  return {
    systemState,
    canActNow: 'Arda may inspect readiness across compatibility, prerequisites, providers, and services without changing the system.',
    approvalAuthority: 'Only you, the operator, can approve setup actions or project execution. No setup or project change runs automatically.',
    evidenceQuality: `${passCount} passing readiness checks, ${warnCount} warnings, and ${failCount} failures were reported at ${snapshot.generated_at_utc}. Readiness evidence permits startup only; it does not prove a project change succeeded.`,
    executionBlockers,
    nextAction,
  }
}
