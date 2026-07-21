// sigil: REPAIR
import type { JsonRecord } from './ardaSource'

type ProtocolMarkerKey = 'REVIEW' | 'JOULE_WORK' | 'TRIAD_GATE' | string

function asRecord(value: unknown): JsonRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as JsonRecord
}

function getString(value: unknown, fallback = ''): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback
}

const EMPTY_CONTRACT: JsonRecord = {}

function contractOrBundled(contract: JsonRecord | null): JsonRecord {
  return contract ?? EMPTY_CONTRACT
}

function glyphFromGroup(contract: JsonRecord, group: string, key: string): string {
  const glyphGroups = asRecord(contract.glyph_groups)
  const groupRecord = asRecord(glyphGroups?.[group])
  const entry = asRecord(groupRecord?.[key.toUpperCase()])
  return getString(entry?.glyph, '')
}

function signatureFromSourceDefault(sourceDefault: JsonRecord | null): string {
  const glyphs = asRecord(sourceDefault?.glyphs)
  return getString(glyphs?.signature, '')
}

export function glyphSignatureToArray(signature: string | null | undefined): string[] {
  return signature ? Array.from(signature) : []
}

export function protocolMarkerFromContract(
  contract: JsonRecord | null,
  key: ProtocolMarkerKey,
  fallback = '',
): string {
  return glyphFromGroup(contractOrBundled(contract), 'protocol_markers', key) || fallback
}

export function stateSignalFromContract(contract: JsonRecord | null, key: string, fallback = ''): string {
  return glyphFromGroup(contractOrBundled(contract), 'state_signals', key) || fallback
}

export function fallbackProtocolMarker(key: ProtocolMarkerKey): string {
  return protocolMarkerFromContract(null, key)
}

export function primarySigilForSource(source: string, fallback = ''): string {
  const sigils = resolveSourceSigilsFromContract(null, source)
  return sigils[0] ?? fallback
}

export function resolveSourceSigilsFromContract(contract: JsonRecord | null, source: string): string[] {
  const effectiveContract = contractOrBundled(contract)
  const sourceDefaults = asRecord(effectiveContract.source_defaults)
  const normalized = source.toLowerCase()
  const direct = asRecord(sourceDefaults?.[normalized])
  const directSignature = signatureFromSourceDefault(direct)
  if (directSignature) return glyphSignatureToArray(directSignature)

  for (const [key, value] of Object.entries(sourceDefaults ?? {})) {
    if (!normalized.includes(key.toLowerCase())) continue

    const signature = signatureFromSourceDefault(asRecord(value))
    if (signature) return glyphSignatureToArray(signature)
  }

  return []
}

export function resolveAgentSigilFromContract(contract: JsonRecord | null, agent: JsonRecord): string {
  const explicitSigil = getString(agent.sigil, '')
  if (explicitSigil) return explicitSigil

  const effectiveContract = contractOrBundled(contract)
  const candidateKeys = [
    getString(agent.sigil_code, '').toUpperCase(),
    getString(agent.name, '').toUpperCase(),
    getString(agent.realm, '').toUpperCase(),
    getString(agent.owner, '').toUpperCase(),
  ].filter(Boolean)

  for (const key of candidateKeys) {
    const glyph = glyphFromGroup(effectiveContract, 'agent_identity', key)
    if (glyph) return glyph
  }

  return glyphFromGroup(effectiveContract, 'state_signals', 'IDLE') || '*'
}
