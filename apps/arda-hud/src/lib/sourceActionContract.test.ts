// sigil: REPAIR
import { describe, expect, it } from 'vitest'
import type { ArdaSourceProvenance } from './ardaProvenance'
import { buildSourceActionContract, formatSourceActionPacket } from './sourceActionContract'

const record: ArdaSourceProvenance = {
  domainId: 'planning:queue',
  label: 'Queue Ledger',
  sourcePaths: ['core/projects/tasks/queue.jsonl', 'core/state/queue_summary.json'],
  generatedAtUtc: '2026-06-01T07:00:00Z',
  observedAtUtc: '2026-06-01T07:01:00Z',
  state: 'fresh',
  sourceKind: 'snapshot',
  derivedFrom: ['core/projects/tasks/queue.jsonl'],
  notes: 'Queue projection loaded.',
}

describe('sourceActionContract', () => {
  it('builds safe copy/export/reveal actions and gates external sharing', () => {
    const contract = buildSourceActionContract(record)

    expect(contract.safeActions.map((action) => action.id)).toEqual(['copy_source_paths', 'copy_source_packet', 'reveal_native_source'])
    expect(contract.gatedActions.map((action) => action.id)).toEqual(['share_source_packet'])
    expect(contract.packet.sourcePaths).toEqual(record.sourcePaths)
    expect(contract.safeActions[2]?.target).toBe('core/projects/tasks/queue.jsonl')
    expect(contract.gatedActions[0]?.reason).toContain('external sharing')
  })

  it('formats a stable provenance packet', () => {
    const packet = formatSourceActionPacket(buildSourceActionContract(record))

    expect(packet).toContain('"domainId": "planning:queue"')
    expect(packet).toContain('"sourcePaths"')
    expect(JSON.parse(packet)).toMatchObject({
      label: 'Queue Ledger',
      state: 'fresh',
      sourceKind: 'snapshot',
    })
  })
})
