// sigil: REPAIR
import { useEffect, useMemo, useRef, useState } from 'react'
import {
  ARDA_WORLD_SURFACE_STORAGE_KEY,
  createDefaultWorldSurfaceSettings,
  loadWorldSurfaceSettings,
  readLocalWorldSurfaceAssignments,
  saveWorldSurfaceSettingsDocument,
  WORLD_SCENE_SURFACE_IDS,
  worldSurfaceDocumentWithLayout,
  worldSurfaceLayoutsFromDocument,
  type WorldSceneSurfaceAssignments,
  type WorldSceneSurfaceId,
  type WorldSurfaceAssignmentMode,
  type WorldSurfaceLayout,
  type WorldSurfaceSettingsDocument,
} from '../../../lib/worldSurfaceSettings'

interface UseWorldSurfaceAssignmentsResult {
  assignments: WorldSceneSurfaceAssignments
  mode: WorldSurfaceAssignmentMode
  message: string
  saveStatus: 'idle' | 'saving' | 'saved' | 'error'
  document: WorldSurfaceSettingsDocument
  surfaceLayouts: Record<WorldSceneSurfaceId, WorldSurfaceLayout>
  updateSurfaceLayout: (surfaceId: WorldSceneSurfaceId, updater: WorldSurfaceLayout | ((current: WorldSurfaceLayout) => WorldSurfaceLayout)) => void
}

function localStorageOrNull(): Storage | null {
  return typeof window === 'undefined' ? null : window.localStorage
}

export function useWorldSurfaceAssignments(rootPath: string | null | undefined): UseWorldSurfaceAssignmentsResult {
  const initialAssignments = useMemo(() => readLocalWorldSurfaceAssignments(localStorageOrNull()), [])
  const [assignments, setAssignments] = useState<WorldSceneSurfaceAssignments>(initialAssignments)
  const [mode, setMode] = useState<WorldSurfaceAssignmentMode>('local')
  const [message, setMessage] = useState('Using browser-local world surface assignments')
  const [saveStatus, setSaveStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle')
  const [document, setDocument] = useState<WorldSurfaceSettingsDocument>(() => createDefaultWorldSurfaceSettings())
  const dirtyRef = useRef(false)

  useEffect(() => {
    try {
      localStorageOrNull()?.setItem(ARDA_WORLD_SURFACE_STORAGE_KEY, JSON.stringify(assignments))
    } catch {
      // Browser-local persistence is a fallback; failure should not block the world scene.
    }
  }, [assignments])

  useEffect(() => {
    if (!rootPath) return
    let cancelled = false
    loadWorldSurfaceSettings(rootPath).then((result) => {
      if (cancelled) return
      if (result.mode === 'workspace') {
        setAssignments(result.assignments)
      }
      setDocument(result.document)
      setMode(result.mode)
      setMessage(result.message)
    }).catch((error: unknown) => {
      if (cancelled) return
      setMode('local')
      setMessage(error instanceof Error ? error.message : 'Using browser-local world surface assignments')
    })
    return () => {
      cancelled = true
    }
  }, [rootPath])

  useEffect(() => {
    if (!rootPath || mode !== 'workspace' || !dirtyRef.current) return
    let cancelled = false
    setSaveStatus('saving')
    saveWorldSurfaceSettingsDocument(rootPath, document).then((result) => {
      if (cancelled) return
      if (result.success) {
        dirtyRef.current = false
        setSaveStatus('saved')
        setMessage(`Saved ${WORLD_SCENE_SURFACE_IDS.length} world surfaces to workspace state`)
      } else {
        setSaveStatus('error')
        setMode('local')
        setMessage(result.error ?? 'Workspace save failed; using browser-local world surface assignments')
      }
    }).catch((error: unknown) => {
      if (cancelled) return
      setSaveStatus('error')
      setMode('local')
      setMessage(error instanceof Error ? error.message : 'Workspace save failed; using browser-local world surface assignments')
    })
    return () => {
      cancelled = true
    }
  }, [document, mode, rootPath])

  const updateSurfaceLayout = (
    surfaceId: WorldSceneSurfaceId,
    updater: WorldSurfaceLayout | ((current: WorldSurfaceLayout) => WorldSurfaceLayout),
  ) => {
    dirtyRef.current = true
    setDocument((currentDocument) => {
      const currentLayout = worldSurfaceLayoutsFromDocument(currentDocument)[surfaceId]
      const nextLayout = typeof updater === 'function' ? updater(currentLayout) : updater
      return worldSurfaceDocumentWithLayout(currentDocument, surfaceId, nextLayout)
    })
  }

  return {
    assignments,
    mode,
    message,
    saveStatus,
    document,
    surfaceLayouts: worldSurfaceLayoutsFromDocument(document),
    updateSurfaceLayout,
  }
}
