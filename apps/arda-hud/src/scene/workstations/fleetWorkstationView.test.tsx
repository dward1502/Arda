import { fireEvent, render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { FleetFocusedWorkstationView } from './fleetWorkstationView'
import { createEmptyFleetViewModel, type FleetViewModel } from './viewModels'

function fleetModel(): FleetViewModel {
  return {
    ...createEmptyFleetViewModel(),
    status: 'attention',
    providers: [{
      providerId: 'linked-provider', providerName: 'Linked', accessTier: 'local', qualityBand: 'local',
      enabled: true, healthy: true, models: [], avgLatencyMs: 10, activeConnections: 1,
    }],
    nodes: [
      { id: 'core', displayName: 'Core Node', hostname: 'core', nodeClass: 'core_compute', online: true, enrollmentStatus: 'active', expectedModels: ['LFM'], hardwareSummary: '32Gi · 1 GPU' },
      { id: 'backbone', displayName: 'Backbone Node', hostname: 'backbone', nodeClass: 'backbone_compute', online: false, enrollmentStatus: 'active', expectedModels: ['Qwen'], hardwareSummary: '64Gi · 2 GPUs' },
    ],
    backboneNodeId: 'backbone',
    sources: ['runtime', 'nodes', 'models', 'health', 'hardware', 'backbone'].map((name) => ({
      id: `fleet_${name}`,
      label: `Fleet ${name}`,
      freshness: { status: 'fresh' as const },
    })),
  }
}

describe('FleetFocusedWorkstationView', () => {
  it('uses topology and node selection as the Fleet information architecture', () => {
    render(<FleetFocusedWorkstationView fleetViewModel={fleetModel()} />)

    expect(screen.getByLabelText('Fleet topology rack line')).toBeInTheDocument()
    const detail = screen.getByRole('heading', { name: 'Selected Node' }).parentElement!
    expect(within(detail).getByText('Core Node')).toBeInTheDocument()

    fireEvent.click(within(screen.getByRole('region', { name: 'Fleet node index' })).getByRole('button', { name: /Backbone Nodeoffline/i }))

    expect(within(detail).getByText('Backbone Node')).toBeInTheDocument()
    expect(within(detail).getByText('64Gi · 2 GPUs')).toBeInTheDocument()
    expect(within(detail).getByText('Qwen')).toBeInTheDocument()
    expect(within(detail).getByText('primary')).toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'Lane Ownership' })).not.toBeInTheDocument()
    expect(screen.queryByRole('heading', { name: 'Providers' })).not.toBeInTheDocument()
  })

  it('distinguishes unavailable node data from a loaded zero-node projection', () => {
    const missing = { ...fleetModel(), nodes: [], sources: [{ id: 'fleet_nodes', label: 'Fleet Nodes', freshness: { status: 'missing' as const } }] }
    const loadedEmpty = { ...fleetModel(), nodes: [], sources: [{ id: 'fleet_nodes', label: 'Fleet Nodes', freshness: { status: 'fresh' as const } }] }
    const { rerender } = render(<FleetFocusedWorkstationView fleetViewModel={missing} />)
    expect(screen.getByText('Node projection unavailable.')).toBeInTheDocument()

    rerender(<FleetFocusedWorkstationView fleetViewModel={loadedEmpty} />)
    expect(screen.getByText('Node projection loaded: zero nodes.')).toBeInTheDocument()
  })
})
