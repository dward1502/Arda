export type FloatingWorkstationFocusOrigins = Map<string, HTMLElement>

export function rememberFloatingWorkstationFocusOrigin(
  origins: FloatingWorkstationFocusOrigins,
  workstationId: string,
): void {
  if (origins.has(workstationId)) return
  const activeElement = document.activeElement
  if (activeElement instanceof HTMLElement && activeElement !== document.body) {
    origins.set(workstationId, activeElement)
  }
}

export function focusFloatingWorkstation(workstationId: string): boolean {
  const workstation = document.querySelector<HTMLElement>(
    `[data-workstation-id="${workstationId}"]`,
  )
  if (!workstation) return false
  workstation.focus()
  return document.activeElement === workstation
}

export function restoreFloatingWorkstationFocus(
  origins: FloatingWorkstationFocusOrigins,
  workstationId: string,
): boolean {
  const origin = origins.get(workstationId)
  origins.delete(workstationId)
  if (!origin?.isConnected) return false
  origin.focus()
  return document.activeElement === origin
}
