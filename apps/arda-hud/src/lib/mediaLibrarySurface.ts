// sigil: REPAIR
import type { InventoryTreeNode } from './weathertop'

export type MediaLibraryEntryKind = 'markdown' | 'pdf' | 'image' | 'video' | 'document' | 'data' | 'other'
export type MediaLibraryPreviewMode = 'text_preview' | 'image_preview' | 'video_preview' | 'pdf_preview' | 'native_focus' | 'adapter_pending'

export interface MediaLibraryRoot {
  id: string
  title: string
  relativePath: string
  purpose: string
}

export interface MediaLibraryEntry {
  id: string
  name: string
  relativePath: string
  kind: MediaLibraryEntryKind
  previewMode: MediaLibraryPreviewMode
  sizeBytes: number | null
  modifiedUnix: number | null
}

export interface MediaLibrarySummary {
  total: number
  markdown: number
  pdf: number
  image: number
  video: number
  document: number
  data: number
  other: number
}

export const MEDIA_LIBRARY_ROOTS: MediaLibraryRoot[] = [
  {
    id: 'human_docs',
    title: 'Human Docs',
    relativePath: 'human',
    purpose: 'Operator notes, plans, reference packets, and markdown docs.',
  },
  {
    id: 'project_docs',
    title: 'Project Docs',
    relativePath: 'docs',
    purpose: 'System architecture, policy, runbooks, and reference documents.',
  },
  {
    id: 'media_outputs',
    title: 'Media Outputs',
    relativePath: 'data',
    purpose: 'Generated packets, visual references, images, video clips, and data streams.',
  },
]

const TEXT_PREVIEW_EXTENSIONS = new Set(['.md', '.markdown', '.txt', '.json', '.jsonl', '.yaml', '.yml', '.toml', '.csv', '.log'])
const IMAGE_EXTENSIONS = new Set(['.png', '.jpg', '.jpeg', '.webp', '.gif', '.avif'])
const VIDEO_EXTENSIONS = new Set(['.mov', '.mp4', '.webm', '.mkv'])
const DOCUMENT_EXTENSIONS = new Set(['.doc', '.docx', '.odt', '.rtf'])

function extensionForPath(path: string): string {
  const match = path.toLowerCase().match(/\.[a-z0-9]+$/)
  return match?.[0] ?? ''
}

export function classifyMediaLibraryEntry(path: string): MediaLibraryEntryKind {
  const extension = extensionForPath(path)
  if (extension === '.md' || extension === '.markdown') return 'markdown'
  if (extension === '.pdf') return 'pdf'
  if (IMAGE_EXTENSIONS.has(extension)) return 'image'
  if (VIDEO_EXTENSIONS.has(extension)) return 'video'
  if (DOCUMENT_EXTENSIONS.has(extension)) return 'document'
  if (TEXT_PREVIEW_EXTENSIONS.has(extension)) return 'data'
  return 'other'
}

export function previewModeForMediaEntry(kind: MediaLibraryEntryKind): MediaLibraryPreviewMode {
  if (kind === 'markdown' || kind === 'data') return 'text_preview'
  if (kind === 'image') return 'image_preview'
  if (kind === 'video') return 'video_preview'
  if (kind === 'pdf') return 'pdf_preview'
  if (kind === 'document') return 'native_focus'
  return 'adapter_pending'
}

export function buildMediaLibraryEntries(nodes: InventoryTreeNode[], limit = 80): MediaLibraryEntry[] {
  const entries: MediaLibraryEntry[] = []

  function visit(node: InventoryTreeNode) {
    if (entries.length >= limit) return
    if (!node.is_dir) {
      const kind = classifyMediaLibraryEntry(node.relative_path)
      if (kind !== 'other') {
        entries.push({
          id: node.relative_path,
          name: node.name,
          relativePath: node.relative_path,
          kind,
          previewMode: previewModeForMediaEntry(kind),
          sizeBytes: node.size_bytes,
          modifiedUnix: node.modified_unix,
        })
      }
    }
    for (const child of node.children) visit(child)
  }

  nodes.forEach(visit)
  return entries.sort((left, right) => {
    const byKind = left.kind.localeCompare(right.kind)
    if (byKind !== 0) return byKind
    return left.relativePath.localeCompare(right.relativePath)
  })
}

export function summarizeMediaLibrary(entries: MediaLibraryEntry[]): MediaLibrarySummary {
  return entries.reduce<MediaLibrarySummary>((summary, entry) => ({
    ...summary,
    total: summary.total + 1,
    [entry.kind]: summary[entry.kind] + 1,
  }), {
    total: 0,
    markdown: 0,
    pdf: 0,
    image: 0,
    video: 0,
    document: 0,
    data: 0,
    other: 0,
  })
}
