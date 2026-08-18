import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import {
  MIRROMERE_APP_VIEW_IDS,
  MIRROMERE_ALLOWED_INTERACTIONS,
  MIRROMERE_DISPLAY_ROLES,
  MIRROMERE_MAX_SLOTS,
  MIRROMERE_MIME_TYPES,
  MIRROMERE_PRIVACY_CLASSES,
  MIRROMERE_SCENE_IDS,
  MIRROMERE_SOURCE_MODES,
  MIRROMERE_SURFACE_SCHEMA_VERSION,
  parseMirromereSurface,
} from './types'
import {
  ambientIdleFixture,
  continuityHandoffReadyFixture,
  systemDegradedFixture,
} from './fixtures'

function canonicalSchema(): Record<string, any> {
  return JSON.parse(readFileSync(
    resolve(process.cwd(), '../../spec/mirromere-surface/v1/mirromere-surface.schema.json'),
    'utf8',
  ))
}

describe('Mirromere surface contract', () => {
  const fixtureNow = new Date('2026-08-17T12:01:00Z')

  it('parses the three representative scenes and keeps fixture mode explicit', () => {
    for (const fixture of [ambientIdleFixture, systemDegradedFixture, continuityHandoffReadyFixture]) {
      const parsed = parseMirromereSurface(fixture, fixtureNow)
      expect(parsed.schema_version).toBe(MIRROMERE_SURFACE_SCHEMA_VERSION)
      expect(parsed.source_mode).toBe('fixture')
      expect(parsed.evidence.length).toBeGreaterThan(0)
    }
  })

  it('rejects unknown fields recursively', () => {
    expect(() => parseMirromereSurface({ ...ambientIdleFixture, unexpected: true }, fixtureNow)).toThrow(/unknown/i)
    expect(() => parseMirromereSurface({
      ...ambientIdleFixture,
      slots: [{ ...ambientIdleFixture.slots[0], unexpected: true }],
    }, fixtureNow)).toThrow(/unknown/i)
  })

  it('fails closed for expiry, privacy escalation, unknown interaction, and missing evidence', () => {
    expect(() => parseMirromereSurface(ambientIdleFixture, new Date('2026-08-17T12:06:00Z'))).toThrow(/expired/i)
    expect(() => parseMirromereSurface({
      ...ambientIdleFixture,
      privacy: { privacy_class: 'operator_private', visibility_ceiling: 'public_ambient' },
    }, fixtureNow)).toThrow(/privacy/i)
    expect(() => parseMirromereSurface({
      ...ambientIdleFixture,
      allowed_interactions: ['launch_shell'],
    }, fixtureNow)).toThrow(/interaction/i)
    expect(() => parseMirromereSurface({ ...ambientIdleFixture, evidence: [] }, fixtureNow)).toThrow(/evidence/i)
  })

  it('rejects URL, HTML, shell-shaped, and oversized content', () => {
    expect(() => parseMirromereSurface({
      ...ambientIdleFixture,
      slots: [{
        ...ambientIdleFixture.slots[0],
        content: {
          kind: 'media_ref', asset_id: 'hero', digest: `sha256:${'a'.repeat(64)}`,
          mime_type: 'image/png', url: 'https://example.invalid/raw.png',
        },
      }],
    }, fixtureNow)).toThrow()
    expect(() => parseMirromereSurface({
      ...ambientIdleFixture,
      slots: [{ ...ambientIdleFixture.slots[0], content: { kind: 'text', text: '<script>alert(1)</script>' } }],
    }, fixtureNow)).toThrow(/unsafe/i)
    expect(() => parseMirromereSurface({
      ...ambientIdleFixture,
      slots: [{ ...ambientIdleFixture.slots[0], content: { kind: 'app_view', view_id: 'terminal', command: 'rm -rf /' } }],
    }, fixtureNow)).toThrow()
    expect(() => parseMirromereSurface({
      ...ambientIdleFixture,
      accessibility: { ...ambientIdleFixture.accessibility, description: '<b>trusted</b>' },
    }, fixtureNow)).toThrow(/unsafe/i)
    expect(() => parseMirromereSurface({
      ...ambientIdleFixture,
      slots: Array.from({ length: MIRROMERE_MAX_SLOTS + 1 }, (_, index) => ({
        ...ambientIdleFixture.slots[0], id: `slot-${index}`,
      })),
    }, fixtureNow)).toThrow(/slots/i)
  })

  it('stays aligned with canonical schema enums and bounds', () => {
    const schema = canonicalSchema()
    expect(schema.properties.schema_version.const).toBe(MIRROMERE_SURFACE_SCHEMA_VERSION)
    expect(schema.properties.slots.maxItems).toBe(MIRROMERE_MAX_SLOTS)
    expect(schema.$defs.sceneId.enum).toEqual(MIRROMERE_SCENE_IDS)
    expect(schema.$defs.interactionId.enum).toEqual(MIRROMERE_ALLOWED_INTERACTIONS)
    expect(schema.properties.display_role.enum).toEqual(MIRROMERE_DISPLAY_ROLES)
    expect(schema.properties.source_mode.enum).toEqual(MIRROMERE_SOURCE_MODES)
    expect(schema.$defs.privacy.properties.privacy_class.enum).toEqual(MIRROMERE_PRIVACY_CLASSES)
    expect(schema.$defs.mediaContent.properties.mime_type.enum).toEqual(MIRROMERE_MIME_TYPES)
    expect(schema.$defs.appViewContent.properties.view_id.enum).toEqual(MIRROMERE_APP_VIEW_IDS)
    expect(schema.additionalProperties).toBe(false)
  })
})
