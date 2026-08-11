import { describe, expect, it } from 'vitest'
import contract from '../../../../spec/hud-convergence/v1/fixtures/valid-shared-contract.json'

const projectionStates = [
  'loading',
  'healthy',
  'stale',
  'partial',
  'degraded',
  'unavailable',
  'failed',
]

const monitorSlots = [
  'monitor_1',
  'monitor_2',
  'monitor_3',
  'monitor_4',
  'monitor_5',
]

describe('shared HUD convergence contract', () => {
  it('keeps browser intents free of backend-created authority', () => {
    const intent = contract.mutation.intent as Record<string, unknown>
    const planIntent = contract.workbench.plan_intent as Record<string, unknown>
    const approvalIntent = contract.workbench.approval_intent as Record<string, unknown>
    const completionIntent = contract.workbench.completion_intent as Record<string, unknown>

    expect(intent).not.toHaveProperty('receipt_id')
    expect(intent).not.toHaveProperty('recorded_at_utc')
    expect(planIntent).not.toHaveProperty('run_id')
    expect(planIntent).not.toHaveProperty('node_ids')
    expect(approvalIntent).not.toHaveProperty('policy_decision')
    expect(approvalIntent).not.toHaveProperty('approved_at_utc')
    expect(completionIntent).not.toHaveProperty('evidence')
    expect(completionIntent).not.toHaveProperty('receipt_digest')
  })

  it('pins the complete frontend projection vocabulary', () => {
    expect(contract.load_states.map(({ status }) => status)).toEqual(projectionStates)
  })

  it('pins five independently owned same-session monitor handoffs', () => {
    expect(Object.keys(contract.monitor_sessions)).toEqual(monitorSlots)

    const monitors = Object.entries(contract.monitor_sessions)
    expect(new Set(monitors.map(([, monitor]) => monitor.owner)).size).toBe(5)

    for (const [slotId, monitor] of monitors) {
      expect(monitor.slot_id).toBe(slotId)
      expect(monitor.workstation_handoff.session_id).toBe(monitor.session_id)
      expect(monitor.workstation_handoff.mode).toBe('same_live_session')
    }
  })
})
