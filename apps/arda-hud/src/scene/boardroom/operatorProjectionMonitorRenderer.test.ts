import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import { resolveOperatorProjectionCanvasModel } from './operatorProjectionMonitorRenderer'

const fixture = JSON.parse(readFileSync(
  resolve(process.cwd(), '../../spec/operator-projection/v1/fixtures/valid-operator-projection.json'),
  'utf8',
))

describe('operator projection monitor renderer', () => {
  it('renders canonical IDs and state from one read-only projection', () => {
    const model = resolveOperatorProjectionCanvasModel(fixture)

    expect(model).toMatchObject({
      ok: true,
      projectionId: 'projection-p9-fixture',
      authority: 'read_only',
      freshness: 'fresh',
      title: 'Unify operator projections',
    })
    expect(model.ok && model.rows).toEqual(expect.arrayContaining([
      'OBJECTIVE  objective-p9  ACTIVE',
      'RUN        run-p9  RUNNING',
      'APPROVAL   approval-p9  PENDING',
      'DEPENDENCY arda-harness  READY / FRESH',
      'DEPENDENCY warden-scout  DEGRADED / STALE',
    ]))
  })

  it('fails closed instead of inventing monitor-local truth', () => {
    expect(resolveOperatorProjectionCanvasModel({
      ...fixture,
      schema_version: 'arda.operator-projection.v0',
    })).toEqual({ ok: false, reason: 'operator projection unavailable' })
  })
})
