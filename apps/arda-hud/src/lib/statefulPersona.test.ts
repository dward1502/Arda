// sigil: REPAIR
import { describe, expect, it, vi } from 'vitest'
import {
  loadStatefulPersona,
  parseStatefulPersonaRecord,
  type PersonaFileReader,
} from './statefulPersona'

const canonicalProjection = JSON.stringify({
  id: 'persona:arandur:projection',
  actor: 'arandur',
  extensions: {
    'persona.schema_version': 1,
    'persona.traits': [
      {
        trait_id: 'direct',
        label: 'Direct',
        evidence_count: 4,
        confidence: 0.4,
        first_seen: '2026-05-01T00:00:00Z',
        last_seen: '2026-07-31T00:00:00Z',
        last_reinforced_by: 'evidence-4',
        stale: false,
      },
      {
        trait_id: 'curious',
        label: 'Curious',
        evidence_count: 3,
        confidence: 0.3,
        first_seen: '2026-01-01T00:00:00Z',
        last_seen: '2026-02-01T00:00:00Z',
        last_reinforced_by: 'evidence-3',
        stale: true,
      },
    ],
    'persona.mood': [],
    'persona.mood_summary': {
      as_of: '2026-08-03T12:00:00Z',
      weighted_valence: 0.42,
      sample_count: 6,
      window_hours: 336,
    },
    'persona.value_evidence': [],
  },
})

describe('statefulPersona', () => {
  it('parses traits and mood from the canonical Vaire projection record', () => {
    const persona = parseStatefulPersonaRecord(canonicalProjection, 'arandur')

    expect(persona.status).toBe('ready')
    expect(persona.actor).toBe('arandur')
    expect(persona.sourceRecordId).toBe('persona:arandur:projection')
    expect(persona.traits).toEqual([
      expect.objectContaining({ traitId: 'direct', label: 'Direct', evidenceCount: 4, confidence: 0.4, stale: false }),
      expect.objectContaining({ traitId: 'curious', label: 'Curious', evidenceCount: 3, confidence: 0.3, stale: true }),
    ])
    expect(persona.moodSummary).toEqual(expect.objectContaining({ weightedValence: 0.42, sampleCount: 6 }))
  })

  it('returns a neutral unavailable state for malformed or absent projection data', () => {
    const persona = parseStatefulPersonaRecord('{"extensions":{"persona.schema_version":1}}', 'arandur')

    expect(persona).toMatchObject({
      actor: 'arandur',
      status: 'unavailable',
      traits: [],
      moodSummary: null,
    })
    expect(persona.message).toMatch(/projection unavailable/i)
  })

  it('loads the actor projection through the existing file reader boundary', async () => {
    const reader = vi.fn<PersonaFileReader>().mockResolvedValue({
      success: true,
      content: canonicalProjection,
      error: null,
      path: '/arda/data/mnemosyne/persona/arandur.json',
    })

    const persona = await loadStatefulPersona('/arda', 'arandur', reader)

    expect(reader).toHaveBeenCalledWith('/arda/data/mnemosyne/persona/arandur.json')
    expect(persona.status).toBe('ready')
  })
})
