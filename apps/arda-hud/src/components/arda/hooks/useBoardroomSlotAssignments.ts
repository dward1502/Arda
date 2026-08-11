// sigil: REPAIR
import { useEffect, useMemo, useRef, useState } from 'react'
import {
  ARDA_BOARDROOM_SLOT_STORAGE_KEY,
  BOARDROOM_MONITOR_SLOT_IDS,
  BOARDROOM_SCENE_SLOT_IDS,
  DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS,
  assignmentsFromDocument,
  claimMonitorSlot,
  createDefaultBoardroomSlotSettings,
  documentFromAssignments,
  documentWithSurfaceLayout,
  documentWithVisualizationSelection,
  exportBoardroomProfile,
  importBoardroomProfile,
  loadBoardroomSlotSettings,
  readLocalBoardroomSlotSettingsDocument,
  refreshMonitorSlot,
  releaseMonitorSlot,
  resetBoardroomProfile,
  resetMonitorSlot,
  resolveMonitorSlotSource,
  type BoardroomAgentClaim,
  type BoardroomMonitorSlotSource,
  type BoardroomSceneSlotAssignments,
  type BoardroomSceneSlotId,
  type BoardroomSlotAssignmentMode,
  type BoardroomSlotSettingsDocument,
  type BoardroomSurfaceLayout,
  saveBoardroomSlotSettingsDocument,
  surfaceLayoutsFromDocument,
} from '../../../lib/boardroomSlotSettings'
import { parseJsonOrNull } from '../../../lib/jsonParse'
import type { BoardroomVisualizationSelection } from '../../../scene/boardroom/boardroomVisualizationPresets'

interface UseBoardroomSlotAssignmentsResult {
  assignments: BoardroomSceneSlotAssignments
  setAssignments: (updater: BoardroomSceneSlotAssignments | ((current: BoardroomSceneSlotAssignments) => BoardroomSceneSlotAssignments)) => void
  mode: BoardroomSlotAssignmentMode
  message: string
  saveStatus: 'idle' | 'saving' | 'saved' | 'error'
  document: BoardroomSlotSettingsDocument
  surfaceLayouts: Record<string, BoardroomSurfaceLayout>
  monitorSlotSources: Record<string, BoardroomMonitorSlotSource | null>
  claimMonitorSlot: (slotId: BoardroomSceneSlotId, claim: BoardroomAgentClaim) => void
  releaseMonitorSlot: (slotId: BoardroomSceneSlotId, owner: string) => void
  refreshMonitorSlot: (slotId: BoardroomSceneSlotId, owner: string, leaseExpiresAtUtc: string) => void
  resetMonitorSlot: (slotId: BoardroomSceneSlotId) => void
  updateSurfaceLayout: (slotId: BoardroomSceneSlotId, updater: BoardroomSurfaceLayout | ((current: BoardroomSurfaceLayout) => BoardroomSurfaceLayout)) => void
  updateVisualization: (slotId: BoardroomSceneSlotId, selection: BoardroomVisualizationSelection) => { ok: boolean; message: string }
  exportProfile: () => string
  importProfile: (serialized: string) => { ok: boolean; message: string }
  resetProfile: () => void
}

function localStorageOrNull(): Storage | null {
  return typeof window === 'undefined' ? null : window.localStorage
}

