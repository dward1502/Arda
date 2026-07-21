/** Pure bundle/ui derivation helpers extracted from `App.tsx`. */
import type { ArdaSection } from './ardaSource'
import type { ModuleId, SourceCoverageBadgeState } from '../components/arda/types'

export const FLOATING_WORKSTATION_MARGIN = 28
export const FLOATING_WORKSTATION_TILE_GAP = 18

export function clampFloatingWorkstationValue(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(value, max))
}

export function getFloatingWorkstationViewport(): { width: number, height: number } {
  if (typeof window === 'undefined') {
    return { width: 1440, height: 900 }
  }
  return { width: window.innerWidth, height: window.innerHeight }
}

export function getFloatingWorkstationTileLayout(index: number, total: number) {
  const viewport = getFloatingWorkstationViewport()
  const safeTotal = Math.max(1, total)
  const availableWidth = Math.max(360, viewport.width - FLOATING_WORKSTATION_MARGIN * 2)
  const availableHeight = Math.max(280, viewport.height - FLOATING_WORKSTATION_MARGIN * 2)

  if (safeTotal === 1) {
    const width = Math.min(940, availableWidth)
    const height = Math.min(680, availableHeight)
    return {
      x: Math.round(FLOATING_WORKSTATION_MARGIN + (availableWidth - width) / 2),
      y: Math.round(FLOATING_WORKSTATION_MARGIN + Math.max(0, (availableHeight - height) * 0.28)),
      width,
      height,
    }
  }

  const columns = safeTotal <= 4 ? 2 : Math.min(3, Math.ceil(Math.sqrt(safeTotal)))
  const rows = Math.ceil(safeTotal / columns)
  const gap = FLOATING_WORKSTATION_TILE_GAP
  const margin = FLOATING_WORKSTATION_MARGIN
  const tileWidth = Math.floor((availableWidth - gap * (columns - 1)) / columns)
  const tileHeight = Math.floor((availableHeight - gap * (rows - 1)) / rows)
  const row = Math.floor(index / columns)
  const column = index % columns
  const rowItemCount = Math.min(columns, safeTotal - row * columns)
  const rowWidth = rowItemCount * tileWidth + Math.max(0, rowItemCount - 1) * gap
  const rowOffset = Math.max(0, (availableWidth - rowWidth) / 2)

  return {
    x: Math.round(margin + rowOffset + column * (tileWidth + gap)),
    y: Math.round(margin + row * (tileHeight + gap)),
    width: clampFloatingWorkstationValue(tileWidth, 320, availableWidth),
    height: clampFloatingWorkstationValue(tileHeight, 240, availableHeight),
  }
}

export function getFloatingWorkstationCenteredLayout() {
  const viewport = getFloatingWorkstationViewport()
  const margin = FLOATING_WORKSTATION_MARGIN
  const availableWidth = Math.max(360, viewport.width - margin * 2)
  const availableHeight = Math.max(280, viewport.height - margin * 2)
  const width = Math.min(940, availableWidth)
  const height = Math.min(680, availableHeight)

  return {
    x: Math.round(margin + Math.max(0, (availableWidth - width) / 2)),
    y: Math.round(margin + Math.max(0, (availableHeight - height) / 2)),
    width,
    height,
  }
}

export function isDerivedRecord(record: { authority?: string } | null): boolean {
  return typeof record?.authority === 'string' && record.authority.startsWith('arda_derived')
}

export interface JsonRecord {
  authority?: string
}

export function provenanceTag(record: JsonRecord | null, label: string): string {
  if (!record) return `${label}: missing`
  return `${label}: ${isDerivedRecord(record) ? 'Derived' : 'Projected'}`
}

export function statusTone(status: string): 'gold' | 'cyan' | 'ember' | 'mint' | 'violet' {
  const normalized = status.toLowerCase()
  if (normalized.includes('ready') || normalized.includes('healthy') || normalized.includes('online')) return 'mint'
  if (normalized.includes('attention') || normalized.includes('degraded')) return 'ember'
  if (normalized.includes('offline') || normalized.includes('lock')) return 'violet'
  return 'cyan'
}

export function sourceCoverageForSections(sections: ArdaSection[]): SourceCoverageBadgeState | undefined {
  if (sections.length === 0) return undefined

  const missingCount = sections.reduce((count, section) => count + (section.missing_projections?.length ?? 0), 0)
  if (missingCount > 0) {
    return { status: 'partial', label: 'source map partial', missingCount }
  }

  return { status: 'backed', label: 'source map backed', missingCount: 0 }
}

export function sourceCoverageForPanel(sections: ArdaSection[], panelId: ModuleId): SourceCoverageBadgeState | undefined {
  const mappedSections = sections.filter((section) => section.arda_panels.includes(panelId))
  if (mappedSections.length === 0) return { status: 'unmapped', label: 'source map unmapped', missingCount: 0 }
  return sourceCoverageForSections(mappedSections)
}
