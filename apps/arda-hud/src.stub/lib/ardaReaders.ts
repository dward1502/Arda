// sigil: REPAIR
import { fetchInventoryTree, readFile, type FileReadResult, type InventoryTreeNode } from './weathertop'
import type { ArdaLedgerState, JsonRecord } from './ardaBundleTypes'
import { parseJsonOrNull } from './jsonParse'

function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as JsonRecord
}

export async function readJson(rootPath: string, relativePath: string): Promise<JsonRecord | null> {
  const result = await readFile(`${rootPath}/${relativePath}`)
  if (!result.success || !result.content) {
    return null
  }

  return asRecord(parseJsonOrNull<unknown>(result.content))
}

export async function readJsonLines(rootPath: string, relativePath: string): Promise<JsonRecord[]> {
  const result = await readFile(`${rootPath}/${relativePath}`)
  if (!result.success || !result.content) {
    return []
  }

  return result.content
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => asRecord(parseJsonOrNull<unknown>(line)))
    .filter((entry): entry is JsonRecord => entry !== null)
}

export async function ledgerState(rootPath: string, relativePath: string, label: string): Promise<ArdaLedgerState> {
  const result = await readFile(`${rootPath}/${relativePath}`)
  if (!result.success || !result.content) {
    return {
      path: relativePath,
      label,
      present: false,
      recordCount: 0,
      status: 'missing',
      detail: 'ledger file is missing or unreadable',
    }
  }

  const recordCount = relativePath.endsWith('.jsonl')
    ? result.content.split('\n').map((line) => line.trim()).filter(Boolean).length
    : 1
  return {
    path: relativePath,
    label,
    present: true,
    recordCount,
    status: recordCount > 0 ? 'ready' : 'empty',
    detail: recordCount > 0 ? `${recordCount} records available` : 'ledger exists but has no records',
  }
}

export async function readText(rootPath: string, relativePath: string): Promise<string> {
  const result = await readFile(`${rootPath}/${relativePath}`)
  return result.success && result.content ? result.content : ''
}

function truncatePreview(value: string, max = 220): string {
  const normalized = value.replace(/\s+/g, ' ').trim()
  if (!normalized) return ''
  return normalized.length > max ? `${normalized.slice(0, max - 1)}...` : normalized
}

export function filenameFromPath(path: string): string {
  return path.split('/').filter(Boolean).slice(-1)[0] ?? path
}

export function titleFromPath(path: string): string {
  const filename = filenameFromPath(path)
  const stem = filename.replace(/\.[^.]+$/, '')
  return stem
    .split(/[-_]/g)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ')
}

export async function summarizeReadable(rootPath: string, relativePath: string): Promise<JsonRecord> {
  const result = await readFile(`${rootPath}/${relativePath}`)
  if (!result.success || !result.content) {
    return {
      title: titleFromPath(relativePath),
      path: relativePath,
      body_preview: '',
    }
  }

  const lines = result.content.split('\n').map((line) => line.trim())
  const titleLine = lines.find((line) => line.startsWith('#'))?.replace(/^#+\s*/, '').trim()
  return {
    title: titleLine || titleFromPath(relativePath),
    path: relativePath,
    body_preview: truncatePreview(result.content),
  }
}

export async function readInventoryTree(rootPath: string, relativePath: string, maxDepth = 4): Promise<InventoryTreeNode | null> {
  try {
    const result: FileReadResult = await fetchInventoryTree(rootPath, relativePath, maxDepth)
    if (!result.success || !result.content) return null
    const parsed = parseJsonOrNull<unknown>(result.content)
    return parsed as InventoryTreeNode
  } catch {
    return null
  }
}

export function collectInventoryPaths(tree: InventoryTreeNode | null, extension: string, acc: string[] = []): string[] {
  if (!tree) return acc
  if (!tree.is_dir && tree.relative_path.endsWith(extension)) {
    acc.push(tree.relative_path)
  }
  for (const child of tree.children ?? []) {
    collectInventoryPaths(child, extension, acc)
  }
  return acc
}
