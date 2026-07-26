type Json = Record<string, unknown> | Array<unknown> | string | number | boolean | null

declare module '@tauri-apps/api/tauri' {
  export function invoke<T = Json>(cmd: string, args?: Record<string, Json>): Promise<T>
}

declare module '@tauri-apps/api/core' {
  export function invoke<T = Json>(cmd: string, args?: Record<string, Json>): Promise<T>
}

declare module '@tauri-apps/api/plugin-opener' {
  export interface OpenOptions {
    withExecutor?: boolean
  }
  export function open(path: string, options?: Partial<OpenOptions>): Promise<void>
}

declare module '@tauri-apps/api/path' {
  export function resourceDir(): Promise<string>
}

declare module '@tauri-apps/api' {
  export function convertFileSrc(path: string): string
}
