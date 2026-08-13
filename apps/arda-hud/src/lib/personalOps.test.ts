import { afterEach, describe, expect, it, vi } from 'vitest'
import { createPersonalOpsClient, loadConfiguredOperatorId } from './personalOps'

describe('configured personal operations identity', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
  })

  it('loads and trims the daemon-configured operator identity', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({
      operator_id: ' operator:primary ',
    }), { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)

    await expect(loadConfiguredOperatorId('http://127.0.0.1:7878')).resolves.toBe('operator:primary')
    expect(fetchMock).toHaveBeenCalledWith('http://127.0.0.1:7878/v1/status')
  })

  it('fails closed when status omits the configured identity', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify({ daemon: 'arda' }), {
      status: 200,
    })))

    await expect(loadConfiguredOperatorId()).rejects.toThrow('did not publish a configured operator identity')
  })

  it('rejects a blank identity before issuing requests', () => {
    expect(() => createPersonalOpsClient('   ')).toThrow('require a configured operator identity')
  })

  it('uses the normalized identity in read headers and mutation bodies', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ event_id: 'event-1', capture_id: 'capture-1' }), {
      status: 201,
    }))
    vi.stubGlobal('fetch', fetchMock)
    const client = createPersonalOpsClient(' operator:primary ', 'http://127.0.0.1:7878')

    await client.createCapture('configured identity')

    const init = fetchMock.mock.calls[0][1] as RequestInit
    expect(init.headers).toMatchObject({ 'x-arda-operator-id': 'operator:primary' })
    expect(JSON.parse(String(init.body))).toMatchObject({ operator_id: 'operator:primary' })
  })
})