export function useBoardroomSlotAssignments(rootPath: string | null | undefined): UseBoardroomSlotAssignmentsResult {
  const initialDocument = useMemo(
    () => readLocalBoardroomSlotSettingsDocument(localStorageOrNull()) ?? createDefaultBoardroomSlotSettings(),
    [],
  )
  const initialAssignments = useMemo(() => assignmentsFromDocument(initialDocument), [initialDocument])
  const [assignments, setAssignmentsState] = useState<BoardroomSceneSlotAssignments>(initialAssignments)
  const [mode, setMode] = useState<BoardroomSlotAssignmentMode>('local')
  const [message, setMessage] = useState('Using browser-local boardroom slot assignments')
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle')
  const [document, setDocument] = useState<BoardroomSlotSettingsDocument>(initialDocument)
  const dirtyRef = useRef(false)

  useEffect(() => {
    const storage = localStorageOrNull()
    try {
      storage?.setItem(ARDA_BOARDROOM_SLOT_STORAGE_KEY, exportBoardroomProfile(document))
    } catch {
      // Browser local persistence is a fallback; failure should not block scene operation.
    }
  }, [document])

  useEffect(() => {
    if (!rootPath) return
    let cancelled = false
    loadBoardroomSlotSettings(rootPath).then((result) => {
      if (cancelled) return
      if (result.mode === 'workspace') {
        setAssignmentsState(result.assignments)
      }
      setDocument(result.document)
      setMode(result.mode)
      setMessage(result.message)
    }).catch((error: unknown) => {
      if (cancelled) return
      setMode('local')
      setMessage(error instanceof Error ? error.message : 'Using browser-local boardroom slot assignments')
    })
    return () => {
      cancelled = true
    }
  }, [rootPath])

  useEffect(() => {
    if (!rootPath || mode !== 'workspace' || !dirtyRef.current) return
    let cancelled = false
    setSaveStatus('saving')
    saveBoardroomSlotSettingsDocument(rootPath, document).then((result) => {
      if (cancelled) return
      if (result.success) {
        dirtyRef.current = false
        try {
          const parsed = parseJsonOrNull<BoardroomSlotSettingsDocument>(result.content)
          if (parsed) setDocument(parsed)
        } catch {
          // The saved assignment state remains authoritative for this session.
        }
        setSaveStatus('saved')
        setMessage(`Saved ${BOARDROOM_SCENE_SLOT_IDS.length} boardroom slots to workspace state`)
      } else {
        setSaveStatus('error')
        setMode('local')
        setMessage(result.error ?? 'Workspace save failed; using browser-local boardroom slot assignments')
      }
    }).catch((error: unknown) => {
      if (cancelled) return
      setSaveStatus('error')
      setMode('local')
      setMessage(error instanceof Error ? error.message : 'Workspace save failed; using browser-local boardroom slot assignments')
    })
    return () => {
      cancelled = true
    }
  }, [document, mode, rootPath])

  const markDirty = () => {
    dirtyRef.current = true
    if (rootPath && mode === 'fallback') setMode('workspace')
  }

  const setAssignments = (updater: BoardroomSceneSlotAssignments | ((current: BoardroomSceneSlotAssignments) => BoardroomSceneSlotAssignments)) => {
    markDirty()
    setAssignmentsState((current) => {
      const next = typeof updater === 'function' ? updater(current) : updater
      const normalized = BOARDROOM_SCENE_SLOT_IDS.reduce<BoardroomSceneSlotAssignments>((normalizedAssignments, slotId) => {
        normalizedAssignments[slotId] = next[slotId] ?? DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS[slotId]
        return normalizedAssignments
      }, { ...DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS })
      setDocument((currentDocument) => documentFromAssignments(normalized, new Date().toISOString(), currentDocument))
      return normalized
    })
  }

  const updateSurfaceLayout = (
    slotId: BoardroomSceneSlotId,
    updater: BoardroomSurfaceLayout | ((current: BoardroomSurfaceLayout) => BoardroomSurfaceLayout),
  ) => {
    markDirty()
    setDocument((currentDocument) => {
      const currentLayout = surfaceLayoutsFromDocument(currentDocument)[slotId]
      const nextLayout = typeof updater === 'function' ? updater(currentLayout) : updater
      return documentWithSurfaceLayout(currentDocument, slotId, nextLayout)
    })
  }

  const updateVisualization = (slotId: BoardroomSceneSlotId, selection: BoardroomVisualizationSelection) => {
    const result = documentWithVisualizationSelection(document, slotId, selection)
    if (result.ok) {
      markDirty()
      setDocument(result.document)
      setMessage(result.message)
    }
    return { ok: result.ok, message: result.message }
  }

  const claimSlot = (slotId: BoardroomSceneSlotId, claim: BoardroomAgentClaim) => {
    markDirty()
    setDocument((current) => claimMonitorSlot(current, slotId, claim))
  }

  const releaseSlot = (slotId: BoardroomSceneSlotId, owner: string) => {
    markDirty()
    setDocument((current) => releaseMonitorSlot(current, slotId, owner))
  }

  const refreshSlot = (slotId: BoardroomSceneSlotId, owner: string, leaseExpiresAtUtc: string) => {
    markDirty()
    setDocument((current) => refreshMonitorSlot(current, slotId, owner, leaseExpiresAtUtc))
  }

  const resetSlot = (slotId: BoardroomSceneSlotId) => {
    markDirty()
    setDocument((current) => resetMonitorSlot(current, slotId))
  }

  const monitorSlotSources: Record<string, BoardroomMonitorSlotSource | null> = useMemo(() => {
    return BOARDROOM_MONITOR_SLOT_IDS.reduce<Record<string, BoardroomMonitorSlotSource | null>>((sources, slotId) => {
      sources[slotId] = resolveMonitorSlotSource(slotId, document)
      return sources
    }, {})
  }, [document])

  const importProfile = (serialized: string) => {
    const result = importBoardroomProfile(serialized)
    if (!result.ok || !result.document) {
      setMessage(result.message)
      return { ok: false, message: result.message }
    }
    markDirty()
    setDocument(result.document)
    setAssignmentsState(assignmentsFromDocument(result.document))
    setMessage(result.message)
    return { ok: true, message: result.message }
  }

  const resetProfile = () => {
    const next = resetBoardroomProfile()
    markDirty()
    setDocument(next)
    setAssignmentsState(assignmentsFromDocument(next))
    setMessage('Reset boardroom profile to defaults')
  }

  return {
    assignments,
    setAssignments,
    mode,
    message,
    saveStatus,
    document,
    surfaceLayouts: surfaceLayoutsFromDocument(document),
    monitorSlotSources,
    claimMonitorSlot: claimSlot,
    releaseMonitorSlot: releaseSlot,
    refreshMonitorSlot: refreshSlot,
    resetMonitorSlot: resetSlot,
    updateSurfaceLayout,
    updateVisualization,
    exportProfile: () => exportBoardroomProfile(document),
    importProfile,
    resetProfile,
  }
}
