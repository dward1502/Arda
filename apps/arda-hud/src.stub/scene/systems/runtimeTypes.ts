// sigil: REPAIR
export type SceneMode = 'boardroom' | 'world' | 'workstation_focus'

export type AnchorType =
  | 'monitor'
  | 'console'
  | 'control'
  | 'hologram'
  | 'gate'
  | 'district'
  | 'terminal'
  | 'workstation_spawn'

export type AnchorActivationBehavior =
  | 'focus'
  | 'open_workstation'
  | 'transition_world'
  | 'transition_boardroom'
  | 'open_terminal'

export interface SceneAnchorDefinition {
  id: string
  scene: 'boardroom' | 'world'
  type: AnchorType
  label: string
  zoneId: string
  activationBehavior: AnchorActivationBehavior
  dataBinding: string[]
}

export interface SceneZoneDefinition {
  id: string
  title: string
  scene: 'boardroom' | 'world'
  owner: string
  status: string
  anchorIds: string[]
  surfaceIds: string[]
  workstationIds: string[]
  sourceIds: string[]
}

export interface WorkstationManifestDefinition {
  id: string
  title: string
  sourceZoneId: string
  entryAnchorId: string
  moduleIds: string[]
  presentationModes: Array<'in_scene' | 'native_window'>
}

export interface CameraStateDefinition {
  id: string
  scene: 'boardroom' | 'world'
  purpose: string
}

export interface SceneInteractionEvent {
  anchorId: string
  interactionType: 'hover_reveal' | 'focus' | 'activate' | 'transition' | 'spawn_workstation' | 'open_native_window' | 'return_to_scene'
  resultState: string
}
