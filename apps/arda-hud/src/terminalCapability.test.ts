// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import defaultCapability from '../src-tauri/capabilities/default.json'

describe('Hermes terminal capability', () => {
  it('allows the dedicated PTY window to invoke its shell commands', () => {
    expect(defaultCapability.windows).toContain('arda-hermes-terminal')
  })
})
