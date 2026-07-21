// sigil: REPAIR
import { invoke } from '@tauri-apps/api/core'

/**
 * Tauri environment detection and safe invocation wrapper
 */

export function isTauriEnvironment(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

export class TauriUnavailableError extends Error {
  readonly code = 'TAURI_UNAVAILABLE'
  readonly operation: string
  readonly command?: string
  readonly environment = 'browser'

  constructor(operation: string, command?: string) {
    super(command
      ? `${operation} cannot invoke '${command}' outside the Tauri desktop runtime`
      : `${operation} requires the Tauri desktop runtime`)
    this.name = 'TauriUnavailableError'
    this.operation = operation
    this.command = command
  }
}

export function createTauriUnavailableError(operation: string, command?: string): TauriUnavailableError {
  return new TauriUnavailableError(operation, command)
}

export function requireTauri(operation: string): void {
  if (!isTauriEnvironment()) {
    throw createTauriUnavailableError(operation)
  }
}

export function safeTauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (!isTauriEnvironment()) {
    return Promise.reject(createTauriUnavailableError('safeTauriInvoke', command))
  }

  return invoke(command, args) as Promise<T>
}
