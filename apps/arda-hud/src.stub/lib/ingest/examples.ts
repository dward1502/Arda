import { ingest, loadFromFile, loadFromUrl, COMPONENT_REGISTRY, type IngestRequest, type ParsedData } from './index'

const EXAMPLES = {
  jsonMetrics: JSON.stringify({
    tasks_total: 42,
    tasks_completed: 28,
    tasks_pending: 14,
    uptime_hours: 1250,
    active_agents: 7,
  }),
  
  jsonKanban: JSON.stringify([
    { id: '1', title: 'Design UI', status: 'todo', priority: 'high', tags: ['ux'] },
    { id: '2', title: 'Write docs', status: 'in-progress', priority: 'medium', tags: ['docs'] },
    { id: '3', title: 'Fix bug', status: 'done', priority: 'low', tags: ['bug'] },
    { id: '4', title: 'Deploy', status: 'todo', priority: 'high', tags: ['ops'] },
  ]),
  
  csvData: `name,role,status,bugs
Athena,research,active,12
Hermes,communications,active,5
Plutus,economics,active,8
Oracle,reasoning,active,2`,
  
  markdownDoc: `# Project Overview

## Goals
Build an awesome agent system.

## Status
- Phase 1: Complete
- Phase 2: In Progress
- Phase 3: Planned

### Notes
This is a **bold** statement.`,
}

export async function runExamples() {
  console.log('=== Ingestion Examples ===\n')
  
  console.log('1. JSON Metrics → MetricPill:')
  const metrics = await ingest({
    source: EXAMPLES.jsonMetrics,
    format: 'json',
    componentType: 'metric',
  })
  console.log(JSON.stringify(metrics.componentPayload, null, 2))
  console.log()
  
  console.log('2. JSON Kanban → Kanban:')
  const kanban = await ingest({
    source: EXAMPLES.jsonKanban,
    format: 'json',
    componentType: 'kanban',
  })
  console.log(JSON.stringify(kanban.componentPayload, null, 2))
  console.log()
  
  console.log('3. CSV → Grid:')
  const grid = await ingest({
    source: EXAMPLES.csvData,
    format: 'csv',
    componentType: 'grid',
  })
  console.log(JSON.stringify(grid.componentPayload, null, 2))
  console.log()
  
  console.log('4. Markdown → Document:')
  const doc = await ingest({
    source: EXAMPLES.markdownDoc,
    format: 'markdown',
    componentType: 'document',
  })
  console.log(JSON.stringify(doc.componentPayload, null, 2))
  console.log()
  
  console.log('5. Auto-detect format:')
  const auto = await ingest({
    source: EXAMPLES.jsonKanban,
    format: 'auto',
    componentType: 'auto',
  })
  console.log(`Detected: ${auto.format} → ${auto.componentPayload.type}`)
  console.log()
  
  console.log('=== Available Components ===')
  COMPONENT_REGISTRY.forEach(c => {
    console.log(`- ${c.type}: ${c.label} (${c.description})`)
  })
}

export { EXAMPLES }
