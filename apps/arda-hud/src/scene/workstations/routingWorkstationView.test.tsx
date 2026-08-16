import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { RoutingViewModel } from './viewModels'
import { RoutingFocusedWorkstationView } from './routingWorkstationView'

const model: RoutingViewModel = {
  roleId: 'routing',
  title: 'Routing + Communications',
  status: 'attention',
  summary: ['2/3 providers healthy'],
  metrics: [],
  sources: [
    { id: 'operator_runtime_status', label: 'Operator Runtime', path: 'core/state/operator_runtime_status.json', freshness: { status: 'fresh', timestamp: '2026-08-16T11:59:00Z' } },
    { id: 'manwe_router', label: 'CHARON Router', path: 'core/state/manwe_router.json', freshness: { status: 'missing', timestamp: null } },
  ],
  actions: [
    { id: 'arda.chronos_run_provider_checks', label: 'Run Provider Checks', safety: 'read_only' },
    { id: 'charon.refresh_provider_intelligence', label: 'Refresh Provider Intelligence', safety: 'read_only' },
  ],
  previewKinds: [],
  focusedCapabilities: [],
  rawDisclosure: false,
  providers: [
    { providerId: 'groq', providerName: 'Groq', healthy: true, enabled: true, activeConnections: 2, modelCount: 4, accessTier: 'cloud', qualityBand: 'high' },
    { providerId: 'openai', providerName: 'OpenAI', healthy: false, enabled: true, activeConnections: 0, modelCount: 2, accessTier: 'cloud', qualityBand: 'high' },
  ],
  lanes: [
    { lane: 'interactive', label: 'Normal Chat', providerId: 'groq', modelId: 'fast', routeClass: 'cloud', reason: 'low latency', headroom: 4, softCap: 6, avgLatencyMs: 80, successes: 9, failures: 1 },
  ],
  routeHistory: { successes: 9, failures: 1 },
  budgetPressure: { highestLevel: 'warning', cooldownTotal: 1, exhaustedTotal: 0 },
  communicationPathways: [{ id: 'discord', label: 'Discord', state: 'delivered', receipts: 1 }],
}

describe('RoutingFocusedWorkstationView', () => {
  it('renders routing lanes, provider detail, communications, truth states, and registered read-only actions', () => {
    const onRunAction = vi.fn()
    render(<RoutingFocusedWorkstationView model={model} busyActionId={null} onRunAction={onRunAction} />)

    expect(screen.getByRole('heading', { name: 'Routing + Communications' })).toBeTruthy()
    const laneRegion = screen.getByRole('region', { name: 'Routing lane ownership' })
    expect(within(laneRegion).getByRole('button', { name: /Normal Chat.*groq/i })).toBeTruthy()
    expect(screen.getByText('low latency')).toBeTruthy()
    expect(screen.getByRole('region', { name: 'Communication pathways' })).toHaveTextContent('Discord')
    expect(screen.getByText(/CHARON Router: missing/)).toBeTruthy()

    fireEvent.click(screen.getByRole('button', { name: 'Run Provider Checks' }))
    fireEvent.click(screen.getByRole('button', { name: 'Refresh Provider Intelligence' }))
    expect(onRunAction).toHaveBeenNthCalledWith(1, 'arda.chronos_run_provider_checks')
    expect(onRunAction).toHaveBeenNthCalledWith(2, 'charon.refresh_provider_intelligence')
  })

  it('changes selected lane detail through semantic lane controls', () => {
    const secondLane: RoutingViewModel['lanes'][number] = { ...model.lanes[0], lane: 'execution', label: 'High Code', providerId: 'openai', modelId: 'coder', reason: 'tool capability' }
    render(<RoutingFocusedWorkstationView model={{ ...model, lanes: [...model.lanes, secondLane] }} busyActionId={null} onRunAction={() => undefined} />)

    fireEvent.click(screen.getByRole('button', { name: /High Code.*openai/i }))
    expect(screen.getByRole('region', { name: 'Selected routing lane' })).toHaveTextContent('tool capability')
    expect(screen.getByRole('region', { name: 'Selected routing lane' })).toHaveTextContent('coder')
  })
})
