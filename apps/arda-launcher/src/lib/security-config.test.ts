import { describe, expect, it } from 'vitest'

import rawConfig from '../../src-tauri/tauri.conf.json?raw'

describe('packaged Tauri security configuration', () => {
  it('enforces a restrictive content security policy', () => {
    const config = JSON.parse(rawConfig)
    const csp = config.app?.security?.csp

    expect(csp).toBeTypeOf('object')
    expect(csp['default-src']).toContain("'self'")
    expect(csp['object-src']).toBe("'none'")
    expect(csp['frame-ancestors']).toBe("'none'")
    expect(Object.values(csp).join(' ')).not.toContain("'unsafe-eval'")
  })
})