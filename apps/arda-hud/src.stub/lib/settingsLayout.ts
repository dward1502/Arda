import { ModuleId } from "../components/arda/types"
import {ArdaSection} from "./ardaSource"
import { parseJsonOrNull } from './jsonParse'

export const MODULE_STORAGE_KEY = 'arda.module.order.v1'
const DEFAULT_MODULE_ORDER: ModuleId[] = [
  'operating_surface',
  'executive_overview',
  'section_focus',
  'human_realm',
  'systems',
  'governance_controls',
  'operations_and_packages',
  'hermes_dashboard',
  'planning',
  'learning_loop',
  'business',
  'personal_growth',
  'culture_and_art',
  'service_embed',
  'media_library',
  'settings',
]
const PANEL_LAYOUTS: Record<string, ModuleId[]> = {
  sovereign_world: ['operating_surface', 'executive_overview', 'systems'],
  now_command: ['operating_surface', 'executive_overview', 'governance_controls', 'systems'],
  governance_guardhouse: ['governance_controls', 'operating_surface'],
  decisions: ['governance_controls', 'operating_surface'],
  knowledge_and_reasoning: ['human_realm', 'section_focus'],
  routing_and_comms: ['section_focus', 'operations_and_packages'],
  systems_health: ['systems', 'operations_and_packages'],
  routing_health: ['operations_and_packages', 'governance_controls'],
  hermes_dashboard: ['hermes_dashboard', 'operations_and_packages'],
  lifecycle_execution_economics: ['planning', 'operations_and_packages'],
  memory_and_continuity: ['human_realm', 'section_focus'],
  planning_and_queue: ['planning', 'learning_loop', 'operations_and_packages', 'section_focus'],
  business_ops: ['business', 'planning', 'operations_and_packages'],
  evidence_trust: ['operating_surface', 'systems', 'human_realm'],
  personal_growth: ['personal_growth', 'human_realm'],
  culture_and_art: ['culture_and_art', 'human_realm'],
  service_factory_ai: ['service_embed'],
  service_warp_dev: ['service_embed'],
  service_vast_ai_os: ['service_embed'],
  service_beelink_grafana: ['service_embed'],
  service_beelink_openwebui: ['service_embed'],
  media_library: ['media_library', 'service_embed'],
  agent_remote_session: ['service_embed'],
}
const PANEL_TITLES: Record<string, string> = {
  sovereign_world: 'Sovereign World',
  now_command: 'Now Command Surface',
  governance_guardhouse: 'Governance Guardhouse',
  decisions: 'Decisions',
  knowledge_and_reasoning: 'Knowledge And Reasoning',
  routing_and_comms: 'Routing And Comms',
  systems_health: 'Fleet Systems Health',
  routing_health: 'Routing Health',
  hermes_dashboard: 'Hermes Dashboard',
  lifecycle_execution_economics: 'Lifecycle Execution Economics',
  memory_and_continuity: 'Memory And Continuity',
  planning_and_queue: 'Planning And Queue',
  business_ops: 'Business Operations',
  evidence_trust: 'Evidence And Trust',
  personal_growth: 'Personal Growth',
  culture_and_art: 'Culture And Art',
  service_factory_ai: 'Factory AI Surface',
  service_warp_dev: 'Warp Surface',
  service_vast_ai_os: 'VAST AI OS Surface',
  service_beelink_grafana: 'Beelink Grafana',
  service_beelink_openwebui: 'Beelink Open WebUI',
  media_library: 'ARDA Media Library',
  agent_remote_session: 'Agent Remote Session',
  settings: 'Settings',
}

export function localStorageOrNull(): Storage | null {
  try {
    return typeof window === 'undefined' ? null : window.localStorage
  } catch {
    return null
  }
}
export function readStoredModuleOrder(): ModuleId[] {
  try {
    const raw = localStorageOrNull()?.getItem(MODULE_STORAGE_KEY)
    if (!raw) return DEFAULT_MODULE_ORDER
    const parsed = parseJsonOrNull<unknown>(raw)
    if (!Array.isArray(parsed)) return DEFAULT_MODULE_ORDER
    const ordered = parsed.filter((item): item is ModuleId => DEFAULT_MODULE_ORDER.includes(item as ModuleId))
    return ordered.length === DEFAULT_MODULE_ORDER.length ? ordered : DEFAULT_MODULE_ORDER
  } catch {
    return DEFAULT_MODULE_ORDER
  }
}

export function moveModule(order: ModuleId[], moduleId: ModuleId, direction: 'up' | 'down'): ModuleId[] {
  const index = order.indexOf(moduleId)
  if (index === -1) return order
  const targetIndex = direction === 'up' ? index - 1 : index + 1
  if (targetIndex < 0 || targetIndex >= order.length) return order
  const next = [...order]
  const [item] = next.splice(index, 1)
  next.splice(targetIndex, 0, item)
  return next
}

export function sectionToPanelLayout(sectionId: string | null): ModuleId[] {
  if (!sectionId) return ['executive_overview', 'section_focus']
  return PANEL_LAYOUTS[sectionId] ?? ['section_focus', 'systems']
}

export function formatSectionStatus(section: ArdaSection | null): string {
  if (!section) return 'No focus'
  return `${section.status} / ${section.owner}`
}

export function formatPanelStatus(sectionId: string | null, section: ArdaSection | null): string {
  if (section) return formatSectionStatus(section)
  if (!sectionId) return 'No focus'
  if (sectionId === 'systems_health') return 'Fleet command / local mesh'
  if (sectionId === 'routing_health') return 'Charon router / lane command'
  if (sectionId === 'settings') return 'System settings / operator controls'
  return 'Focused workstation'
}

export function formatProviderLabel(value: string | null | undefined): string {
  if (!value) return 'n/a'
  return value.split('_').join(' ')
}

export function asModuleId(value: string | null | undefined): ModuleId | null {
  if (!value) return null
  return DEFAULT_MODULE_ORDER.includes(value as ModuleId) ? (value as ModuleId) : null
}

export function titleForSectionOrPanel(sectionId: string | null, sections: ArdaSection[]): string {
  if (!sectionId) return 'Focused Panel'
  return sections.find((section) => section.id === sectionId)?.title ?? PANEL_TITLES[sectionId] ?? sectionId
}