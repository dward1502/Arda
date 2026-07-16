import { describe, it, expect } from 'vitest'
import {
  ADAPTER_DOC_PATH,
  defaultDiffStreamAdapterState,
  parseFixedLengthTail,
  consumeDiffStreamTail,
  createDiffStreamHtml,
} from './diffStreamAdapter'

describe('diffStreamAdapter', () => {
  it('exposes the intended edit stream document path', () => {
    expect(ADAPTER_DOC_PATH).toBe('core/state/agent_edit_stream.jsonl')
  })

  it('defaults to an idle adapter state', () => {
    expect(defaultDiffStreamAdapterState()).toEqual({
      status: 'idle',
      lastRecord: null,
      records: [],
      error: null,
    })
  })

  it('returns a healthy polling state when the stream has a valid record', () => {
    const record = {
      timestamp: Date.now(),
      file: 'src/lib/foo.ts',
      before_hash: 'abc',
      after_hash: 'def',
      diff: '@@ -1,2 +1,2 @@\n-old\n+new',
      success: true,
      error: null,
    }
    const content = JSON.stringify(record) + '\n'
    const state = consumeDiffStreamTail(content, defaultDiffStreamAdapterState())

    expect(state.status).toBe('polling')
    expect(state.error).toBeNull()
    expect(state.lastRecord).toEqual(record)
    expect(state.records).toHaveLength(1)
  })

  it('ignores malformed JSONL lines without losing later records', () => {
    const validRecord = {
      timestamp: Date.now(),
      file: 'src/lib/foo.ts',
      diff: '@@ -1 +1 @@\n-old',
      success: true,
      error: null,
    }
    const content = `not-json\n${JSON.stringify(validRecord)}\n`
    const state = consumeDiffStreamTail(content, defaultDiffStreamAdapterState())

    expect(state.status).toBe('polling')
    expect(state.error).toBeTruthy()
    expect(state.lastRecord).toEqual(validRecord)
  })

  it('switches to error state when JSON parsing fails for every line', () => {
    const state = consumeDiffStreamTail('}', defaultDiffStreamAdapterState())

    expect(state.status).toBe('error')
    expect(state.lastRecord).toBeNull()
    expect(state.records).toHaveLength(0)
    expect(state.error).toBeTruthy()
  })

  it('records a parse warning but stays polling when at least one valid record is found', () => {
    const mixed = `invalid line\n${JSON.stringify({ timestamp: 1, file: 'x.ts', diff: '' })}\n`
    const state = consumeDiffStreamTail(mixed, defaultDiffStreamAdapterState())

    expect(state.status).toBe('polling')
    expect(state.error).toBeTruthy()
    expect(state.lastRecord?.file).toBe('x.ts')
  })

  it('renders without escaping diff fragment markers', () => {
    const record = {
      timestamp: Date.now(),
      file: 'README.md',
      diff: '@@ -1,2 +1,2 @@\n-old\n+new',
      success: true,
      error: null,
    }
    const html = createDiffStreamHtml(record.diff, record)

    expect(html).toContain('+new')
    expect(html).toContain('-old')
    expect(html).toContain('@@ -1')
    expect(html).toContain('<span class="diff-success">✓</span>')
  })

  it('preserves only the latest record when multiple JSONL records are present in the tail', () => {
    const recentRecord = {
      timestamp: Date.now(),
      file: 'src/lib/bar.ts',
      diff: '@@ -1 +1 @@\n-x',
      success: false,
      error: 'write failed',
    }
    const olderRecord = { timestamp: 0, file: 'old.ts', diff: '---' }
    const content = `${JSON.stringify(olderRecord)}\n${JSON.stringify(recentRecord)}\n`
    const state = consumeDiffStreamTail(content, defaultDiffStreamAdapterState())

    expect(state.status).toBe('polling')
    expect(state.lastRecord).toEqual(recentRecord)
  })
})
