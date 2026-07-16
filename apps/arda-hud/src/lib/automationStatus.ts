// sigil: REPAIR

export type JsonRecord = Record<string, unknown>

export interface AutomationStatusCheck {
  name: string
  status: string
  detail: string
}

export interface AutomationStatusSurface {
  schema_version: 'arda.arda.automation_status_surface.v1'
  authority: 'arda_automation_status_surface'
  generated_at_utc: string
  posture: 'passing' | 'held' | 'warning' | 'unknown'
  execution_allowed: boolean
  eval: {
    present: boolean
    status: string
    score: number | null
    generated_at_utc: string
    evaluated_receipts: number
    failed_receipts: number
  }
  hold: {
    present: boolean
    reason: string
    execution_allowed: boolean
    mutate_task_queue: boolean
    blocked_actions: string[]
    generated_at_utc: string
  }
  top_checks: AutomationStatusCheck[]
  evidence_sources: string[]
}

function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as JsonRecord
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function stringValue(value: unknown, fallback = ''): string {
  return typeof value === 'string' ? value : fallback
}

function numberValue(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function booleanValue(value: unknown, fallback = false): boolean {
  return typeof value === 'boolean' ? value : fallback
}

function checksFromEval(evalReceipt: JsonRecord | null): AutomationStatusCheck[] {
  return asArray(evalReceipt?.checks)
    .map((entry) => asRecord(entry))
    .filter((entry): entry is JsonRecord => entry !== null)
    .map((entry) => ({
      name: stringValue(entry.name, 'unknown_check'),
      status: stringValue(entry.status, 'unknown'),
      detail: stringValue(entry.detail, ''),
    }))
    .sort((left, right) => {
      const rank = (status: string) => (status === 'fail' ? 0 : status === 'warn' ? 1 : 2)
      const byStatus = rank(left.status) - rank(right.status)
      return byStatus === 0 ? left.name.localeCompare(right.name) : byStatus
    })
    .slice(0, 4)
}

function postureFor(evalStatus: string, holdPresent: boolean): AutomationStatusSurface['posture'] {
  if (holdPresent) return 'held'
  if (evalStatus === 'pass') return 'passing'
  if (evalStatus === 'warn') return 'warning'
  return 'unknown'
}

export function deriveAutomationStatusSurface(
  dailyEvalReceipt: JsonRecord | null,
  holdReceipt: JsonRecord | null,
  now = new Date().toISOString(),
): AutomationStatusSurface {
  const evalSummary = asRecord(dailyEvalReceipt?.summary)
  const evalStatus = dailyEvalReceipt ? stringValue(dailyEvalReceipt.status, 'unknown') : 'missing'
  const holdPresent = holdReceipt !== null
  const holdExecutionAllowed = holdPresent ? booleanValue(holdReceipt.execution_allowed, false) : false
  const executionAllowed = evalStatus === 'pass' && (!holdPresent || holdExecutionAllowed)

  return {
    schema_version: 'arda.arda.automation_status_surface.v1',
    authority: 'arda_automation_status_surface',
    generated_at_utc: now,
    posture: postureFor(evalStatus, holdPresent),
    execution_allowed: executionAllowed,
    eval: {
      present: dailyEvalReceipt !== null,
      status: evalStatus,
      score: numberValue(dailyEvalReceipt?.score),
      generated_at_utc: stringValue(dailyEvalReceipt?.generated_at_utc),
      evaluated_receipts: numberValue(evalSummary?.evaluated_receipts) ?? 0,
      failed_receipts: numberValue(evalSummary?.failed_receipts) ?? 0,
    },
    hold: {
      present: holdPresent,
      reason: stringValue(holdReceipt?.reason),
      execution_allowed: holdExecutionAllowed,
      mutate_task_queue: holdPresent ? booleanValue(holdReceipt.mutate_task_queue, false) : false,
      blocked_actions: asArray(holdReceipt?.blocked_actions).map((action) => stringValue(action)).filter(Boolean),
      generated_at_utc: stringValue(holdReceipt?.generated_at_utc),
    },
    top_checks: checksFromEval(dailyEvalReceipt),
    evidence_sources: [
      'data/autonomy/daily_eval_last.json',
      'data/autonomy/hold_reason_last.json',
    ],
  }
}
