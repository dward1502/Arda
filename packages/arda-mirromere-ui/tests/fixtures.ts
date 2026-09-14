// Test-only canonical fixtures. Production code must not import this module.
import ambientIdle from '../../../spec/mirromere-surface/v1/fixtures/ambient-idle.json'
import continuityHandoffReady from '../../../spec/mirromere-surface/v1/fixtures/continuity-handoff-ready.json'
import systemDegraded from '../../../spec/mirromere-surface/v1/fixtures/system-degraded.json'
import type { MirromereSurface } from '../src/contract'

export const ambientIdleFixture = ambientIdle as unknown as MirromereSurface
export const systemDegradedFixture = systemDegraded as unknown as MirromereSurface
export const continuityHandoffReadyFixture = continuityHandoffReady as unknown as MirromereSurface