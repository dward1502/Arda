// sigil: REPAIR
import type { WorldSurfaceLayout } from '../../lib/worldSurfaceSettings'
import type { SceneZoneDefinition } from '../systems/runtimeTypes'
import { contractForZone, type WorldDistrictAction, type WorldDistrictContract } from './worldDistrictContracts'

export interface WorldDistrictWorkflow {
  districtId: string
  title: string
  ownerAgent: string
  sourceZoneId: string
  openTargetZoneId: string
  workstationId: string | null
  transitionLabel: string
  adapterType: string
  focusMode: string
  previewWidgetCount: number
  safeActions: WorldDistrictAction[]
  gatedActions: WorldDistrictAction[]
  acceptanceSummary: string
}

function workflowForContract(
  contract: WorldDistrictContract,
  surfaceLayout: WorldSurfaceLayout | undefined,
): WorldDistrictWorkflow {
  const openTargetZoneId = surfaceLayout?.focus.target ?? contract.primaryActions[0]?.target ?? contract.sourceZoneId
  return {
    districtId: contract.districtId,
    title: contract.title,
    ownerAgent: contract.ownerAgent,
    sourceZoneId: contract.sourceZoneId,
    openTargetZoneId,
    workstationId: contract.workstationId,
    transitionLabel: `Opening ${contract.title} district`,
    adapterType: surfaceLayout?.adapter_type ?? 'contract_only',
    focusMode: surfaceLayout?.focus.mode ?? 'in_scene_workstation',
    previewWidgetCount: surfaceLayout?.preview.widgets.length ?? 0,
    safeActions: [...contract.primaryActions, ...contract.safeActions].filter((action) => action.guard === 'safe'),
    gatedActions: contract.gatedActions.filter((action) => action.guard !== 'safe'),
    acceptanceSummary: contract.acceptanceCriteria.join('; '),
  }
}

export function resolveWorldDistrictWorkflow(
  zone: SceneZoneDefinition,
  surfaceLayouts: Record<string, WorldSurfaceLayout> = {},
): WorldDistrictWorkflow {
  const contract = contractForZone(zone)
  if (!contract) {
    return {
      districtId: zone.id,
      title: zone.title,
      ownerAgent: zone.owner,
      sourceZoneId: zone.sourceIds[0] ?? zone.id,
      openTargetZoneId: zone.sourceIds[0] ?? zone.id,
      workstationId: zone.workstationIds[0] ?? null,
      transitionLabel: `Opening ${zone.title} district`,
      adapterType: 'unmapped',
      focusMode: 'in_scene_workstation',
      previewWidgetCount: 0,
      safeActions: [{ id: 'open-unmapped-district', label: 'Open district panel', guard: 'safe', target: zone.sourceIds[0] ?? zone.id }],
      gatedActions: [],
      acceptanceSummary: 'district contract missing',
    }
  }

  return workflowForContract(contract, surfaceLayouts[contract.districtId] ?? surfaceLayouts[zone.id])
}

export function resolveWorldDistrictWorkflows(
  zones: SceneZoneDefinition[],
  surfaceLayouts: Record<string, WorldSurfaceLayout> = {},
): Record<string, WorldDistrictWorkflow> {
  return Object.fromEntries(
    zones
      .filter((zone) => zone.scene === 'world')
      .map((zone) => [zone.id, resolveWorldDistrictWorkflow(zone, surfaceLayouts)]),
  )
}
