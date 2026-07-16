// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import {
  SURFACE_ADAPTER_MANIFESTS,
  getSurfaceAdapterFocusContract,
  getSurfaceAdapterManifest,
  getSurfaceAdapterWorkstationManifests,
} from './surfaceAdapterManifests'

describe('surface adapter manifests', () => {
  it('defines third-party service surfaces without ARDA-specific source zones', () => {
    expect(SURFACE_ADAPTER_MANIFESTS.map((manifest) => manifest.sourceZoneId)).toEqual([
      'service_factory_ai',
      'service_warp_dev',
      'service_vast_ai_os',
      'service_beelink_grafana',
      'service_beelink_openwebui',
      'media_library',
      'agent_remote_session',
    ])
    expect(SURFACE_ADAPTER_MANIFESTS.filter((manifest) => manifest.kind === 'local_service').map((manifest) => manifest.sourceZoneId)).toEqual([
      'service_beelink_grafana',
      'service_beelink_openwebui',
    ])
  })

  it('converts service surfaces into workstation manifests', () => {
    const workstations = getSurfaceAdapterWorkstationManifests()
    expect(workstations).toHaveLength(SURFACE_ADAPTER_MANIFESTS.length)
    expect(workstations[0]).toMatchObject({
      id: 'service_factory_ai_workstation',
      source_zone_id: 'service_factory_ai',
      module_ids: ['service_embed'],
    })
    expect(workstations.find((manifest) => manifest.source_zone_id === 'media_library')).toMatchObject({
      id: 'media_library_workstation',
      module_ids: ['media_library', 'service_embed'],
    })
  })

  it('looks up manifests by source zone id', () => {
    expect(getSurfaceAdapterManifest('service_warp_dev')?.provider).toBe('Warp')
    expect(getSurfaceAdapterManifest('service_beelink_grafana')).toMatchObject({
      provider: 'Grafana',
      externalUrl: 'http://100.103.125.88:3000',
      allowInlineEmbed: false,
      preferredFocusMode: 'native_window',
    })
    expect(getSurfaceAdapterManifest('media_library')).toMatchObject({
      kind: 'media_adapter',
      preferredFocusMode: 'native_window',
    })
    expect(getSurfaceAdapterManifest('routing_and_comms')).toBeNull()
  })

  it('resolves focused adapter contracts with inline-embed status', () => {
    expect(getSurfaceAdapterFocusContract('service_beelink_grafana')).toMatchObject({
      focusMode: 'native_window',
      target: 'http://100.103.125.88:3000',
      inlineStatus: 'blocked',
    })
    expect(getSurfaceAdapterFocusContract('media_library')).toMatchObject({
      focusMode: 'native_window',
      target: null,
      inlineStatus: 'unavailable',
    })
    expect(getSurfaceAdapterFocusContract('agent_remote_session')?.reason).toContain('focused surface path')
  })
})
