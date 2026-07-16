// sigil: REPAIR
import { ExternalLink, FileText, Image, PlaySquare, RefreshCw } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import ModuleCard from '../ModuleCard'
import LineList from '../primitives/LineList'
import {
  MEDIA_LIBRARY_ROOTS,
  buildMediaLibraryEntries,
  summarizeMediaLibrary,
  type MediaLibraryEntry,
} from '../../../lib/mediaLibrarySurface'
import {
  fetchInventoryTree,
  openSourcePath,
  readFile,
  readSourceImagePreview,
  readSourcePdfPreview,
  readSourceVideoPreview,
  type InventoryTreeNode,
} from '../../../lib/weathertop'
import { parseJsonOrNull } from '../../../lib/jsonParse'

interface MediaLibraryModuleProps {
  rootPath: string | null
}

function iconForKind(kind: MediaLibraryEntry['kind']) {
  if (kind === 'image') return <Image size={14} />
  if (kind === 'video') return <PlaySquare size={14} />
  return <FileText size={14} />
}

function formatBytes(sizeBytes: number | null): string {
  if (sizeBytes === null) return 'unknown'
  if (sizeBytes < 1024) return `${sizeBytes} B`
  if (sizeBytes < 1024 * 1024) return `${Math.round(sizeBytes / 1024)} KB`
  return `${Math.round(sizeBytes / 1024 / 1024)} MB`
}

function previewText(content: string): string {
  return content.split('\n').slice(0, 18).join('\n').slice(0, 3200)
}

