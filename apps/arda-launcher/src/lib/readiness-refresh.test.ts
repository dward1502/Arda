import { describe, expect, it, vi } from 'vitest'
import { createReadinessRequests, refreshReadiness } from './readiness-refresh'
import type { OnboardingSnapshot, RegistryStatusPayload } from './tauri-core-compat'

const registry = { loaded: true, gate_status: 'pass', track_count: 4 } as RegistryStatusPayload
function snapshot(pass: boolean): OnboardingSnapshot {
  return {
    compatibility: { status: 'supported' },
    gate_status: pass ? 'pass' : 'warn',
    can_start_workbench: pass,
    readiness: { gate_status: pass ? 'pass' : 'warn', summary: { pass: 7, warn: pass ? 0 : 1 } },
  } as unknown as OnboardingSnapshot
}

describe('readiness refresh', () => {
  it('rejects older success after replay and a newer failed refresh', async () => {
    const requests = createReadinessRequests()
    let resolve!: (value: OnboardingSnapshot) => void
    const pending = new Promise<OnboardingSnapshot>(done => { resolve = done })
    const old = requests.run(async () => registry, () => pending)
    requests.invalidate()
    const current = await requests.run(async () => registry, async () => { throw new Error('offline') })
    expect(current?.decision.isReady).toBe(false)
    resolve(snapshot(true))
    expect(await old).toBeNull()
  })
  it('invalidates pending reads on replay or unmount without another request', async () => {
    const requests = createReadinessRequests()
    let resolve!: (value: OnboardingSnapshot) => void
    const pending = new Promise<OnboardingSnapshot>(done => { resolve = done })
    const old = requests.run(async () => registry, () => pending)
    requests.invalidate()
    resolve(snapshot(true))
    expect(await old).toBeNull()
  })
  it('reevaluates both current inputs on every refresh, including recovery', async () => {
    const readRegistry = vi.fn().mockResolvedValue(registry)
    const readSnapshot = vi.fn().mockResolvedValueOnce(snapshot(true))
      .mockResolvedValueOnce(snapshot(false)).mockResolvedValueOnce(snapshot(true))
    expect((await refreshReadiness(readRegistry, readSnapshot)).decision.isReady).toBe(true)
    expect((await refreshReadiness(readRegistry, readSnapshot)).decision.isReady).toBe(false)
    expect((await refreshReadiness(readRegistry, readSnapshot)).decision.isReady).toBe(true)
    expect(readRegistry).toHaveBeenCalledTimes(3)
  })
  it('invalidates old snapshots and readiness when either read fails', async () => {
    for (const failRegistry of [true, false]) {
      const fail = () => Promise.reject(new Error('offline'))
      const result = await refreshReadiness(
        failRegistry ? fail : async () => registry,
        failRegistry ? async () => snapshot(true) : fail,
      )
      expect(result.snapshot).toBeNull()
      expect(result.decision.isReady).toBe(false)
      expect(result.decision.registry).toBe('fail')
      expect(result.error).toContain('offline')
    }
  })
})
