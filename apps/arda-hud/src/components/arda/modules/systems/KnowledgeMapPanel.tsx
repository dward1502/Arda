// sigil: REPAIR
import { useState } from 'react'
import LineList from '../../primitives/LineList'
import KnowledgeActionContractPanel from './KnowledgeActionContractPanel'
import type { SystemActionCapabilityStatus, SystemActionDescriptor, SystemActionId } from '../../../../lib/systemActionBus'

interface KnowledgeMapEntry {
  path: string
  title: string
  classification: string
  canonicalHome: string
  authority: string
  domain: string
  glyph: string
}

interface KnowledgeDigestEntry {
  sourceId: string
  title: string
  status: string
  sourceType: string
  tags: string[]
  summary: string
}

interface KnowledgeDeepGraphEntry {
  sourceId: string
  confidence: string
  triadPassed: boolean
  nodeCount: number
  edgeCount: number
  labels: string[]
}

interface KnowledgePolicyReadinessEntry {
  sourceId: string
  readiness: string
  confidence: string
  blockers: string[]
  triadPassed: boolean
}

interface KnowledgePolicySummary {
  status: string
  policyReadyTotal: number
  referenceOnlyTotal: number
  reviewPressureTotal: number
  nextOperatorAction: string
  promotionPreviewAvailable: boolean
  governanceGate: string
}

interface MissingProjectionEntry {
  sectionId: string
  sectionTitle: string
  owner: string
  missing: string[]
}

interface KnowledgeMapPanelProps {
  summary: Array<{ label: string; value: string }>
  entries: KnowledgeMapEntry[]
  digest: KnowledgeDigestEntry[]
  deepGraph: KnowledgeDeepGraphEntry[]
  policyReadiness: KnowledgePolicyReadinessEntry[]
  policySummary: KnowledgePolicySummary
  missingProjections: MissingProjectionEntry[]
  actionDescriptors?: SystemActionDescriptor[]
  capabilityStatuses?: SystemActionCapabilityStatus[]
  busyActionId?: SystemActionId | null
  actionMessage?: string | null
  onRunAction?: (actionId: SystemActionId) => void
}

type KnowledgeViewId = 'registry' | 'digest' | 'deepGraph' | 'policyReadiness' | 'missingProjections'

const EMPTY_VIEW_COPY: Record<KnowledgeViewId, { title: string; body: string }> = {
  registry: {
    title: 'No registry entries loaded',
    body: 'Expected `core/state/knowledge_triage_registry.jsonl`.',
  },
  digest: {
    title: 'No digest packets loaded',
    body: 'Expected ATHENA digest JSONL from the configured HUD settings path.',
  },
  deepGraph: {
    title: 'No deep graph packets loaded',
    body: 'Expected ATHENA deep graph JSONL from the configured HUD settings path.',
  },
  policyReadiness: {
    title: 'No policy readiness packets loaded',
    body: 'Expected ATHENA policy readiness JSONL from the configured HUD settings path.',
  },
  missingProjections: {
    title: 'No missing source-map projections',
    body: 'All loaded ARDA source-map sections expose their declared HUD projections.',
  },
}

function formatCount(label: string, count: number): string {
  return `${label} ${count}`
}

function EmptyView({ view }: { view: KnowledgeViewId }) {
  const copy = EMPTY_VIEW_COPY[view]
  return (
    <article className="document-list__item">
      <strong>{copy.title}</strong>
      <p>{copy.body}</p>
    </article>
  )
}

