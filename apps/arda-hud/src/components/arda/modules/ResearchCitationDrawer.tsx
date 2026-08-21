import { useState } from 'react'
import type { ResearchBriefCitation } from '../../../lib/research'

interface ResearchCitationDrawerProps {
  citations: ResearchBriefCitation[]
}

export default function ResearchCitationDrawer({ citations }: ResearchCitationDrawerProps) {
  const [openId, setOpenId] = useState<string | null>(null)
  return (
    <section className="research-citations" aria-labelledby="research-citations-title">
      <header className="research-section-heading"><h3 id="research-citations-title">Citation provenance</h3><span>{citations.length} sources</span></header>
      {citations.length === 0 ? <p className="research-muted">No fetched citations are attached to this brief.</p> : <ul className="research-citation-list">
        {citations.map((citation) => {
          const state = citation.failure_reason ? 'failed' : citation.rejection_reason ? 'rejected' : citation.fetch_status ?? 'fetched'
          const canonicalUrl = citation.canonical_url ?? citation.url
          const sourceIdentity = citation.normalized_source_id ?? citation.source_identity
          return <li key={citation.citation_id}>
            <button type="button" className="research-citation-toggle" aria-expanded={openId === citation.citation_id} onClick={() => setOpenId(openId === citation.citation_id ? null : citation.citation_id)}>
              <span><strong>{citation.title || sourceIdentity || canonicalUrl || citation.citation_id}</strong><small>{state} · {citation.policy_readiness ?? citation.quality ?? 'quality undisclosed'} · {citation.freshness_status ?? citation.freshness ?? 'freshness undisclosed'}</small></span>
              <span aria-hidden="true">{openId === citation.citation_id ? '−' : '+'}</span>
            </button>
            {openId === citation.citation_id ? <div className="research-citation-detail">
              {canonicalUrl ? <a href={canonicalUrl} target="_blank" rel="noreferrer">Open canonical source</a> : <p role="status">Canonical source URL was not recorded.</p>}
              {citation.excerpt ? <blockquote>{citation.excerpt}</blockquote> : null}
              {citation.rejection_reason ? <p role="status"><strong>Rejected:</strong> {citation.rejection_reason}</p> : null}
              {citation.failure_reason ? <p role="alert"><strong>Failed:</strong> {citation.failure_reason}</p> : null}
            </div> : null}
          </li>
        })}
      </ul>}
    </section>
  )
}
