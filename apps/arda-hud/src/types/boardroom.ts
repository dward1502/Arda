export type BoardroomZoneInteraction = 'open_workstation' | 'open_settings' | 'open_hermes' | 'transition_world' | 'presence_focus' | 'display_only'
export type BoardroomPreviewMode = 'monitor_surface' | 'desk_surface' | 'button' | 'portal' | 'presence'
export type BoardroomVec3 = [number, number, number]

export interface BoardroomSpatialZone {
  id: string
  label: string
  kind: string
  interaction: BoardroomZoneInteraction
  binding?: string
  slotId?: string
  position: BoardroomVec3
  rotation: BoardroomVec3
  size: BoardroomVec3
  color: string
  primary?: boolean
  detail?: string
}
