// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { envEndpointUrl, envString, loopbackUrl } from './endpointConfig'

describe('endpoint config helpers', () => {
  it('normalizes loopback URLs without trailing slash drift', () => {
    expect(loopbackUrl({ port: 8000 })).toBe('http://127.0.0.1:8000')
    expect(loopbackUrl({ scheme: 'https', host: 'localhost', port: '9119' })).toBe('https://localhost:9119')
  })

  it('prefers explicit env URLs and trims trailing slashes', () => {
    expect(envEndpointUrl({ url: ' http://example.test:1234/// ', port: 8000 })).toBe('http://example.test:1234')
  })

  it('builds a loopback default when URL env is absent', () => {
    expect(envEndpointUrl({ url: '', scheme: 'http', host: '127.0.0.1', port: '5110' })).toBe('http://127.0.0.1:5110')
  })

  it('normalizes string env defaults', () => {
    expect(envString('  abc  ', 'fallback')).toBe('abc')
    expect(envString('', 'fallback')).toBe('fallback')
    expect(envString(undefined, 'fallback')).toBe('fallback')
  })
})
