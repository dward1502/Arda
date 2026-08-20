import type { MirromereSurface } from '@arda/mirromere-ui'

export interface DisplayDescriptor {
  id: string
  name: string
  is_primary: boolean
  position: [number, number]
  size: [number, number]
  scale_factor: number
}

export interface DisplayState {
  displays: DisplayDescriptor[]
  selected_display_id: string | null
  projected: boolean
  veil_reason: string | null
}

export function isProjectionVeiled(
  displayState: DisplayState | null,
  surface: MirromereSurface | null,
): boolean {
  return !displayState?.projected || Boolean(displayState.veil_reason) || !surface
}

export function selectableDisplays(displayState: DisplayState | null): DisplayDescriptor[] {
  return displayState?.displays.filter(display => !display.is_primary) ?? []
}
