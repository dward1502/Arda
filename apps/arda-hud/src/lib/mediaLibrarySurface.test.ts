// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import type { InventoryTreeNode } from './weathertop'
import {
  buildMediaLibraryEntries,
  classifyMediaLibraryEntry,
  previewModeForMediaEntry,
  summarizeMediaLibrary,
} from './mediaLibrarySurface'

function file(relativePath: string, sizeBytes = 100): InventoryTreeNode {
  return {
    id: relativePath,
    name: relativePath.split('/').pop() ?? relativePath,
    path: `/workspace/${relativePath}`,
    relative_path: relativePath,
    is_dir: false,
    size_bytes: sizeBytes,
    modified_unix: 123,
    children: [],
  }
}

function dir(relativePath: string, children: InventoryTreeNode[]): InventoryTreeNode {
  return {
    id: relativePath,
    name: relativePath.split('/').pop() ?? relativePath,
    path: `/workspace/${relativePath}`,
    relative_path: relativePath,
    is_dir: true,
    size_bytes: null,
    modified_unix: 123,
    children,
  }
}

describe('mediaLibrarySurface', () => {
  it('classifies supported document and media types', () => {
    expect(classifyMediaLibraryEntry('human/plans/route.md')).toBe('markdown')
    expect(classifyMediaLibraryEntry('docs/spec.pdf')).toBe('pdf')
    expect(classifyMediaLibraryEntry('data/media/reference.png')).toBe('image')
    expect(classifyMediaLibraryEntry('data/media/demo.mov')).toBe('video')
    expect(classifyMediaLibraryEntry('human/report.docx')).toBe('document')
    expect(classifyMediaLibraryEntry('data/events.jsonl')).toBe('data')
    expect(classifyMediaLibraryEntry('target/cache.bin')).toBe('other')
  })

  it('keeps text-like docs and bounded media previewable', () => {
    expect(previewModeForMediaEntry('markdown')).toBe('text_preview')
    expect(previewModeForMediaEntry('data')).toBe('text_preview')
    expect(previewModeForMediaEntry('image')).toBe('image_preview')
    expect(previewModeForMediaEntry('video')).toBe('video_preview')
    expect(previewModeForMediaEntry('pdf')).toBe('pdf_preview')
    expect(previewModeForMediaEntry('document')).toBe('native_focus')
    expect(previewModeForMediaEntry('other')).toBe('adapter_pending')
  })

  it('flattens inventory trees into supported media entries', () => {
    const entries = buildMediaLibraryEntries([
      dir('human', [
        file('human/plans/route.md'),
        file('human/archive/raw.bin'),
      ]),
      dir('data', [
        file('data/media/reference.png'),
        file('data/events.jsonl'),
      ]),
    ])
    const summary = summarizeMediaLibrary(entries)

    expect(entries.map((entry) => entry.relativePath)).toEqual([
      'data/events.jsonl',
      'data/media/reference.png',
      'human/plans/route.md',
    ])
    expect(summary).toMatchObject({
      total: 3,
      markdown: 1,
      image: 1,
      data: 1,
    })
  })
})
