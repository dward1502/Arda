// sigil: REPAIR
import { describe, expect, it, vi } from 'vitest'
import { getBoardroomSpatialZone } from './boardroomSpatialLayout'
import {
  BOARDROOM_COMMAND_CORE_CONTROL_BANKS,
  BOARDROOM_PHYSICAL_CONTROL_ACTIONS,
  deriveBoardroomPhysicalControlState,
  dispatchBoardroomCommandCoreControl,
  getBoardroomPhysicalControlAction,
  resolveBoardroomPhysicalControlInteraction,
} from './boardroomPhysicalControls'

describe('boardroom expanded physical controls', () => {
  it('types every command-core control and retained health signal with authority and verification metadata', () => {
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
    expect(BOARDROOM_COMMAND_CORE_CONTROL_BANKS).toEqual({
      command: ['open_approval_queue', 'open_emergency_stop', 'open_route_selector', 'enter_world'],
      utility: ['open_settings', 'open_hermes_cli', 'open_hermes_dashboard'],
    })
    expect(BOARDROOM_PHYSICAL_CONTROL_ACTIONS).toContainEqual(expect.objectContaining({
      id: 'open_settings',
      zoneId: 'boardroom.control.center.settings',
      shortLabel: 'SETTINGS',
    }))
    expect(BOARDROOM_PHYSICAL_CONTROL_ACTIONS).toContainEqual(expect.objectContaining({
      id: 'open_hermes_cli',
      zoneId: 'boardroom.control.center.terminal',
      shortLabel: 'TERMINAL',
    }))
    expect(BOARDROOM_PHYSICAL_CONTROL_ACTIONS).toContainEqual(expect.objectContaining({
      id: 'open_hermes_dashboard',
      zoneId: 'boardroom.control.center.hermes',
      shortLabel: 'HERMES',
    }))
    for (const action of BOARDROOM_PHYSICAL_CONTROL_ACTIONS) {
      expect(action.authority).toMatch(/^(read_only|operator_confirmed|approval_required)$/)
      expect(action.verificationPath.length).toBeGreaterThan(0)
      expect(action.targetZoneId.length).toBeGreaterThan(0)
    }

    expect(getBoardroomPhysicalControlAction('service_health_status')).toMatchObject({
      zoneId: 'boardroom.control.center.health',
      authority: 'read_only',
      targetZoneId: 'fleet_and_backbone',
    })
    expect(getBoardroomSpatialZone('boardroom.button.service_health')).toBeNull()
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

  it('retires the detached control-row zones after moving utilities to the command core', () => {
    expect(getBoardroomSpatialZone('boardroom.button.service_health')).toBeNull()
    expect(getBoardroomSpatialZone('boardroom.button.settings')).toBeNull()
    expect(getBoardroomSpatialZone('boardroom.button.hermes_cli')).toBeNull()
    expect(getBoardroomSpatialZone('boardroom.button.hermes')).toBeNull()
  })

  it.each([
    ['open_settings', 'onOpenSettings'],
    ['open_hermes_cli', 'onOpenHermesCli'],
    ['open_hermes_dashboard', 'onOpenHermesDashboard'],
  ] as const)('dispatches the relocated %s callback exactly once', (actionId, expectedHandler) => {
    const handlers = {
      onOpenSettings: vi.fn(),
      onOpenHermesCli: vi.fn(),
      onOpenHermesDashboard: vi.fn(),
      onEnterWorld: vi.fn(),
      onOpenWorkstation: vi.fn(),
    }

    dispatchBoardroomCommandCoreControl(getBoardroomPhysicalControlAction(actionId), handlers)

    expect(handlers[expectedHandler]).toHaveBeenCalledOnce()
    expect(Object.values(handlers).reduce((count, handler) => count + handler.mock.calls.length, 0)).toBe(1)
  })
})
