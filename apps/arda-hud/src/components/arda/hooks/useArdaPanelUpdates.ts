// sigil: REPAIR
import { getArdaRuntimeSurface, getNumenorPath, type ArdaHudSettings, type JsonRecord } from '../../../lib/weathertop'
import { createPanelWatcher, getActivePanelWatcher } from '../../../lib/ardaPanelWatch'
import type { ArdaPanel, ArdaPanelChangeEvent } from '../../../lib/ardaPanelWatch'

export type { ArdaPanel, ArdaPanelChangeEvent } from '../../../lib/ardaPanelWatch'

export interface ArdaPanelSlice {
  lastEvent?: ArdaPanelChangeEvent
  updatedAt?: string
}

export function useArdaPanelUpdates(panel: ArdaPanel): ArdaPanelSlice {
  const [slice, setSlice] = useState<ArdaPanelSlice>({})

  useEffect(() => {
    const watcher = createPanelWatcher(panel)
    const watcherRef = getActivePanelWatcher(panel)

    const handleChange = (event: ArdaPanelChangeEvent) => {
      setSlice({ lastEvent: event, updatedAt: new Date().toISOString() })
    }

    watcherRef.current?.on('change', handleChange)

    return () => {
      watcherRef.current?.off('change', handleChange)
      watcherRef.current?.dispose()
      watcherRef.current = null
      ;(createPanelWatcher as (panel: ArdaPanel) => ArdaPanelWatcher)(panel)
    }
  }, [panel])

  return slice
}

class ArdaPanelWatcher {
  private listeners = new Set<(event: ArdaPanelChangeEvent) => void>()
  readonly path: string

  constructor(private readonly panel: ArdaPanel, private readonly numenorPath: string) {
    this.path = `${this.numenorPath}/${this.panel}`
  }

  onChange(callback: (event: ArdaPanelChangeEvent) => void) {
    this.listeners.add(callback)
    return () => {
      this.listeners.delete(callback)
    }
  }

  off(callback: (event: ArdaPanelChangeEvent) => void) {
    this.listeners.delete(callback)
  }

  async observe(settings: ArdaHudSettings) {
    const latestState = await getArdaRuntimeSurface(this.numenorPath, settings)
    const event = {
      panel: this.panel,
      type: 'projection_ready',
      path: this.path,
      bundle: JSON.stringify(latestState),
    } as const satisfies ArdaPanelChangeEvent

    this.emit(event)
  }

  emit(event: ArdaPanelChangeEvent) {
    for (const listener of this.listeners) {
      try {
        listener(event)
      } catch (error) {
        console.error(`Panel watcher [${this.panel}] listener failed`, error)
      }
    }
  }

  dispose() {
    this.listeners.clear()
  }
}

const watcherCache = new Map<ArdaPanel, ArdaPanelWatcher | null>()
let ardaRuntimeCache: JsonRecord | null = null
let ardaRuntimeCacheExpiresAt = 0

export function warmArdaPanelRuntime(numenorPath: string, settings: ArdaHudSettings, ttlMs = 2000) {
  const cacheKey = `${numenorPath}::${ttlMs}`
  if (ardaRuntimeCache && Date.now() < ardaRuntimeCacheExpiresAt && ardaRuntimeCache.key === numenorPath) {
    return ardaRuntimeCache.value
  }

  const later = () => {
    getArdaRuntimeSurface(numenorPath, settings).then((surface) => {
      ardaRuntimeCache = { key: numenorPath, value: surface }
      ardaRuntimeCacheExpiresAt = Date.now() + Math.max(ttlMs, 0)
    }).catch((error) => {
      console.error('Arda runtime cache refresh failed', error)
    })
  }

  if (typeof setTimeout !== 'undefined') {
    setTimeout(later, 0)
  }

  return ardaRuntimeCache?.value ?? null
}

export function resetArdaPanelRuntimeCache() {
  ardaRuntimeCache = null
  ardaRuntimeCacheExpiresAt = 0
}
