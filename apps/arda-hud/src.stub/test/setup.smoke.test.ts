// sigil: REPAIR
import { describe, expect, it } from 'vitest'

describe('vitest browser-like setup', () => {
  it('provides jsdom window and localStorage for HUD modules', () => {
    expect(window).toBeDefined()
    expect(window.localStorage).toBeDefined()

    window.localStorage.setItem('arda.setup.smoke', 'ready')
    expect(window.localStorage.getItem('arda.setup.smoke')).toBe('ready')
  })
})