export default function KnowledgeMapPanel({
  summary,
  entries,
  digest,
  deepGraph,
  policyReadiness,
  policySummary,
  missingProjections,
  actionDescriptors = [],
  capabilityStatuses = [],
  busyActionId,
  actionMessage,
  onRunAction,
}: KnowledgeMapPanelProps) {
  const [activeView, setActiveView] = useState<KnowledgeViewId>('registry')
  const views: Array<{ id: KnowledgeViewId; label: string; count: number }> = [
    { id: 'registry', label: 'Registry', count: entries.length },
    { id: 'digest', label: 'Digest', count: digest.length },
    { id: 'deepGraph', label: 'Deep Graph', count: deepGraph.length },
    { id: 'policyReadiness', label: 'Policy Readiness', count: policyReadiness.length },
    { id: 'missingProjections', label: 'Missing Projections', count: missingProjections.length },
  ]
  const activeLabel = views.find((view) => view.id === activeView)?.label ?? 'Registry'

  return (
    <section aria-labelledby="knowledge-map-heading">
      <h3 className="module-subtitle" id="knowledge-map-heading">Knowledge Map</h3>
      <LineList items={summary} />
      <KnowledgeActionContractPanel
        actionDescriptors={actionDescriptors}
        capabilityStatuses={capabilityStatuses}
        policySummary={policySummary}
        busyActionId={busyActionId}
        message={actionMessage}
        onRunAction={onRunAction}
      />
      <div aria-label="Knowledge map views" className="systems-chip-cloud" role="tablist" style={{ marginTop: 12 }}>
        {views.map((view) => (
          <button
            aria-controls={`knowledge-map-${view.id}`}
            aria-selected={activeView === view.id}
            className={`systems-chip ${activeView === view.id ? 'systems-chip--accent' : 'systems-chip--idle'}`}
            id={`knowledge-map-tab-${view.id}`}
            key={view.id}
            onClick={() => setActiveView(view.id)}
            role="tab"
            type="button"
          >
            {formatCount(view.label, view.count)}
          </button>
        ))}
      </div>
      <div
        aria-labelledby={`knowledge-map-tab-${activeView}`}
        className="document-list compact"
        id={`knowledge-map-${activeView}`}
        role="tabpanel"
        style={{ marginTop: 12 }}
      >
        {activeView === 'registry' && (entries.length > 0 ? entries.map((entry) => (
          <article className="document-list__item" key={`${entry.classification}-${entry.path}`}>
            <strong>{entry.glyph} {entry.title}</strong>
            <span>{entry.classification} / {entry.authority}</span>
            <p>{entry.canonicalHome} / {entry.domain} / {entry.path}</p>
          </article>
        )) : <EmptyView view="registry" />)}

        {activeView === 'digest' && (digest.length > 0 ? digest.map((entry) => (
          <article className="document-list__item" key={entry.sourceId}>
            <strong>{entry.title}</strong>
            <span>{entry.status} / {entry.sourceType} / {entry.sourceId}</span>
            <p>{entry.summary}</p>
            <p>{entry.tags.length > 0 ? entry.tags.join(', ') : 'no tags'}</p>
          </article>
        )) : <EmptyView view="digest" />)}

        {activeView === 'deepGraph' && (deepGraph.length > 0 ? deepGraph.map((entry) => (
          <article className="document-list__item" key={entry.sourceId}>
            <strong>{entry.sourceId}</strong>
            <span>{entry.triadPassed ? 'triad passed' : 'triad gap'} / confidence {entry.confidence}</span>
            <p>nodes {entry.nodeCount} / edges {entry.edgeCount}</p>
            <p>{entry.labels.length > 0 ? entry.labels.join(', ') : 'no graph labels'}</p>
          </article>
        )) : <EmptyView view="deepGraph" />)}

        {activeView === 'policyReadiness' && (policyReadiness.length > 0 ? policyReadiness.map((entry) => (
          <article className="document-list__item" key={entry.sourceId}>
            <strong>{entry.sourceId}</strong>
            <span>{entry.readiness} / confidence {entry.confidence}</span>
            <p>{entry.triadPassed ? 'triad passed' : 'triad gap'}</p>
            <p>{entry.blockers.length > 0 ? `blockers: ${entry.blockers.join(', ')}` : 'no blockers'}</p>
          </article>
        )) : <EmptyView view="policyReadiness" />)}

        {activeView === 'missingProjections' && (missingProjections.length > 0 ? missingProjections.map((entry) => (
          <article className="document-list__item" key={entry.sectionId}>
            <strong>{entry.sectionTitle}</strong>
            <span>{entry.owner} / {entry.sectionId}</span>
            <p>missing projections: {entry.missing.join(', ')}</p>
          </article>
        )) : <EmptyView view="missingProjections" />)}
      </div>
    </section>
  )
}
