// sigil: REPAIR
/**
 * Multi-Window Management for ARDA HUD
 * 
 * Enables opening secondary windows (e.g., Control Deck on secondary monitor)
 * with inter-window communication via CustomEvents.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { safeTauriInvoke } from '../lib/tauriGuard'
import { parseJsonOrNull } from '../lib/jsonParse'

interface BridgeAgent {
  id: string
  [key: string]: unknown
}

interface BridgeMission {
  id: string
  [key: string]: unknown
}

export interface WorkstationBridgeState {
  workstationId: string
  sourceZoneId?: string
  originAnchorId?: string
  presentationMode?: 'in_scene' | 'native_window'
  activeModuleId?: string
  layout?: {
    x?: number
    y?: number
    width?: number
    height?: number
    zIndex?: number
  }
  sourceWindowId?: string
  updatedAt?: string
  payload?: Record<string, unknown>
}

const WORKSTATION_STATE_STORAGE_KEY = 'arda.workstation.state.v1'

function localStorageOrNull(): Storage | null {
  try {
    return typeof window === 'undefined' ? null : window.localStorage
  } catch {
    return null
  }
}

function readStoredWorkstationStateMap(): Record<string, WorkstationBridgeState> {
  try {
    const raw = localStorageOrNull()?.getItem(WORKSTATION_STATE_STORAGE_KEY)
    if (!raw) return {}
    const parsed = parseJsonOrNull<unknown>(raw)
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {}
    return parsed as Record<string, WorkstationBridgeState>
  } catch {
    return {}
  }
}

function persistWorkstationState(state: WorkstationBridgeState): WorkstationBridgeState {
  const normalized: WorkstationBridgeState = {
    ...state,
    updatedAt: state.updatedAt ?? new Date().toISOString(),
  }
  const next = {
    ...readStoredWorkstationStateMap(),
    [normalized.workstationId]: normalized,
  }
  try {
    localStorageOrNull()?.setItem(WORKSTATION_STATE_STORAGE_KEY, JSON.stringify(next))
  } catch {
    // Cross-window persistence is best-effort; still emit the in-memory sync event.
  }
  return normalized
}

function emitWorkstationSync(state: WorkstationBridgeState): void {
  windowManager.emit('workstation-sync', state)
  window.dispatchEvent(new CustomEvent('workstation-sync', { detail: state }))
}

/**
 * Window configuration options
 */
export interface WindowConfig {
  /** Unique identifier for this window */
  id: string
  /** Display title */
  title: string
  /** High-level window role */
  windowRole?: 'workstation' | 'panel' | 'auxiliary'
  /** Workstation identity for scene-backed windows */
  workstationId?: string
  /** Source scene zone when this window mirrors a workstation */
  sourceZoneId?: string
  /** Origin anchor that launched the workstation */
  originAnchorId?: string
  /** Preferred presentation mode */
  presentationMode?: 'in_scene' | 'native_window'
  /** Width in pixels (default: 600) */
  width?: number
  /** Height in pixels (default: 400) */
  height?: number
  /** URL to open (default: current page) */
  url?: string
  /** Position: 'center', 'primary', 'secondary', or {x, y} */
  position?: 'center' | 'primary' | 'secondary' | { x: number; y: number }
  /** Optional workstation subtitle */
  subtitle?: string
}

/**
 * Default configurations for different window types
 */
export const WINDOW_PRESETS: Record<string, WindowConfig> = {
  controlDeck: {
    id: 'control-deck',
    title: 'ARDA Control Deck',
    windowRole: 'auxiliary',
    width: 500,
    height: 600,
    position: 'secondary',
  },
  missionBoard: {
    id: 'mission-board',
    title: 'Mission Board',
    windowRole: 'panel',
    width: 400,
    height: 500,
    position: 'primary',
  },
  agentMonitor: {
    id: 'agent-monitor',
    title: 'Agent Monitor',
    windowRole: 'auxiliary',
    width: 350,
    height: 450,
    position: 'center',
  }
}

/**
 * Store for managed windows
 */
class WindowManager {
  private windows: Map<string, Window> = new Map()
  private eventListeners: Map<string, Set<(data: unknown) => void>> = new Map()

  /**
   * Open a new window with given configuration
   * 
   * @param config - Window configuration
   * @returns The opened Window object or null if failed
   * 
   * @example
   * const win = windowManager.open({
   *   id: 'my-deck',
   *   title: 'Control Deck',
   *   position: 'secondary',
   *   features: true
   * })
   */
  open(config: WindowConfig): Window | null {
    if (config.windowRole === 'workstation') {
      this.openNativeWorkstation(config)
      return null
    }
    return this.openBrowserWindow(config)
  }

  private openNativeWorkstation(config: WindowConfig): void {
    safeTauriInvoke<string>('open_workstation_window', {
      window_label: config.id,
      title: config.title,
      subtitle: config.subtitle ?? null,
      workstation_id: config.workstationId ?? config.id,
      source_zone_id: config.sourceZoneId ?? null,
      origin_anchor_id: config.originAnchorId ?? null,
      presentation_mode: config.presentationMode ?? 'native_window',
      width: config.width ?? 600,
      height: config.height ?? 400,
    })
      .then(() => {
        this.emit('window-opened', {
          id: config.id,
          role: config.windowRole ?? 'panel',
          workstationId: config.workstationId ?? null,
          config,
        })
      })
      .catch(() => {
        this.openBrowserWindow(config)
      })
  }

