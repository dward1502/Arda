// sigil: REPAIR
import { useEffect, useMemo, useRef, useState } from 'react'
import {
  BOARDROOM_SCENE_SLOT_IDS,
  DEFAULT_BOARDROOM_SCENE_SLOT_ASSIGNMENTS,
  createDefaultBoardroomSlotSettings,
  documentFromAssignments,
  documentWithSurfaceLayout,
  type BoardroomSceneSlotAssignments,
  type BoardroomSceneSlotId,
  type BoardroomSlotAssignmentMode,
  type BoardroomSlotSettingsDocument,
  loadBoardroomSlotSettings,
  readLocalBoardroomSlotAssignments,
  saveBoardroomSlotSettingsDocument,
  surfaceLayoutsFromDocument,
  type BoardroomSurfaceLayout,
} from '../../../lib/boardroomSlotSettings'
import { parseJsonOrNull } from '../../../lib/jsonParse'

interface UseBoardroomSlotAssignmentsResult {
  assignments: BoardroomSceneSlotAssignments
  setAssignments: (updater: BoardroomSceneSlotAssignments | ((current: BoardroomSceneSlotAssignments) => BoardroomSceneSlotAssignments)) => void
  mode: BoardroomSlotAssignmentMode
  message: string
  saveStatus: 'idle' | 'saving' | 'saved' | 'error'
  document: BoardroomSlotSettingsDocument
  surfaceLayouts: Record<string, BoardroomSurfaceLayout>
  updateSurfaceLayout: (slotId: BoardroomSceneSlotId, updater: BoardroomSurfaceLayout | ((current: BoardroomSurfaceLayout) => BoardroomSurfaceLayout)) => void
}

function localStorageOrNull(): Storage | null {
  return typeof window === 'undefined' ? null : window.localStorage
}

export function useBoardroomSlotAssignments(rootPath: string | null | undefined): UseBoardroomSlotAssignmentsResult {
  const initialAssignments = useMemo(() => readLocalBoardroomSlotAssignments(localStorageOrNull()), [])
  const [assignments, setAssignmentsState] = useState<BoardroomSceneSlotAssignments>(initialAssignments)
  const [mode, setMode] = useState<BoardroomSlotAssignmentMode>('local')
  const [message, setMessage] = useState('Using browser-local boardroom slot assignments')
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle')
  const [document, setDocument] = useState<BoardroomSlotSettingsDocument>(() => createDefaultBoardroomSlotSettings())
  const dirtyRef = useRef(false)

  useEffect(() => {
    const storage = localStorageOrNull()
    try {
      storage?.setItem('arda.boardroom.scene_slots.v1', JSON.stringify(assignments))
    } catch {
      // Browser local persistence is a fallback; failure should not block scene operation.
    }
  }, [assignments])

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

  const setAssignments = (updater: BoardroomSceneSlotAssignments | ((current: BoardroomSceneSlotAssignments) => BoardroomSceneSlotAssignments)) => {
    dirtyRef.current = true
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
    dirtyRef.current = true
    setDocument((currentDocument) => {
      const currentLayout = surfaceLayoutsFromDocument(currentDocument)[slotId]
      const nextLayout = typeof updater === 'function' ? updater(currentLayout) : updater
      return documentWithSurfaceLayout(currentDocument, slotId, nextLayout)
    })
  }

  return {
    assignments,
    setAssignments,
    mode,
    message,
    saveStatus,
    document,
    surfaceLayouts: surfaceLayoutsFromDocument(document),
    updateSurfaceLayout,
  }
}
