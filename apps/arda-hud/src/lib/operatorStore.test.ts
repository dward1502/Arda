import { afterEach, describe, expect, it } from 'vitest'
import operatorStore, { type HermesRuntimeHealth } from './operatorStore'

function health(sourceTimeUtc: string, state: HermesRuntimeHealth['state']): HermesRuntimeHealth {
  const runtimeReady = state === 'healthy'
  return {
    schemaVersion: 'arda.system-health.hermes.v1',
    state,
    sourceRevision: `revision-${state}`,
    sourceTimeUtc,
    runtimeAvailable: runtimeReady,
    runtimeIdentity: runtimeReady ? 'hermes-dashboard:127.0.0.1:9119' : null,
    runtimeLaunched: false,
    runtimeReady,
    url: 'http://127.0.0.1:9119',
    port: 9119,
    probes: { port: runtimeReady, identity: runtimeReady },
    failure: runtimeReady ? null : 'not listening',
    recoveryAction: runtimeReady ? null : 'start Hermes',
  }
}

afterEach(() => operatorStore.reset())

describe('operatorStore Hermes projection ordering', () => {
  it('rejects an older healthy response after a newer unavailable response', () => {
    operatorStore.patch(health('2026-08-11T16:31:00Z', 'unavailable'))
    operatorStore.patch(health('2026-08-11T16:30:00Z', 'healthy'))

    expect(operatorStore.current.state).toBe('unavailable')
    expect(operatorStore.current.runtimeReady).toBe(false)
    expect(operatorStore.current.sourceTimeUtc).toBe('2026-08-11T16:31:00Z')
  })
})