  private openBrowserWindow(config: WindowConfig): Window | null {
    // Check if window with this ID already exists
    const existing = this.windows.get(config.id)
    if (existing && !existing.closed) {
      existing.focus()
      return existing
    }

    const width = config.width ?? 600
    const height = config.height ?? 400

    const position = this.calculatePosition(config.position ?? 'center', width, height)
    const baseUrl = config.url ?? window.location.href
    const url = new URL(baseUrl)
    url.searchParams.set('__windowId', config.id)
    if (config.windowRole) url.searchParams.set('__windowRole', config.windowRole)
    if (config.workstationId) url.searchParams.set('__workstation', config.workstationId)
    if (config.sourceZoneId) url.searchParams.set('__section', config.sourceZoneId)
    if (config.originAnchorId) url.searchParams.set('__anchor', config.originAnchorId)
    if (config.presentationMode) url.searchParams.set('__presentation', config.presentationMode)

    const win = window.open(
      url.toString(),
      config.id,
      `width=${width},height=${height},left=${position.x},top=${position.y},resizable=yes,scrollbars=yes`
    )

    if (win) {
      this.windows.set(config.id, win)

      const checkClosed = setInterval(() => {
        if (win.closed) {
          clearInterval(checkClosed)
          this.windows.delete(config.id)
          this.emit('window-closed', {
            id: config.id,
            role: config.windowRole ?? 'panel',
            workstationId: config.workstationId ?? null,
          })
        }
      }, 500)

      this.emit('window-opened', {
        id: config.id,
        role: config.windowRole ?? 'panel',
        workstationId: config.workstationId ?? null,
        config,
      })
    }

    return win
  }

  /**
   * Calculate window position based on strategy
   */
  private calculatePosition(
    position: 'center' | 'primary' | 'secondary' | { x: number; y: number },
    width: number,
    height: number
  ): { x: number; y: number } {
    const screens = this.getScreens()
    const primary = screens[0]
    const secondary = screens[1]
    const clampToPrimary = ({ x, y }: { x: number; y: number }): { x: number; y: number } => {
      const maxX = primary.x + Math.max(primary.width - width, 0)
      const maxY = primary.y + Math.max(primary.height - height, 0)
      return {
        x: Math.min(Math.max(Math.round(x), primary.x), maxX),
        y: Math.min(Math.max(Math.round(y), primary.y), maxY),
      }
    }

    if (typeof position === 'object') {
      return clampToPrimary(position)
    }

    switch (position) {
      case 'center':
        return clampToPrimary({
          x: primary.x + (primary.width - width) / 2,
          y: primary.y + (primary.height - height) / 2,
        })
      case 'primary':
        return clampToPrimary({
          x: primary.x + (primary.width - width) / 2,
          y: primary.y + (primary.height - height) / 2,
        })
      case 'secondary':
        if (secondary) {
          return {
            x: Math.round(secondary.x + (secondary.width - width) / 2),
            y: Math.round(secondary.y + (secondary.height - height) / 2),
          }
        }
        // Fallback to primary if no secondary
        return this.calculatePosition('primary', width, height)
      default:
        return clampToPrimary({ x: 100, y: 100 })
    }
  }

  /**
   * Get available screens (multi-monitor support)
   */
  private getScreens(): Array<{ x: number; y: number; width: number; height: number }> {
    const screens: Array<{ x: number; y: number; width: number; height: number }> = []
    
    // Use multi-screen API if available
    if (window.screen && 'availLeft' in window.screen) {
      const s = window.screen as Screen & { availLeft: number; availTop: number; availWidth: number; availHeight: number }
      screens.push({
        x: s.availLeft ?? 0,
        y: s.availTop ?? 0,
        width: s.availWidth,
        height: s.availHeight
      })
    } else {
      // Fallback
      screens.push({
        x: 0,
        y: 0,
        width: window.screen.width,
        height: window.screen.height
      })
    }

    return screens
  }

  /**
   * Close a window by ID
   */
  close(id: string): boolean {
    const win = this.windows.get(id)
    if (win && !win.closed) {
      win.close()
      this.windows.delete(id)
      return true
    }
    return false
  }

  /**
   * Focus a window by ID
   */
  focus(id: string): boolean {
    const win = this.windows.get(id)
    if (win && !win.closed) {
      win.focus()
      return true
    }
    return false
  }

  /**
   * Send data to a specific window
   */
  sendTo(id: string, channel: string, data: unknown): boolean {
    const win = this.windows.get(id)
    if (win && !win.closed) {
      win.postMessage({ channel, data }, '*')
      return true
    }
    return false
  }

