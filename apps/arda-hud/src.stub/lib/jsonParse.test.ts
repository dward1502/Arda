// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { compactJsonPreview, jsonParseFailureMessage, parseJsonOrDefault, parseJsonOrNull, parseJsonResult } from './jsonParse'

describe('jsonParse helpers', () => {
  it('returns a typed success result for valid JSON', () => {
    const parsed = parseJsonResult<{ ok: boolean }>(`{ "ok": true }`)

    expect(parsed.ok).toBe(true)
    if (parsed.ok) {
      expect(parsed.value).toEqual({ ok: true })
    }
  })

  it('captures parse diagnostics without throwing', () => {
    const parsed = parseJsonResult('{ "broken": true,, }')

    expect(parsed.ok).toBe(false)
    if (!parsed.ok) {
      expect(parsed.error).toContain('JSON')
      expect(parsed.preview).toContain('broken')
      expect(jsonParseFailureMessage('settings', parsed)).toContain('settings JSON parse failed')
    }
  })

  it('preserves fallback behavior for null/default call sites', () => {
    expect(parseJsonOrNull('{not-json')).toBeNull()
    expect(parseJsonOrDefault('{not-json', { fallback: true })).toEqual({ fallback: true })
  })

  it('compacts previews to bounded single-line text', () => {
    expect(compactJsonPreview(' {\n  "value": "abcdefghij"\n } ', 12)).toBe('{ "value": …')
  })
})
