// sigil: REPAIR
import { ExternalLink, PlugZap, ShieldCheck } from 'lucide-react'
import ModuleCard from '../ModuleCard'
import LineList from '../primitives/LineList'
import { getSurfaceAdapterFocusContract, type SurfaceAdapterManifest } from '../../../lib/surfaceAdapterManifests'

interface ServiceEmbedModuleProps {
  manifest: SurfaceAdapterManifest | null
}

export default function ServiceEmbedModule({ manifest }: ServiceEmbedModuleProps) {
  if (!manifest) {
    return (
      <ModuleCard title="Service Surface" eyebrow="No Adapter" accent="violet">
        <div className="empty-state">No service adapter manifest is bound to this workstation.</div>
      </ModuleCard>
    )
  }
  const focusContract = getSurfaceAdapterFocusContract(manifest.sourceZoneId)

  return (
    <ModuleCard
      title={manifest.title}
      eyebrow={`${manifest.provider} / ${manifest.kind.replace(/_/g, ' ')}`}
      accent="violet"
      tag={manifest.trust.replace(/_/g, ' ')}
      actions={manifest.externalUrl ? (
        <a className="module-card__action" href={manifest.externalUrl} target="_blank" rel="noreferrer">
          <ExternalLink size={14} />
          Open
        </a>
      ) : null}
    >
      <div className="service-surface-hero">
        <div>
          <span>{manifest.provider}</span>
          <strong>{manifest.title}</strong>
          <p>{manifest.summary}</p>
        </div>
        <div className="service-surface-hero__status">
          <ShieldCheck size={18} />
          <span>{manifest.trust.replace(/_/g, ' ')}</span>
        </div>
      </div>
      <div className="split-stack" style={{ marginTop: 16 }}>
        <div>
          <div className="module-subtitle"><PlugZap size={14} /> Surface Contract</div>
          <LineList
            items={[
              { label: 'Source Zone', value: manifest.sourceZoneId },
              { label: 'Freshness', value: manifest.freshnessSource.replace(/_/g, ' ') },
              { label: 'Embed', value: manifest.allowInlineEmbed ? 'inline allowed' : 'external window' },
              { label: 'Focus', value: focusContract?.focusMode.replace(/_/g, ' ') ?? manifest.preferredFocusMode.replace(/_/g, ' ') },
              { label: 'Target', value: focusContract?.target ?? 'adapter pending' },
            ]}
          />
        </div>
        <div>
          <div className="module-subtitle">Preview Signal</div>
          <LineList items={manifest.previewRows} />
        </div>
      </div>
      <div className="service-surface-capabilities">
        {manifest.capabilities.map((capability) => (
          <span key={capability}>{capability}</span>
        ))}
      </div>
      <div className="service-surface-readiness">
        {manifest.readiness.map((item) => (
          <article className={`service-surface-readiness__item service-surface-readiness__item--${item.status}`} key={item.label}>
            <strong>{item.label}</strong>
            <span>{item.status}</span>
            <p>{item.detail}</p>
          </article>
        ))}
      </div>
      {manifest.allowInlineEmbed && manifest.embedUrl ? (
        <iframe className="service-embed-frame" src={manifest.embedUrl} title={manifest.title} />
      ) : (
        <div className="service-embed-placeholder">
          <strong>{manifest.kind === 'media_adapter' ? 'Focused media surface' : manifest.kind === 'session_adapter' ? 'Focused session surface' : 'External service surface'}</strong>
          <span>{focusContract?.reason ?? 'Inline embedding is disabled for this adapter until the service is explicitly approved for framed display.'}</span>
        </div>
      )}
    </ModuleCard>
  )
}
