// sigil: REPAIR
import { parseJsonOrNull } from './jsonParse'
import { readFile, type FileReadResult } from './weathertop'

export interface StatefulPersonaTrait {
  traitId: string
  label: string
  evidenceCount: number
  confidence: number
  stale: boolean
}

export interface StatefulPersonaMoodSummary {
  asOf: string
  weightedValence: number
  sampleCount: number
  windowHours: number
}

export interface StatefulPersona {
  actor: string
  status: 'ready' | 'unavailable'
  sourceRecordId: string | null
  traits: StatefulPersonaTrait[]
  moodSummary: StatefulPersonaMoodSummary | null
  message: string
}

export type PersonaFileReader = (path: string) => Promise<FileReadResult>

type JsonRecord = Record<string, unknown>

function asRecord(value: unknown): JsonRecord | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    ? value as JsonRecord
    : null
}

function finiteNumber(value: unknown): number | null {
  return typeof value === 'number' && Number.isFinite(value) ? value : null
}

function parseTrait(value: unknown): StatefulPersonaTrait | null {
  const record = asRecord(value)
  const traitId = record?.trait_id
  const label = record?.label
  const evidenceCount = finiteNumber(record?.evidence_count)
  const confidence = finiteNumber(record?.confidence)
  if (
    typeof traitId !== 'string'
    || typeof label !== 'string'
    || evidenceCount === null
    || confidence === null
    || typeof record?.stale !== 'boolean'
  ) {
    return null
  }
  return { traitId, label, evidenceCount, confidence, stale: record.stale }
}

function parseMoodSummary(value: unknown): StatefulPersonaMoodSummary | null {
  if (value === null) return null
  const record = asRecord(value)
  const weightedValence = finiteNumber(record?.weighted_valence)
  const sampleCount = finiteNumber(record?.sample_count)
  const windowHours = finiteNumber(record?.window_hours)
  if (
    typeof record?.as_of !== 'string'
    || weightedValence === null
    || sampleCount === null
    || windowHours === null
  ) {
    return null
  }
  return {
    asOf: record.as_of,
    weightedValence,
    sampleCount,
    windowHours,
  }
}

export function unavailableStatefulPersona(actor: string, message = 'Persona projection unavailable.'): StatefulPersona {
  return { actor, status: 'unavailable', sourceRecordId: null, traits: [], moodSummary: null, message }
}

export function parseStatefulPersonaRecord(content: string, actor: string): StatefulPersona {
  const record = parseJsonOrNull<JsonRecord>(content)
  const extensions = asRecord(record?.extensions)
  const schemaVersion = extensions?.['persona.schema_version']
  const rawTraits = extensions?.['persona.traits']
  const hasMoodSummary = extensions
    ? Object.prototype.hasOwnProperty.call(extensions, 'persona.mood_summary')
    : false
  if (schemaVersion !== 1 || !Array.isArray(rawTraits) || !hasMoodSummary) {
    return unavailableStatefulPersona(actor)
  }

  const traits = rawTraits.map(parseTrait).filter((trait): trait is StatefulPersonaTrait => trait !== null)
  const moodSummary = parseMoodSummary(extensions['persona.mood_summary'])
  return {
    actor,
    status: 'ready',
    sourceRecordId: typeof record?.id === 'string' ? record.id : null,
    traits,
    moodSummary,
    message: traits.length > 0 || moodSummary
      ? 'Persona projection loaded from Vairë.'
      : 'Persona projection is current with no promoted traits or mood evidence.',
  }
}

export async function loadStatefulPersona(
  rootPath: string,
  actor: string,
  reader: PersonaFileReader = readFile,
): Promise<StatefulPersona> {
  const path = `${rootPath.replace(/\/$/, '')}/data/mnemosyne/persona/${encodeURIComponent(actor)}.json`
  try {
    const result = await reader(path)
    if (!result.success || !result.content) {
      return unavailableStatefulPersona(actor, result.error || 'Persona projection unavailable.')
    }
    return parseStatefulPersonaRecord(result.content, actor)
  } catch (error) {
    return unavailableStatefulPersona(
      actor,
      error instanceof Error ? error.message : 'Persona projection unavailable.',
    )
  }
}
