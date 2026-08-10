import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import {
  parseOperatorProjection,
  projectionMonitorSignals,
  type OperatorProjection,
} from './operatorProjection'

function fixtureValue(): unknown {
  const path = resolve(process.cwd(), '../../spec/operator-projection/v1/fixtures/valid-operator-projection.json')
  return JSON.parse(readFileSync(path, 'utf8'))
}

describe('canonical operator projection', () => {
  it('parses the shared P9.1 fixture as a read-only typed projection', () => {
    const projection: OperatorProjection = parseOperatorProjection(fixtureValue())

    expect(projection.schema_version).toBe('arda.operator-projection.v1')
    expect(projection.authority).toBe('read_only')
    expect(projection.runs[0]?.workers[0]?.worker_id).toBe('hermes-p9')
    expect(projection.capabilities.find((item) => item.optional)?.health).toBe('unavailable')
    expect(projection.pending_approvals).toHaveLength(1)
    expect(projection.councils[0]?.non_approval).toBe(true)
    expect(projection.personal_operations.reminders[0]?.status).toBe('deferred')
    expect(projection.joulework.source_confidence).toBe(0.92)
  })

  it('derives compact monitor signals without copying transition authority', () => {
    const signals = projectionMonitorSignals(parseOperatorProjection(fixtureValue()))

    expect(signals).toMatchObject({
      activeObjectives: 1,
      activeRuns: 1,
      runningWorkers: 1,
      pendingApprovals: 1,
      degradedDependencies: 1,
      unavailableOptionalCapabilities: 1,
      authority: 'read_only',
    })
    expect(Object.keys(signals)).not.toContain('actions')
  })

  it('fails closed on unknown fields and inconsistent stale state', () => {
    expect(() => parseOperatorProjection({ ...fixtureValue() as object, transition: 'approve' }))
      .toThrow(/unknown operator projection field: transition/)

    const stale = fixtureValue() as { dependencies: Array<Record<string, unknown>> }
    stale.dependencies[0] = { ...stale.dependencies[0], health: 'stale', freshness: 'fresh' }
    expect(() => parseOperatorProjection(stale)).toThrow(/stale dependency cannot be fresh/)
  })
})
