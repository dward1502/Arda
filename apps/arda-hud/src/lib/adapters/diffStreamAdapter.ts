export const ADAPTER_DOC_PATH = 'core/state/agent_edit_stream.jsonl'

export interface DiffStreamRecord {
  timestamp: number
  file: string
  before_hash?: string
  after_hash?: string
  diff: string
  success?: boolean
  error?: string | null
}

export type DiffStreamAdapterStatus = 'idle' | 'polling' | 'error'

export interface DiffStreamAdapterState {
  status: DiffStreamAdapterStatus
  lastRecord: DiffStreamRecord | null
  records: DiffStreamRecord[]
  error: string | null
}

export const defaultDiffStreamAdapterState = (): DiffStreamAdapterState => ({
  status: 'idle',
  lastRecord: null,
  records: [],
  error: null,
})

const MAX_RECORDS = 200

function isValidDiffStreamRecord(value: unknown): value is DiffStreamRecord {
  if (!value || typeof value !== 'object') return false
  const record = value as Record<string, unknown>
  if (typeof record.timestamp !== 'number') return false
  if (typeof record.file !== 'string' || record.file.length === 0) return false
  if (typeof record.diff !== 'string') return false
  if ('success' in record && typeof record.success !== 'boolean') return false
  if ('error' in record && record.error !== null && typeof record.error !== 'string') return false
  return true
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
}

function renderDiffHunks(diff: string): string {
  const lines = diff.split(/\r?\n/)
  const html: string[] = []
  for (const line of lines) {
    const escaped = escapeHtml(line)
    if (line.startsWith('+') && !line.startsWith('+++')) {
      html.push(`<span class="diff-line-added">${escapeHtml(line)}</span>`)
      continue
    }
    if (line.startsWith('-') && !line.startsWith('---')) {
      html.push(`<span class="diff-line-removed">${escapeHtml(line)}</span>`)
      continue
    }
    html.push(`<span class="diff-line-context">${escaped}</span>`)
  }
  return html.join('')
}

export function createDiffStreamHtml(diff: string, record: DiffStreamRecord): string {
  const successMarker = record.success !== false ? '<span class="diff-success">✓</span>' : '<span class="diff-failed">✗</span>'
  const errorLine = record.error ? `<div class="diff-error">${escapeHtml(record.error)}</div>` : ''
  const fileName = escapeHtml(record.file)
  return `<div class="diff-entry">
<div class="diff-header">
  <span class="diff-title">${fileName}</span>
  <span class="diff-meta">${new Date(record.timestamp).toLocaleString()}</span>
  ${successMarker}
</div>
<div class="diff-body"><pre>${renderDiffHunks(diff)}</pre></div>
${errorLine}
</div>`
}

export function parseFixedLengthTail(content: string, maxLength = 5000): string {
  if (!content || content.length <= maxLength) return content
  return content.slice(content.length - maxLength)
}

export function consumeDiffStreamTail(content: string, state: DiffStreamAdapterState): DiffStreamAdapterState {
  if (!content || content.trim().length === 0) {
    return { ...state, status: 'idle', error: null }
  }

  let records: DiffStreamRecord[] = []
  let parseError: string | null = null
  let hasAnyLine = false

  try {
    const tail = parseFixedLengthTail(content)
    const lines = tail.split(/\r?\n/)

    for (const rawLine of lines) {
      const trimmed = rawLine.trim()
      if (!trimmed) continue
      hasAnyLine = true

      let parsed: unknown
      try {
        parsed = JSON.parse(trimmed)
      } catch {
        parseError = parseError || 'Invalid JSONL line in agent_edit_stream.jsonl'
        continue
      }

      if (isValidDiffStreamRecord(parsed)) {
        records.push(parsed)
      }
    }
  } catch (error) {
    parseError = error instanceof Error ? error.message : String(error)
  }

  if (parseError && records.length === 0) {
    return {
      ...defaultDiffStreamAdapterState(),
      status: 'error',
      error: parseError,
    }
  }

  const latest = records.length > 0 ? records[records.length - 1] : null
  const trimmedRecords = records.length > MAX_RECORDS
    ? records.slice(records.length - MAX_RECORDS)
    : records

  return {
    status: latest ? 'polling' : state.status === 'error' ? 'error' : 'idle',
    lastRecord: latest,
    records: trimmedRecords,
    error: parseError,
  }
}
