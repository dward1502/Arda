import { describe, expect, it } from 'vitest'
import {
  getStaticWorkstationManifest,
  resolveWorkstationComposition,
} from './workstationComposition'
import { sectionToPanelLayout } from './settingsLayout'
import { getSceneSlotWorkstationTemplates } from '../scene/workstations/sceneSlotWorkstationTemplates'

const canonicalLowerCompositions = {
  governance_guardhouse: ['governance_controls'],
  fleet_and_backbone: ['systems', 'operations_and_packages'],
  routing_and_comms: ['systems', 'operations_and_packages'],
  human_business_personal: ['human_realm', 'business', 'personal_growth'],
} as const

describe('workstation composition authority', () => {
  it('owns the canonical module composition for every configurable lower workstation', () => {
    for (const [sourceZoneId, moduleIds] of Object.entries(canonicalLowerCompositions)) {
      expect(resolveWorkstationComposition(sourceZoneId, []).moduleIds).toEqual(moduleIds)
      expect(sectionToPanelLayout(sourceZoneId)).toEqual(moduleIds)
    }
  })

  it('drives lower scene-slot fallback templates from the canonical compositions', () => {
    const templates = getSceneSlotWorkstationTemplates()

    expect(templates.view_desk_l.moduleIds).toEqual(canonicalLowerCompositions.governance_guardhouse)
    expect(templates.view_desk_control_panel.moduleIds).toEqual(canonicalLowerCompositions.fleet_and_backbone)
    expect(templates.view_desk_r.moduleIds).toEqual(canonicalLowerCompositions.routing_and_comms)
    expect(templates.view_desk_aux.moduleIds).toEqual(canonicalLowerCompositions.human_business_personal)
  })

  it('owns static utility manifests instead of duplicating them in lookup code', () => {
    expect(getStaticWorkstationManifest('settings')).toMatchObject({
      id: 'settings_workstation',
      module_ids: ['settings'],
      presentation_modes: ['in_scene', 'native_window'],
    })
    expect(getStaticWorkstationManifest('hermes_runtime')).toMatchObject({
      id: 'hermes_dashboard_workstation',
      module_ids: ['hermes_dashboard', 'operations_and_packages'],
      presentation_modes: ['in_scene', 'native_window'],
    })
    expect(getStaticWorkstationManifest('unknown')).toBeNull()
  })
})
