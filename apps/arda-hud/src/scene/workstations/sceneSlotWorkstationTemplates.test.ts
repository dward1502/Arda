// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import { BOARDROOM_SCENE_SLOT_IDS } from '../../lib/boardroomSlotSettings'
import {
  getSceneSlotWorkstationManifestById,
  getSceneSlotWorkstationManifestByZoneId,
  getSceneSlotWorkstationTemplates,
  sceneSlotZoneIdForSlot,
} from './sceneSlotWorkstationTemplates'

describe('sceneSlotWorkstationTemplates', () => {
  it('defines a real workstation template for every boardroom scene slot', () => {
    const templates = getSceneSlotWorkstationTemplates()

    expect(Object.keys(templates).sort()).toEqual([...BOARDROOM_SCENE_SLOT_IDS].sort())

    for (const slotId of BOARDROOM_SCENE_SLOT_IDS) {
      const manifest = getSceneSlotWorkstationManifestByZoneId(sceneSlotZoneIdForSlot(slotId))

      expect(manifest?.title).not.toMatch(/placeholder/i)
      expect(manifest?.source_zone_id).toBe(`scene_slot:${slotId}`)
      expect(manifest?.entry_anchor_id).toBe(`${slotId}_template_entry`)
      expect(manifest?.module_ids.length).toBeGreaterThan(0)
      expect(manifest?.presentation_modes).toContain('in_scene')
    }
  })

  it('round-trips slot manifests by manifest id', () => {
    const manifest = getSceneSlotWorkstationManifestByZoneId('scene_slot:view_desk_control_panel')

    expect(manifest?.id).toBe('scene_slot_view_desk_control_panel_workstation')
    expect(manifest?.module_ids).toContain('operating_surface')
    expect(getSceneSlotWorkstationManifestById(manifest?.id ?? null)).toEqual(manifest)
  })

  it('rejects unknown slot ids instead of producing generic placeholders', () => {
    expect(getSceneSlotWorkstationManifestByZoneId('scene_slot:unknown')).toBeNull()
    expect(getSceneSlotWorkstationManifestById('scene_slot_unknown_workstation')).toBeNull()
    expect(getSceneSlotWorkstationManifestByZoneId(null)).toBeNull()
  })
})
