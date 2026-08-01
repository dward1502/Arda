// sigil: REPAIR
export interface TerminalStartupOperations {
  announce: () => void
  settleLayout: () => Promise<void>
  fit: () => Promise<void>
  createShell: () => Promise<void>
  refresh: () => void
  focus: () => void
  startReading: () => void
}

export async function initializeTerminalSession({
  announce,
  settleLayout,
  fit,
  createShell,
  refresh,
  focus,
  startReading,
}: TerminalStartupOperations): Promise<void> {
  announce()
  await settleLayout()
  await fit()
  await createShell()
  await settleLayout()
  await fit()
  refresh()
  focus()
  startReading()
}

export function waitForTerminalLayout(): Promise<void> {
  return new Promise((resolve) => {
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => resolve())
    })
  })
}
