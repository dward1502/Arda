// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { deriveAutomationStatusSurface } from './automationStatus'

describe('deriveAutomationStatusSurface', () => {
  it('surfaces failed daily eval hold state without granting execution', () => {
    const surface = deriveAutomationStatusSurface(
      {
        schema_version: 'arda.daily_autonomy_eval.v1',
        generated_at_utc: '2026-05-24T10:00:00.000Z',
        status: 'fail',
        score: 0.42,
        summary: {
          evaluated_receipts: 7,
          failed_receipts: 2,
        },
        checks: [
          { name: 'receipt_contracts', status: 'fail', detail: '2 receipts missing schema_version' },
          { name: 'queue_integrity', status: 'pass', detail: 'valid jsonl' },
        ],
      },
      {
        schema_version: 'arda.autonomy_hold_reason.v1',
        generated_at_utc: '2026-05-24T10:00:01.000Z',
        execution_allowed: false,
        mutate_task_queue: false,
        blocked_actions: ['retry_failed_services', 'mutate_task_queue'],
        reason: 'daily autonomy eval failed',
        source_eval_status: 'fail',
        source_eval_score: 0.42,
      },
    )

    expect(surface.schema_version).toBe('arda.arda.automation_status_surface.v1')
    expect(surface.execution_allowed).toBe(false)
    expect(surface.posture).toBe('held')
    expect(surface.eval.status).toBe('fail')
    expect(surface.eval.score).toBe(0.42)
    expect(surface.hold.present).toBe(true)
    expect(surface.hold.mutate_task_queue).toBe(false)
    expect(surface.top_checks[0]).toMatchObject({ name: 'receipt_contracts', status: 'fail' })
    expect(surface.evidence_sources).toEqual([
      'data/autonomy/daily_eval_last.json',
      'data/autonomy/hold_reason_last.json',
    ])
  })

  it('defaults to unknown when receipts are missing', () => {
    const surface = deriveAutomationStatusSurface(null, null)

    expect(surface.posture).toBe('unknown')
    expect(surface.execution_allowed).toBe(false)
    expect(surface.eval.status).toBe('missing')
    expect(surface.hold.present).toBe(false)
  })
})
