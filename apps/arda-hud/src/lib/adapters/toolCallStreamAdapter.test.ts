import { describe, expect, it } from 'vitest'
import {
  ADAPTER_DOC_PATH,
  consumeToolCallStreamTail,
  createToolCallStreamHtml,
  defaultToolCallStreamAdapterState,
  parseToolCallTail,
} from './toolCallStreamAdapter'

const LEDGER = [
  {
    timestamp: 1710000000000,
    tool: 'terminal',
    params: { command: 'npm test' },
    output: 'PASS',
    duration_ms: 1234,
    joulework: 4,
    success: true,
  },
  {
    timestamp: 1710000001000,
    tool: 'web_search',
    params: { query: 'креж' },
    output: null,
    success: false,
    error: 'provider unavailable',
  },
  {
    timestamp: 1710000002000,
    tool: 'patch',
    params: { path: '/missing/file' },
    success: true,
  },
] as const

function singleLine(record: Record<string, unknown>): string {
  return `${JSON.stringify(record)}\n`
}

describe('tool-call stream adapter', () => {
  it('exposes the core state stream path constant', () => {
    expect(ADAPTER_DOC_PATH).toBe('core/state/tool_call_stream.jsonl')
  })

  it('provides a clean default state', () => {
    expect(defaultToolCallStreamAdapterState()).toEqual({
      status: 'idle',
      lastRecord: null,
      records: [],
      error: null,
    })
  })

  it('tolerates malformed JSONL lines and still returns valid records from the stream tail', () => {
    const ledger = [
      singleLine(LEDGER[0]),
      singleLine({ invalid: true }),
      '\n',
      singleLine(LEDGER[1]),
      singleLine(LEDGER[2]),
    ].join('')

    const state = consumeToolCallStreamTail(ledger, defaultToolCallStreamAdapterState())

    expect(state.status).toBe('polling')
    expect(state.records.map((record) => record.tool)).toEqual(['terminal', 'web_search', 'patch'])
    expect(state.lastRecord).toEqual({
      timestamp: 1710000002000,
      tool: 'patch',
      params: { path: '/missing/file' },
      success: true,
    })
  })

  it('falls back to idle once stale state was idle', () => {
    const state = consumeToolCallStreamTail('', { ...defaultToolCallStreamAdapterState(), status: 'idle' })

    expect(state).toEqual(defaultToolCallStreamAdapterState())
  })

  it('preserves stale error state when no records parse', () => {
    const state = consumeToolCallStreamTail(
      '{bad',
      { status: 'error', lastRecord: null, records: [], error: 'previous parse failure' },
    )

    expect(state.status).toBe('error')
    expect(state.error).toBe('previous parse failure')
    expect(state.records).toHaveLength(0)
  })

  it('truncates very long output for web surfaces without breaking the record schema', () => {
    const record = {
      timestamp: Date.now(),
      tool: 'web_search',
      params: { query: 'agent runtime streaming' },
      output: 'A'.repeat(700),
      duration_ms: 88,
      success: true,
    }

    const html = createToolCallStreamHtml(record)
    expect(html).not.toContain('A'.repeat(700))
    expect(html).toContain('web_search')
    expect(html).toContain('88 ms')
    expect(html).toContain('env:web')
  })

  it('marks successful tool calls with OK and failures with ERR', () => {
    expect(createToolCallStreamHtml({ ...LEDGER[0], success: true })).toContain('tool-call--success')
    expect(createToolCallStreamHtml({ ...LEDGER[0], success: false, error: 'boom' })).toContain('tool-call--failed')
  })
})
