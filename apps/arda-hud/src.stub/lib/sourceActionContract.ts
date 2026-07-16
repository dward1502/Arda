// sigil: REPAIR
import type { ArdaSourceProvenance } from './ardaProvenance'

export interface SourceActionContract {
  title: string
  sourceId: string
  safeActions: Array<{
    id: 'copy_source_paths' | 'copy_source_packet' | 'reveal_native_source'
    label: string
    target: string
  }>
  gatedActions: Array<{
    id: 'share_source_packet'
    label: string
    reason: string
  }>
  packet: {
    domainId: string
    label: string
    state: ArdaSourceProvenance['state']
    sourceKind: ArdaSourceProvenance['sourceKind']
    sourcePaths: string[]
    generatedAtUtc: string | null
    observedAtUtc: string | null
    derivedFrom: string[]
    notes: string | null
  }
}

export function buildSourceActionContract(record: ArdaSourceProvenance): SourceActionContract {
  const primaryPath = record.sourcePaths[0] ?? 'no-source-path'
  return {
    title: 'Source Action Contract',
    sourceId: record.domainId,
    safeActions: [
      { id: 'copy_source_paths', label: 'Copy Source Paths', target: primaryPath },
      { id: 'copy_source_packet', label: 'Copy Source Packet', target: record.domainId },
      { id: 'reveal_native_source', label: 'Reveal Native Source', target: primaryPath },
    ],
    gatedActions: [
      {
        id: 'share_source_packet',
        label: 'Share Source Packet',
        reason: 'external sharing requires destination, redaction, and operator approval',
      },
    ],
    packet: {
      domainId: record.domainId,
      label: record.label,
      state: record.state,
      sourceKind: record.sourceKind,
      sourcePaths: [...record.sourcePaths],
      generatedAtUtc: record.generatedAtUtc,
      observedAtUtc: record.observedAtUtc,
      derivedFrom: record.derivedFrom ? [...record.derivedFrom] : [],
      notes: record.notes ?? null,
    },
  }
}

export function formatSourceActionPacket(contract: SourceActionContract): string {
  return JSON.stringify(contract.packet, null, 2)
}
