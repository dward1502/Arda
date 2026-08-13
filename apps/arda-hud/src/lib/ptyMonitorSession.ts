export interface PtyCaptureDescriptor {
  sessionId: string
  owner: string
  revision: number
  outputRevision: number
  processId: number | null
  rows: number
  cols: number
  output: string
}

export async function startPtyMonitorSession(
  sessionId: string,
  owner: string,
  command: string,
): Promise<PtyCaptureDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<PtyCaptureDescriptor>('start_pty_capture', {
    request: { sessionId, owner, command },
  })
}

export async function getPtyMonitorSession(sessionId: string): Promise<PtyCaptureDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<PtyCaptureDescriptor>('get_pty_capture_status', { sessionId })
}

export async function writePtyMonitorSession(
  sessionId: string,
  owner: string,
  expectedRevision: number,
  data: string,
): Promise<PtyCaptureDescriptor> {
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<PtyCaptureDescriptor>('write_pty_capture', {
    request: { sessionId, owner, expectedRevision, data },
  })
}

export async function stopPtyMonitorSession(sessionId: string, owner: string): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke<void>('stop_pty_capture', { request: { sessionId, owner } })
}
