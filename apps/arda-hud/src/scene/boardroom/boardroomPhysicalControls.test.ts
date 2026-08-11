// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { getBoardroomSpatialZone } from './boardroomSpatialLayout'
import {
  BOARDROOM_PHYSICAL_CONTROL_ACTIONS,
  deriveBoardroomPhysicalControlState,
  resolveBoardroomPhysicalControlInteraction,
} from './boardroomPhysicalControls'

describe('boardroom expanded physical controls', () => {
  it('types every rendered command-core and tray control with authority and verification metadata', () => {
    expect(BOARDROOM_PHYSICAL_CONTROL_ACTIONS.map((action) => action.id)).toEqual([
      'service_health_status',
      'open_settings',
      'open_hermes_cli',
      'open_hermes_dashboard',
      'open_approval_queue',
      'open_command_core',
      'open_emergency_stop',
      'open_route_selector',
      'enter_world',
    ])
    expect(BOARDROOM_PHYSICAL_CONTROL_ACTIONS).toContainEqual(expect.objectContaining({
      id: 'service_health_status',
      zoneId: 'boardroom.button.service_health',
      label: 'Service Health',
      shortLabel: 'HEALTH',
      authority: 'read_only',
      targetZoneId: 'fleet_and_backbone',
      verificationPath: 'fleetViewModel.status',
    }))
    for (const action of BOARDROOM_PHYSICAL_CONTROL_ACTIONS) {
      expect(action.authority).toMatch(/^(read_only|operator_confirmed|approval_required)$/)
      expect(action.verificationPath.length).toBeGreaterThan(0)
      expect(action.targetZoneId.length).toBeGreaterThan(0)
    }

    expect(getBoardroomSpatialZone('boardroom.button.service_health')).toMatchObject({
      kind: 'physical_button',
      interaction: 'open_workstation',
      binding: 'fleet_and_backbone',
    })
  })

  it('keeps stop/cancel approval-gated and all non-source controls actionable', () => {
    const stop = BOARDROOM_PHYSICAL_CONTROL_ACTIONS.find((action) => action.id === 'open_emergency_stop')!
    expect(stop).toMatchObject({ authority: 'approval_required', targetZoneId: 'governance_guardhouse' })
    expect(deriveBoardroomPhysicalControlState(stop.id, null)).toEqual({
      state: 'attention',
      statusLabel: 'CONFIRM',
      disabled: false,
      error: null,
    })

    for (const action of BOARDROOM_PHYSICAL_CONTROL_ACTIONS.filter((candidate) => candidate.id !== 'service_health_status')) {
      expect(deriveBoardroomPhysicalControlState(action.id, null).disabled).toBe(false)
    }
  })

  it('fails closed when health state is unavailable and exposes the reason', () => {
    expect(deriveBoardroomPhysicalControlState('service_health_status', null)).toEqual({
      state: 'error',
      statusLabel: 'NO DATA',
      disabled: true,
      error: 'Fleet health projection unavailable',
    })
  })

  it('maps projected fleet health into visible ready and attention states', () => {
    expect(deriveBoardroomPhysicalControlState('service_health_status', 'ok')).toEqual({
      state: 'ready',
      statusLabel: 'NOMINAL',
      disabled: false,
      error: null,
    })
    expect(deriveBoardroomPhysicalControlState('service_health_status', 'attention')).toEqual({
      state: 'attention',
      statusLabel: 'ATTN',
      disabled: false,
      error: null,
    })
  })

  it('returns visible fail-closed feedback instead of silently ignoring unavailable controls', () => {
    const action = BOARDROOM_PHYSICAL_CONTROL_ACTIONS[0]
    const state = deriveBoardroomPhysicalControlState(action.id, null)

    expect(resolveBoardroomPhysicalControlInteraction(action, state)).toEqual({
      kind: 'blocked',
      message: 'Service Health unavailable: Fleet health projection unavailable',
      verificationPath: 'fleetViewModel.status',
    })
  })

  it('returns a typed dispatch receipt for an available read-only control', () => {
    const action = BOARDROOM_PHYSICAL_CONTROL_ACTIONS[0]
    const state = deriveBoardroomPhysicalControlState(action.id, 'ok')

    expect(resolveBoardroomPhysicalControlInteraction(action, state)).toEqual({
      kind: 'dispatch',
      actionId: 'service_health_status',
      targetZoneId: 'fleet_and_backbone',
      message: 'Opening Service Health.',
      verificationPath: 'fleetViewModel.status',
    })
  })

  it('keeps the health control compact and adjacent to the other embedded controls', () => {
    const health = getBoardroomSpatialZone('boardroom.button.service_health')!
    const settings = getBoardroomSpatialZone('boardroom.button.settings')!
    const healthRightEdge = health.position[0] + health.size[0] / 2
    const settingsLeftEdge = settings.position[0] - settings.size[0] / 2

    expect(settingsLeftEdge).toBeGreaterThan(healthRightEdge)
    expect(settings.position[0] - health.position[0]).toBeLessThan(0.5)
    expect(health.size).toEqual([0.14, 0.045, 0.16])
  })
})
