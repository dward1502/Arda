// sigil: REPAIR
import { render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import HermesDashboardModule from './HermesDashboardModule'

const ensureHermesRuntimeSpots = vi.fn()
const readHermesRuntimeHealth = vi.fn()

vi.mock('../../../lib/hermesDashboardLauncher', () => ({
  describeHermesRuntimeLaunch: (result: { launched: boolean; ready: boolean; url: string | null; port: number | null; failure?: string | null }) => {
    if (result.ready && result.url) return `Hermes runtime ready at ${result.url}`
    if (result.launched && result.port) return `Hermes runtime launched on port ${result.port}`
    if (result.failure) return `Hermes runtime launch failed: ${result.failure}`
    return 'Hermes runtime launch awaiting confirmation'
  },
  ensureHermesRuntimeSpots: (...args: unknown[]) => ensureHermesRuntimeSpots(...args),
  readHermesRuntimeHealth: (...args: unknown[]) => readHermesRuntimeHealth(...args),
}))

function renderModule() {
  return render(
    <HermesDashboardModule
      summary={[]}
      tools={[]}
      runtimeSurfaces={[]}
      auditReadiness={null}
      sourceProvenance={[]}
      tag="test"
    />,
  )
}

describe('HermesDashboardModule', () => {
  beforeEach(() => {
    ensureHermesRuntimeSpots.mockReset()
    readHermesRuntimeHealth.mockReset()
  })

  it('shows verified Hermes dashboard status after launch', async () => {
    readHermesRuntimeHealth.mockResolvedValue({
      health: {
        url: 'http://127.0.0.1:9119',
        runtimeAvailable: true,
        runtimeReady: true,
        runtimeLaunched: true,
        runtimeIdentity: null,
        runtimeVersion: null,
        sessionDirectory: null,
        spotsCount: 1,
        spotsActive: 1,
        probes: { port: true, identity: true, version: true, sessionDirectory: true, atLeastOneSpot: true },
        failure: null,
      },
      surfaceAvailable: true,
    })
    ensureHermesRuntimeSpots.mockResolvedValue({
      window_label: 'arda-workstation-hermes_runtime_workstation',
      url: 'http://127.0.0.1:9119',
      port: 9119,
      launched: true,
      ready: true,
      identity: null,
      spotCount: 1,
      failure: null,
    })

    renderModule()

    expect(await screen.findByText('Live Hermes dashboard embedded')).toBeInTheDocument()
    expect(await screen.findByText(/Hermes runtime ready at http:\/\/127\.0\.0\.1:9119/i)).toBeInTheDocument()
    expect(screen.getByText('ready')).toBeInTheDocument()
    expect(screen.getByText('owned process')).toBeInTheDocument()
    expect(screen.getByText('http://127.0.0.1:9119')).toBeInTheDocument()
  })

  it('surfaces launch errors and keeps diagnostics visible', async () => {
    readHermesRuntimeHealth.mockResolvedValue({
      health: {
        url: null,
        runtimeAvailable: false,
        runtimeReady: false,
        runtimeLaunched: false,
        runtimeIdentity: null,
        runtimeVersion: null,
        sessionDirectory: null,
        spotsCount: 0,
        spotsActive: 0,
        probes: { port: false, identity: false, version: false, sessionDirectory: false, atLeastOneSpot: false },
        failure: 'Port 9119 is already listening, but it did not identify as Hermes dashboard',
      },
      surfaceAvailable: false,
    })
    ensureHermesRuntimeSpots.mockRejectedValue(
      new Error('Port 9119 is already listening, but it did not identify as Hermes dashboard'),
    )

    renderModule()

    expect(await screen.findByText('Hermes dashboard unavailable')).toBeInTheDocument()
    expect(screen.getByText('error')).toBeInTheDocument()
    expect(screen.getByText(/Could not load Hermes dashboard/)).toBeInTheDocument()
  })
})
