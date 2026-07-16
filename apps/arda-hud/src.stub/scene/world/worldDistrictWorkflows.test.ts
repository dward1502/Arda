// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import type { WorldSurfaceLayout } from '../../lib/worldSurfaceSettings'
import type { SceneZoneDefinition } from '../systems/runtimeTypes'
import { resolveWorldDistrictWorkflow, resolveWorldDistrictWorkflows } from './worldDistrictWorkflows'

const routingZone: SceneZoneDefinition = {
  id: 'routing_and_comms',
  title: 'Routing & Comms',
  scene: 'world',
  owner: 'CHARON/HERMES',
  status: 'ok',
  anchorIds: ['routing-terminal'],
  surfaceIds: [],
  workstationIds: ['routing_and_comms-workstation'],
  sourceIds: ['routing_and_comms'],
}

const routingLayout: WorldSurfaceLayout = {
  enabled: true,
  adapter_type: 'component_grid',
  preview: {
    mode: 'component_grid',
    refresh_ms: 3000,
    widgets: [
      { id: 'routing.signal', kind: 'metric_strip', title: 'Signals', data_binding: 'routing.summary', grid_area: 'top' },
      { id: 'routing.health', kind: 'status_grid', title: 'Health', data_binding: 'routing.status', grid_area: 'main' },
    ],
  },
  focus: {
    mode: 'native_window',
    target: 'routing_and_comms',
    refresh_ms: 1000,
  },
  embed: {
    url: null,
    allow_inline: false,
  },
}

describe('world district workflows', () => {
  it('resolves click target, safe actions, gated actions, and surface metadata for a district', () => {
    const workflow = resolveWorldDistrictWorkflow(routingZone, {
      district_communications: routingLayout,
    })

    expect(workflow.districtId).toBe('district_communications')
    expect(workflow.openTargetZoneId).toBe('routing_and_comms')
    expect(workflow.adapterType).toBe('component_grid')
    expect(workflow.focusMode).toBe('native_window')
    expect(workflow.previewWidgetCount).toBe(2)
    expect(workflow.safeActions.map((action) => action.id)).toContain('open-routing-workstation')
    expect(workflow.safeActions.map((action) => action.id)).toContain('inspect-routing-health')
    expect(workflow.gatedActions.map((action) => action.id)).toContain('draft-provider-reroute')
  })

  it('returns a safe fallback workflow for an unmapped world zone', () => {
    const workflow = resolveWorldDistrictWorkflow({
      ...routingZone,
      id: 'future_world_zone',
      title: 'Future Zone',
      owner: 'PROMETHEUS',
      sourceIds: ['future_section'],
      workstationIds: [],
    })

    expect(workflow.districtId).toBe('future_world_zone')
    expect(workflow.adapterType).toBe('unmapped')
    expect(workflow.openTargetZoneId).toBe('future_section')
    expect(workflow.safeActions[0]?.guard).toBe('safe')
    expect(workflow.acceptanceSummary).toBe('district contract missing')
  })

  it('resolves workflows for world zones only', () => {
    const workflows = resolveWorldDistrictWorkflows([
      routingZone,
      { ...routingZone, id: 'boardroom_zone', scene: 'boardroom' },
    ], {
      district_communications: routingLayout,
    })

    expect(Object.keys(workflows)).toEqual(['routing_and_comms'])
  })
})