  /**
   * Broadcast data to all open windows
   */
  broadcast(channel: string, data: unknown): void {
    for (const [, win] of this.windows) {
      if (!win.closed) {
        win.postMessage({ channel, data }, '*')
      }
    }
    // Also emit locally for same-window listeners
    this.emit(channel, data)
  }

  /**
   * Subscribe to messages from windows
   */
  on(channel: string, callback: (data: unknown) => void): () => void {
    if (!this.eventListeners.has(channel)) {
      this.eventListeners.set(channel, new Set())
    }
    this.eventListeners.get(channel)!.add(callback)
    
    // Return unsubscribe function
    return () => {
      this.eventListeners.get(channel)?.delete(callback)
    }
  }

  /**
   * Emit event locally (public for cross-window bridge)
   */
  emit(channel: string, data: unknown): void {
    this.eventListeners.get(channel)?.forEach(cb => cb(data))
  }

  /**
   * Get all open window IDs
   */
  getOpenWindows(): string[] {
    return Array.from(this.windows.entries())
      .filter(([, win]) => !win.closed)
      .map(([id]) => id)
  }

  /**
   * Check if a window is open
   */
  isOpen(id: string): boolean {
    const win = this.windows.get(id)
    return win !== undefined && !win.closed
  }
}

// Singleton instance
export const windowManager = new WindowManager()

// ============================================================================
// CROSS-WINDOW EVENT BRIDGE
// ============================================================================

/**
 * Initialize message listener for inter-window communication
 * Call this once in your app's initialization
 */
let bridgeInitialized = false
let tauriWorkstationUnlisten: Promise<UnlistenFn | null> | null = null

export function initWindowBridge(): void {
  if (bridgeInitialized) return
  bridgeInitialized = true

  window.addEventListener('message', (event) => {
    const { channel, data } = event.data ?? {}
    if (typeof channel === 'string' && channel.length > 0) {
      windowManager.emit(channel, data)
      if (channel.startsWith('workstation-')) {
        window.dispatchEvent(new CustomEvent(channel, { detail: data }))
      }
    }
  })

  window.addEventListener('storage', (event) => {
    if (event.key !== WORKSTATION_STATE_STORAGE_KEY || !event.newValue) return
    try {
      const parsed = parseJsonOrNull<Record<string, WorkstationBridgeState>>(event.newValue)
      if (!parsed) return
      Object.values(parsed).forEach((state) => {
        if (state?.workstationId) {
          emitWorkstationSync(state)
        }
      })
    } catch {
      // Ignore malformed external storage updates.
    }
  })

  tauriWorkstationUnlisten = listen<WorkstationBridgeState>('workstation-sync', (event) => {
    if (!event.payload?.workstationId) return
    const state = persistWorkstationState(event.payload)
    emitWorkstationSync(state)
  }).catch(() => null)
}

export function syncWorkstationState(state: WorkstationBridgeState): void {
  const next = persistWorkstationState(state)
  windowManager.broadcast('workstation-sync', next)
  emitWorkstationSync(next)
}

export function focusWorkstationWindow(workstationId: string): void {
  windowManager.broadcast('workstation-focus', { workstationId })
}

export function getStoredWorkstationState(workstationId: string): WorkstationBridgeState | null {
  return readStoredWorkstationStateMap()[workstationId] ?? null
}

/**
 * Send agent update to all windows
 * 
 * @param agent - Updated agent data
 * 
 * @example
 * // In main window
 * broadcastAgentUpdate(agent)
 * 
 * // In secondary window
 * windowManager.on('agent-update', (data) => { ... })
 */
export function broadcastAgentUpdate(agent: BridgeAgent): void {
  windowManager.broadcast('agent-update', agent)
}

/**
 * Send mission update to all windows
 * 
 * @param mission - Updated mission data
 */
export function broadcastMissionUpdate(mission: BridgeMission): void {
  windowManager.broadcast('mission-update', mission)
}

/**
 * Send command to all windows (for coordination)
 * 
 * @param command - Command name
 * @param payload - Optional payload
 */
export function broadcastCommand(command: string, payload?: unknown): void {
  windowManager.broadcast('command', { command, payload })
}

// ============================================================================
// REACT HOOK (for use in components)
// ============================================================================

/**
 * React hook for multi-window management
 * 
 * @example
 * const { openWindow, closeWindow, isOpen } = useMultiWindow()
 * 
 * <button onClick={() => openWindow('controlDeck')}>
 *   Open Control Deck
 * </button>
 */
export function useMultiWindow() {
  return {
    openWorkstation: (config: WindowConfig) => windowManager.open(config),
    closeWorkstation: (id: string) => windowManager.close(id),
    focusWorkstation: (id: string) => windowManager.focus(id),
    isWorkstationOpen: (id: string) => windowManager.isOpen(id),
    openWindow: (preset: string) => {
      const config = WINDOW_PRESETS[preset]
      if (config) {
        windowManager.open(config)
      }
    },
    closeWindow: (id: string) => windowManager.close(id),
    isOpen: (id: string) => windowManager.isOpen(id),
    getOpenWindows: () => windowManager.getOpenWindows()
  }
}

export default windowManager
