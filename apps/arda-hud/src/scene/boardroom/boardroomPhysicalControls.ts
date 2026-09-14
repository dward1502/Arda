// sigil: REPAIR
import type { WorkstationStatus } from '../workstations/viewModels'

export type BoardroomPhysicalControlAuthority = 'read_only' | 'operator_confirmed' | 'approval_required'
export type BoardroomPhysicalControlVisualState = 'ready' | 'attention' | 'error'

export interface BoardroomPhysicalControlAction {
  id: string
  zoneId: string
  label: string
  shortLabel: string
  authority: BoardroomPhysicalControlAuthority
  targetZoneId: string
  verificationPath: string
}

export interface BoardroomPhysicalControlState {
  state: BoardroomPhysicalControlVisualState
  statusLabel: string
  disabled: boolean
  error: string | null
}

export type BoardroomPhysicalControlInteraction =
  | {
      kind: 'blocked'
      message: string
      verificationPath: string
    }
  | {
      kind: 'dispatch'
      actionId: string
      targetZoneId: string
      message: string
      verificationPath: string
    }

export const BOARDROOM_PHYSICAL_CONTROL_ACTIONS: BoardroomPhysicalControlAction[] = [
  {
    id: 'service_health_status',
    zoneId: 'boardroom.control.center.health',
    label: 'Service Health',
    shortLabel: 'HEALTH',
    authority: 'read_only',
    targetZoneId: 'fleet_and_backbone',
    verificationPath: 'fleetViewModel.status',
  },
  {
    id: 'open_settings',
    zoneId: 'boardroom.control.center.settings',
    label: 'Settings',
    shortLabel: 'SETTINGS',
    authority: 'operator_confirmed',
    targetZoneId: 'settings',
    verificationPath: 'settings workspace load status',
  },
  {
    id: 'open_hermes_cli',
    zoneId: 'boardroom.control.center.terminal',
    label: 'Hermes CLI',
    shortLabel: 'TERMINAL',
    authority: 'operator_confirmed',
    targetZoneId: 'hermes_cli_window',
    verificationPath: 'native terminal window lifecycle',
  },
  {
    id: 'open_hermes_dashboard',
    zoneId: 'boardroom.control.center.hermes',
    label: 'Hermes Dashboard',
    shortLabel: 'HERMES',
    authority: 'read_only',
    targetZoneId: 'hermes_dashboard_window',
    verificationPath: 'verified Hermes dashboard runtime status',
  },
  {
    id: 'open_approval_queue',
    zoneId: 'boardroom.control.center.go',
    label: 'Approval Queue',
    shortLabel: 'GO',
    authority: 'read_only',
    targetZoneId: 'planning_and_queue',
    verificationPath: 'planning and queue source projection',
  },
  {
    id: 'open_command_core',
    zoneId: 'boardroom.control.center.screen',
    label: 'ARDA Control',
    shortLabel: 'CONTROL',
    authority: 'read_only',
    targetZoneId: 'sovereign_world',
    verificationPath: 'sovereign world source projection',
  },
  {
    id: 'open_emergency_stop',
    zoneId: 'boardroom.control.center.stop',
    label: 'Gated Stop / Cancel Review',
    shortLabel: 'STOP',
    authority: 'approval_required',
    targetZoneId: 'governance_guardhouse',
    verificationPath: 'governance decision receipt before mutation',
  },
  {
    id: 'open_route_selector',
    zoneId: 'boardroom.control.center.route',
    label: 'Route Selector',
    shortLabel: 'ROUTE',
    authority: 'operator_confirmed',
    targetZoneId: 'routing_and_comms',
    verificationPath: 'routing provider capability receipt',
  },
  {
    id: 'enter_world',
    zoneId: 'boardroom.control.center.world',
    label: 'World View',
    shortLabel: 'WORLD',
    authority: 'operator_confirmed',
    targetZoneId: 'sovereign_world',
    verificationPath: 'scene transition state',
  },
]

export const BOARDROOM_COMMAND_CORE_CONTROL_BANKS = {
  command: ['open_approval_queue', 'open_emergency_stop', 'open_route_selector', 'enter_world'],
  utility: ['open_settings', 'open_hermes_cli', 'open_hermes_dashboard'],
} as const satisfies Record<'command' | 'utility', readonly string[]>

export interface BoardroomCommandCoreControlHandlers {
  onOpenSettings: () => void
  onOpenHermesCli: () => void
  onOpenHermesDashboard: () => void
  onEnterWorld: () => void
  onOpenWorkstation: (zoneId: string) => void
}

export function dispatchBoardroomCommandCoreControl(
  action: BoardroomPhysicalControlAction,
  handlers: BoardroomCommandCoreControlHandlers,
): void {
  if (action.id === 'open_settings') {
    handlers.onOpenSettings()
    return
  }
  if (action.id === 'open_hermes_cli') {
    handlers.onOpenHermesCli()
    return
  }
  if (action.id === 'open_hermes_dashboard') {
    handlers.onOpenHermesDashboard()
    return
  }
  if (action.id === 'enter_world') {
    handlers.onEnterWorld()
    return
  }
  handlers.onOpenWorkstation(action.targetZoneId)
}

export function getBoardroomPhysicalControlAction(actionId: string): BoardroomPhysicalControlAction {
  const action = BOARDROOM_PHYSICAL_CONTROL_ACTIONS.find((candidate) => candidate.id === actionId)
  if (!action) throw new Error(`Unknown boardroom physical control: ${actionId}`)
  return action
}

export function deriveBoardroomPhysicalControlState(
  actionId: string,
  sourceStatus: WorkstationStatus | null | undefined,
): BoardroomPhysicalControlState {
  const action = BOARDROOM_PHYSICAL_CONTROL_ACTIONS.find((candidate) => candidate.id === actionId)
  if (!action) {
    return {
      state: 'error',
      statusLabel: 'INVALID',
      disabled: true,
      error: `Unknown boardroom physical control: ${actionId}`,
    }
  }

  if (actionId !== 'service_health_status') {
    return action.authority === 'approval_required'
      ? { state: 'attention', statusLabel: 'CONFIRM', disabled: false, error: null }
      : { state: 'ready', statusLabel: 'READY', disabled: false, error: null }
  }

  if (sourceStatus === 'ok') {
    return { state: 'ready', statusLabel: 'NOMINAL', disabled: false, error: null }
  }
  if (sourceStatus === 'attention') {
    return { state: 'attention', statusLabel: 'ATTN', disabled: false, error: null }
  }

  return {
    state: 'error',
    statusLabel: 'NO DATA',
    disabled: true,
    error: 'Fleet health projection unavailable',
  }
}

export function resolveBoardroomPhysicalControlInteraction(
  action: BoardroomPhysicalControlAction,
  state: BoardroomPhysicalControlState,
): BoardroomPhysicalControlInteraction {
  if (state.disabled || state.error) {
    return {
      kind: 'blocked',
      message: `${action.label} unavailable: ${state.error ?? 'Control is disabled'}`,
      verificationPath: action.verificationPath,
    }
  }

  return {
    kind: 'dispatch',
    actionId: action.id,
    targetZoneId: action.targetZoneId,
    message: `Opening ${action.label}.`,
    verificationPath: action.verificationPath,
  }
}
