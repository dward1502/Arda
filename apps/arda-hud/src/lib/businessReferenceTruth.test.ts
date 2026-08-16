import { describe, expect, it } from 'vitest'
import { reconcileBusinessRuntimeReferences } from './businessReferenceTruth'

describe('business reference truth', () => {
  it('overrides stale snapshot exists flags using the live derived client inventory', () => {
    const runtime = reconcileBusinessRuntimeReferences(
      {
        client_records: [
          { path: 'data/business/clients/live/control.json', exists: false },
          { path: 'data/business/clients/gone/control.json', exists: true },
        ],
      },
      { highlights: { client_paths: ['data/business/clients/live/control.json'] } },
    )

    expect(runtime.client_records).toEqual([
      expect.objectContaining({ path: 'data/business/clients/live/control.json', exists: true }),
      expect.objectContaining({ path: 'data/business/clients/gone/control.json', exists: false }),
    ])
  })

  it('reconciles referenced project paths inside company operations', () => {
    const runtime = reconcileBusinessRuntimeReferences({
      company_ops: {
        projects: [
          { project_id: 'live', path: 'data/projects/live/project.json', exists: false },
          { project_id: 'gone', path: 'data/projects/gone/project.json', exists: true },
        ],
      },
    }, {
      highlights: { project_paths: ['data/projects/live/project.json'] },
    })

    expect((runtime.company_ops as { projects: unknown[] }).projects).toEqual([
      expect.objectContaining({ project_id: 'live', exists: true }),
      expect.objectContaining({ project_id: 'gone', exists: false }),
    ])
  })
})