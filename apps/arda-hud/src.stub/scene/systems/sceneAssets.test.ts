// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { getSceneAssetByBinding } from './sceneAssets'

describe('sceneAssets', () => {
  it('registers the controlled ARDA workstation GLB with provenance metadata', () => {
    const asset = getSceneAssetByBinding('controlled_arda_workstation')

    expect(asset?.metadata.id).toBe('world_controlled_arda_workstation')
    expect(asset?.metadata.scene_binding).toBe('controlled_arda_workstation')
    expect(asset?.metadata.source).toContain('arda_pixal3d_spike')
    expect(asset?.glbUrl).toBeTruthy()
  })
})
