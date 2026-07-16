// sigil: REPAIR
import type { BoardroomSurfaceLayout } from '../../../lib/boardroomSlotSettings'

export type ThemeId = 'cyberpunk' | 'gibson2' | 'eva'

export type ModuleId =
  | 'executive_overview'
  | 'operating_surface'
  | 'section_focus'
  | 'human_realm'
  | 'systems'
  | 'governance_controls'
  | 'operations_and_packages'
  | 'hermes_dashboard'
  | 'planning'
  | 'learning_loop'
  | 'business'
  | 'personal_growth'
  | 'culture_and_art'
  | 'service_embed'
  | 'media_library'
  | 'settings'

export interface ThemeOption {
  id: ThemeId
  label: string
}

export type ViewMode = 'boardroom' | 'world' | 'panel'

export type OperatingSurfaceNavKey =
  | 'Now'
  | 'Work'
  | 'Decisions'
  | 'Knowledge'
  | 'Health'
  | 'Business'
  | 'Evidence'
  | 'Settings'

export interface MonitorAssignment {
  slot: string
  label: string
  sourceZoneId?: string
  componentId?: string
  role?: 'upper_monitor' | 'desk_surface' | 'control_surface' | 'world_district' | 'world_terminal'
  adapterType?: string
  previewMode?: string
  focusMode?: string
  refreshMs?: number
  widgetCount?: number
  embedUrl?: string | null
  surfaceLayout?: BoardroomSurfaceLayout
}
