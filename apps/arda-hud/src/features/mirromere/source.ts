import { safeTauriInvoke } from '../../lib/tauriGuard'
import { parseMirromereSurface, type MirromereSurface } from './types'

export type MirromereInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

export async function loadMirromereSurface(
  invoke: MirromereInvoke = safeTauriInvoke,
  now = new Date(),
): Promise<MirromereSurface> {
  const value = await invoke<unknown>('get_mirromere_surface', { displayRole: 'hud_aperture' })
  const surface = parseMirromereSurface(value, now)
  if (surface.source_mode !== 'runtime') {
    throw new Error('fixture Mirromere surface cannot enter the runtime source path')
  }
  return surface
}