export default function MediaLibraryModule({ rootPath }: MediaLibraryModuleProps) {
  const [entries, setEntries] = useState<MediaLibraryEntry[]>([])
  const [selectedEntryId, setSelectedEntryId] = useState<string | null>(null)
  const [preview, setPreview] = useState<string | null>(null)
  const [imagePreviewUrl, setImagePreviewUrl] = useState<string | null>(null)
  const [videoPreviewUrl, setVideoPreviewUrl] = useState<string | null>(null)
  const [pdfPreviewUrl, setPdfPreviewUrl] = useState<string | null>(null)
  const [status, setStatus] = useState('Waiting for ARDA root.')
  const [actionMessage, setActionMessage] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  const selectedEntry = entries.find((entry) => entry.id === selectedEntryId) ?? entries[0] ?? null
  const summary = useMemo(() => summarizeMediaLibrary(entries), [entries])

  async function loadEntries() {
    if (!rootPath) {
      setStatus('ARDA root is unavailable; native scoped inventory cannot run.')
      setEntries([])
      return
    }
    setLoading(true)
    setStatus('Reading scoped media/document roots...')
    try {
      const trees: InventoryTreeNode[] = []
      for (const root of MEDIA_LIBRARY_ROOTS) {
        const result = await fetchInventoryTree(rootPath, root.relativePath, 4)
        if (result.success && result.content) {
          const tree = parseJsonOrNull<InventoryTreeNode>(result.content)
          if (tree) trees.push(tree)
        }
      }
      const nextEntries = buildMediaLibraryEntries(trees)
      setEntries(nextEntries)
      setSelectedEntryId((current) => current ?? nextEntries[0]?.id ?? null)
      setStatus(nextEntries.length ? `Indexed ${nextEntries.length} media/document entries.` : 'No supported media/document entries found.')
    } catch (error) {
      setStatus(error instanceof Error ? error.message : 'Failed to read media library inventory.')
      setEntries([])
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadEntries()
  }, [rootPath])

  useEffect(() => {
    async function loadPreview() {
      if (!rootPath || !selectedEntry) {
        setPreview(null)
        setImagePreviewUrl(null)
        setVideoPreviewUrl(null)
        setPdfPreviewUrl(null)
        return
      }
      setPreview(null)
      setImagePreviewUrl(null)
      setVideoPreviewUrl(null)
      setPdfPreviewUrl(null)
      if (selectedEntry.previewMode === 'image_preview') {
        try {
          const result = await readSourceImagePreview(selectedEntry.relativePath)
          setImagePreviewUrl(result.dataUrl)
        } catch (error) {
          setPreview(error instanceof Error ? error.message : 'Image preview unavailable.')
        }
        return
      }
      if (selectedEntry.previewMode === 'video_preview') {
        try {
          const result = await readSourceVideoPreview(selectedEntry.relativePath)
          setVideoPreviewUrl(result.dataUrl)
        } catch (error) {
          setPreview(error instanceof Error ? error.message : 'Video preview unavailable.')
        }
        return
      }
      if (selectedEntry.previewMode === 'pdf_preview') {
        try {
          const result = await readSourcePdfPreview(selectedEntry.relativePath)
          setPdfPreviewUrl(result.dataUrl)
        } catch (error) {
          setPreview(error instanceof Error ? error.message : 'PDF preview unavailable.')
        }
        return
      }
      if (selectedEntry.previewMode !== 'text_preview') {
        return
      }
      try {
        const result = await readFile(`${rootPath}/${selectedEntry.relativePath}`)
        setPreview(result.success && result.content ? previewText(result.content) : result.error ?? 'Preview unavailable.')
      } catch (error) {
        setPreview(error instanceof Error ? error.message : 'Preview unavailable.')
      }
    }
    void loadPreview()
  }, [rootPath, selectedEntry])

  async function openSelectedEntry() {
    if (!selectedEntry) {
      setActionMessage('No media/document entry selected.')
      return
    }
    try {
      const result = await openSourcePath(selectedEntry.relativePath)
      setActionMessage(result.message)
    } catch (error) {
      setActionMessage(`Native open unavailable: ${String(error)}`)
    }
  }

  return (
    <ModuleCard
      title="ARDA Media Library"
      eyebrow="Focused Adapter"
      accent="gold"
      tag={selectedEntry?.previewMode.replace(/_/g, ' ') ?? 'read only'}
      actions={(
        <>
          <button className="module-card__action" type="button" onClick={() => void openSelectedEntry()} disabled={!selectedEntry}>
            <ExternalLink size={14} />
            Open Native
          </button>
          <button className="module-card__action" type="button" onClick={() => void loadEntries()} disabled={loading}>
            <RefreshCw size={14} />
            Refresh
          </button>
        </>
      )}
    >
      <div className="media-library-layout">
        <div className="media-library-layout__summary">
          <LineList
            items={[
              { label: 'Status', value: status },
              { label: 'Total', value: `${summary.total}` },
              { label: 'Markdown/Data', value: `${summary.markdown + summary.data}` },
              { label: 'PDF/Image/Video', value: `${summary.pdf + summary.image + summary.video}` },
              { label: 'Documents', value: `${summary.document}` },
            ]}
          />
          <div className="service-surface-capabilities">
            {MEDIA_LIBRARY_ROOTS.map((root) => (
              <span key={root.id}>{root.title}: {root.relativePath}</span>
            ))}
          </div>
        </div>
        <div className="media-library-layout__entries" role="list">
          {entries.slice(0, 36).map((entry) => (
            <button
              className={`media-library-entry ${entry.id === selectedEntry?.id ? 'is-active' : ''}`}
              type="button"
              key={entry.id}
              onClick={() => setSelectedEntryId(entry.id)}
            >
              {iconForKind(entry.kind)}
              <span>
                <strong>{entry.name}</strong>
                <small>{entry.kind} / {formatBytes(entry.sizeBytes)}</small>
              </span>
            </button>
          ))}
        </div>
        <div className="media-library-preview">
          {selectedEntry ? (
            <>
              <div className="module-subtitle">{selectedEntry.relativePath}</div>
              {selectedEntry.previewMode === 'text_preview' ? (
                <pre>{preview ?? 'Loading preview...'}</pre>
              ) : selectedEntry.previewMode === 'image_preview' ? (
                imagePreviewUrl ? (
                  <img className="media-library-preview__image" src={imagePreviewUrl} alt={selectedEntry.name} />
                ) : (
                  <div className="service-embed-placeholder">
                    <strong>Image preview unavailable</strong>
                    <span>{preview ?? 'Loading scoped image preview...'}</span>
                  </div>
                )
              ) : selectedEntry.previewMode === 'video_preview' ? (
                videoPreviewUrl ? (
                  <video className="media-library-preview__video" src={videoPreviewUrl} controls preload="metadata" />
                ) : (
                  <div className="service-embed-placeholder">
                    <strong>Video preview unavailable</strong>
                    <span>{preview ?? 'Loading scoped video preview...'}</span>
                  </div>
                )
              ) : selectedEntry.previewMode === 'pdf_preview' ? (
                pdfPreviewUrl ? (
                  <iframe className="media-library-preview__pdf" src={pdfPreviewUrl} title={selectedEntry.name} />
                ) : (
                  <div className="service-embed-placeholder">
                    <strong>PDF preview unavailable</strong>
                    <span>{preview ?? 'Loading scoped PDF preview...'}</span>
                  </div>
                )
              ) : (
                <div className="service-embed-placeholder">
                  <strong>{selectedEntry.kind} native focused view</strong>
                  <span>This item is indexed and can be opened through the scoped native path action. Inline rendering remains blocked until renderer/codec policy is implemented.</span>
                </div>
              )}
              {actionMessage ? <p className="planning-action-contract__message">{actionMessage}</p> : null}
            </>
          ) : (
            <div className="empty-state">No media or document entry selected.</div>
          )}
        </div>
      </div>
    </ModuleCard>
  )
}
