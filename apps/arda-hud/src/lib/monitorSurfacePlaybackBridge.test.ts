import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { agentPatchMonitorSurfacePlayback } from './boardroomSlotSettings'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockedInvoke = vi.mocked(invoke)

describe('monitor surface playback bridge', () => {
  beforeEach(() => mockedInvoke.mockReset())

  it('patches authoritative playback with optimistic revision control', async () => {
    mockedInvoke.mockResolvedValue({
      ok: true,
      message: 'patched',
      registry: {
        schemaVersion: 'arda.monitor-session-registry.v2',
        updatedAtUtc: '2026-08-10T16:00:00.000Z',
        sessions: {
          monitor_1: {
            slotId: 'monitor_1',
            sessionId: 'session-video',
            surfaceSessionId: 'session-video',
            owner: 'agent:video',
            kind: 'video',
            revision: 3,
            openedAtUtc: '2026-08-10T15:00:00.000Z',
            leaseExpiresAtUtc: '2026-08-10T17:00:00.000Z',
            content: {
              kind: 'video',
              source: { kind: 'local', path: 'docs/media/demo.mp4' },
              fit: 'contain',
            },
            playback: { playing: false, currentTime: 42.5, volume: 0.25 },
            workstationHandoff: { sessionId: 'session-video', mode: 'same_live_session' },
            createdAtUtc: '2026-08-10T15:00:00.000Z',
            updatedAtUtc: '2026-08-10T16:00:00.000Z',
          },
        },
      },
      session: { surfaceSessionId: 'session-video' },
    })

    const result = await agentPatchMonitorSurfacePlayback(
      'session-video',
      { kind: 'agent', name: 'video' },
      2,
      { playing: false, currentTime: 42.5, volume: 0.25 },
    )

    expect(mockedInvoke).toHaveBeenCalledWith('patch_monitor_surface_playback', {
      request: {
        surfaceSessionId: 'session-video',
        owner: 'agent:video',
        expectedRevision: 2,
        playback: { playing: false, currentTime: 42.5, volume: 0.25 },
      },
    })
    expect(result.session?.revision).toBe(3)
    expect(result.session?.playback).toEqual({
      playing: false,
      currentTime: 42.5,
      volume: 0.25,
    })
  })
})
