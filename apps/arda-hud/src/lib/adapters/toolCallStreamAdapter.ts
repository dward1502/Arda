export const ADAPTER_DOC_PATH = 'core/state/tool_call_stream.jsonl'

export interface ToolCallRecord {
  timestamp: number
  tool: string
  params?: Record<string, unknown>
  output?: unknown
  duration_ms?: number
  joulework?: number
  success?: boolean
  error?: string | null
}

export type ToolCallStreamStatus = 'idle' | 'polling' | 'error'

export interface ToolCallStreamAdapterState {
  status: ToolCallStreamStatus
  lastRecord: ToolCallRecord | null
  records: ToolCallRecord[]
  error: string | null
}

const DEFAULT_STATE: ToolCallStreamAdapterState = {
  status: 'idle',
  lastRecord: null,
  records: [],
  error: null,
}

const MAX_RECORDS = 200

function parseToolName(value: unknown): string | undefined {
  return typeof value === 'string' && value.trim().length > 0 ? value.trim() : undefined
}

function isValidToolCallRecord(value: unknown): value is ToolCallRecord {
  if (!value || typeof value !== 'object') return false
  const record = value as Record<string, unknown>
  if (typeof record.timestamp !== 'number') return false
  if (!parseToolName(record.tool)) return false
  if ('duration_ms' in record && typeof record.duration_ms !== 'number') return false
  if ('joulework' in record && typeof record.joulework !== 'number') return false
  if ('success' in record && typeof record.success !== 'boolean') return false
  if ('error' in record && record.error !== null && typeof record.error !== 'string') return false
  return true
}

function toEnvironmentTag(tool: string): string {
  const normalized = tool.toLowerCase()
  if (normalized.includes('rust') || normalized.includes('lib.rs') || normalized.includes('setup_console')) return 'rust'
  if (normalized.includes('browser') || normalized.includes('cua')) return 'browser'
  if (normalized.includes('web_search') || normalized.includes('web_extract') || normalized.includes('x_')) return 'web'
  if (normalized.includes('terminal') || normalized.includes('execute_code')) return 'agent'
  if (normalized.includes('render')) return 'scene'
  return 'tool'
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
}

function truncateJson(value: unknown, limit = 220): string {
  const source = typeof value === 'string' ? value : JSON.stringify(value)
  if (source.length <= limit) return escapeHtml(source)
  return `${escapeHtml(source.slice(0, limit))}\u2026`
}

export const defaultToolCallStreamAdapterState = (): ToolCallStreamAdapterState => ({
  ...DEFAULT_STATE,
})

export function createToolCallStreamHtml(record: ToolCallRecord): string {
  const tool = escapeHtml(record.tool)
  const when = new Date(record.timestamp).toLocaleString()
  const statusClass = record.success !== false ? 'tool-call--success' : 'tool-call--failed'
  const statusIcon = record.success !== false ? 'OK' : 'ERR'
  const env = toEnvironmentTag(record.tool)
  const params = record.params !== undefined ? `<pre class="tool-call-params">${truncateJson(record.params)}</pre>` : ''
  const output = record.output !== undefined ? `<pre class="tool-call-output">${truncateJson(record.output)}</pre>` : ''
  const meta = [
    record.duration_ms !== undefined ? `${record.duration_ms} ms` : null,
    record.joulework !== undefined ? `${record.joulework} JW` : null,
    env ? `env:${env}` : null,
  ]
    .filter((item): item is string => item !== null)
    .map((item) => `<span class="tool-call-meta">${escapeHtml(item)}</span>`)
    .join('')

  const error = record.error ? `<div class="tool-call-params tool-call-error">${escapeHtml(record.error)}</div>` : ''

  return `<div class="tool-call ${statusClass}">
<div class="tool-call-header">
  <span class="tool-call-title">${tool}</span>
  <span class="tool-call-when">${when}</span>
  <span class="tool-call-status">${statusIcon}</span>
  ${meta}
</div>
${params}
${output}
${error}
</div>`
}

export function parseToolCallTail(content: string, maxLength = 6000): string {
  if (!content || content.length <= maxLength) return content
  return content.slice(content.length - maxLength)
}

export function consumeToolCallStreamTail(content: string, state: ToolCallStreamAdapterState): ToolCallStreamAdapterState {
  if (!content || content.trim().length === 0) {
    return { ...state, status: 'idle', error: null }
  }

  const records: ToolCallRecord[] = []
  let parseError: string | null = null
  let hasAnyLine = false

  try {
    const tail = parseToolCallTail(content)
    const lines = tail.split(/\r?\n/)

    for (const rawLine of lines) {
      const trimmed = rawLine.trim()
      if (trimmed.length === 0) continue
      hasAnyLine = true

      let parsed: unknown
      try {
        parsed = JSON.parse(trimmed)
      } catch {
        parseError = parseError || 'Invalid JSONL line in tool_call_stream.jsonl'
        continue
      }

      if (isValidToolCallRecord(parsed)) {
        records.push(parsed)
      }
    }
  } catch (error) {
    parseError = error instanceof Error ? error.message : String(error)
  }

  if (parseError && records.length === 0) {
    return {
      ...state,
      status: 'error',
      records: [],
      lastRecord: null,
    }
  }

  const latest = records.length > 0 ? records[records.length - 1] : null
  const trimmedRecords = records.length > MAX_RECORDS ? records.slice(records.length - MAX_RECORDS) : records
  const status: ToolCallStreamStatus =
    latest ? 'polling' : state.status === 'error' ? 'error' : 'idle'
  const error = parseError ? state.error || parseError : state.error

  return {
    status,
    lastRecord: latest,
    records: trimmedRecords,
    error,
  }
}
