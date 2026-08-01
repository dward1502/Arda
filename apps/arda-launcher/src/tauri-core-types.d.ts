declare module '@tauri-apps/api/core' {
  export interface RegistryResult {
    loaded: boolean
    gate_status: 'pass' | 'warn' | 'fail'
    track_count: number
    error?: string
  }

  export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>
}
