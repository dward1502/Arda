// sigil: REPAIR
import { useEffect, useState } from 'react'
import ModuleCard from '../ModuleCard'
import LineList from '../primitives/LineList'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { envEndpointUrl, envString } from '../../../lib/endpointConfig'
import {
  describeHermesRuntimeLaunch,
  ensureHermesRuntimeSpots,
  readHermesRuntimeHealth,
  type HermesRuntimeHealthResult,
} from '../../../lib/hermesDashboardLauncher'
import { DataFreshnessBadge } from './DataFreshnessBadge'
import { SourceRefreshAffordance } from './SourceRefreshAffordance'

interface HermesToolCapability {
  tool: string
  lane: string
  state: string
  readiness: string
  nextAction: string
}

interface HermesRuntimeSurface {
  tool: string
  status: string
  detail: string
  ok: string
}

interface HermesDashboardModuleProps {
  summary: Array<{ label: string; value: string }>
  tools: HermesToolCapability[]
  runtimeSurfaces: HermesRuntimeSurface[]
  auditReadiness?: Record<string, unknown> | null
  sourceProvenance?: ArdaSourceProvenance[]
  tag?: string
}

const DEFAULT_HERMES_DASHBOARD_HOST = envString(import.meta.env.VITE_ARDA_HERMES_DASHBOARD_HOST, '127.0.0.1')
const DEFAULT_HERMES_DASHBOARD_PORT = envString(import.meta.env.VITE_ARDA_HERMES_DASHBOARD_PORT, '9119')
const DEFAULT_HERMES_DASHBOARD_URL = envEndpointUrl({ host: DEFAULT_HERMES_DASHBOARD_HOST, port: DEFAULT_HERMES_DASHBOARD_PORT })

const HERMES_DASHBOARD_PRINCIPLES = [
  { label: 'Mode', value: 'live embedded harness' },
  { label: 'Surface', value: 'ARDA scene floating workstation' },
  { label: 'Runtime', value: 'single local Hermes dashboard process' },
]

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : []
}

function stringValue(value: unknown, fallback = 'unknown'): string {
  return typeof value === 'string' && value.length > 0 ? value : fallback
}

function numberValue(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : 0
}

export default function HermesDashboardModule({
  summary,
  tools,
  runtimeSurfaces,
  auditReadiness,
  sourceProvenance,
  tag,
}: HermesDashboardModuleProps) {
  const [launchState, setLaunchState] = useState<'launching' | 'ready' | 'error'>('launching')
  const [launchMessage, setLaunchMessage] = useState('Starting Hermes dashboard for embedded scene surface…')
  const [dashboardUrl, setDashboardUrl] = useState<string | null>(null)
  const [dashboardStatus, setDashboardStatus] = useState<HermesRuntimeHealthResult | null>(null)
  const [iframeKey, setIframeKey] = useState(0)

  useEffect(() => {
    let cancelled = false
    setLaunchState('launching')
    setDashboardUrl(null)
    setLaunchMessage(`Starting or attaching to configured Hermes dashboard at ${DEFAULT_HERMES_DASHBOARD_URL}…`)

    const refreshStatus = () => readHermesRuntimeHealth()
      .then((status) => {
        if (!cancelled) {
          setDashboardStatus(status)
        }
        return status
      })

    refreshStatus()
      .catch(() => null)

    ensureHermesRuntimeSpots()
      .then((result) => {
        if (cancelled) {
          return
        }
        setLaunchMessage(
          describeHermesRuntimeLaunch({
            ready: result.ready,
            launched: result.launched,
            url: result.url,
            port: result.port,
            identity: result.identity,
            spotCount: result.spotCount,
            failure: result.failure,
          }),
        )
        void refreshStatus()
        window.setTimeout(() => {
          if (cancelled) {
            return
          }
          setDashboardUrl(result.url)
          setLaunchState('ready')
        }, 350)
      })
      .catch((error) => {
        if (cancelled) {
          return
        }
        setLaunchState('error')
        setLaunchMessage(error instanceof Error ? error.message : String(error))
        void refreshStatus().catch(() => null)
      })

    return () => {
      cancelled = true
    }
  }, [])

  const visibleTools = tools.length > 0 ? tools : [
    {
      tool: 'Hermes Agent',
      lane: 'operator_tools',
      state: 'planned',
      readiness: 'DESIGN INTENT — dashboard shell ready',
      nextAction: 'replace with live Hermes capability registry',
    },
  ]
  const visibleSurfaces = runtimeSurfaces.length > 0 ? runtimeSurfaces : [
    {
      tool: 'hermes-agent',
      status: 'planned',
      detail: 'DESIGN INTENT — native dashboard shell is available; live status adapter is next',
      ok: 'check',
    },
  ]
  const phase7Audit = asRecord(auditReadiness?.phase7_closeout)
  const phase8Audit = asRecord(auditReadiness?.phase8_hardening)
  const auditBoundary = asRecord(auditReadiness?.boundary)
  const auditEvidenceSources = asArray(auditReadiness?.evidence_sources).map((item) => stringValue(item, '')).filter(Boolean)
  const auditItems = asArray(auditReadiness?.next_items)
    .map((item) => asRecord(item))
    .filter((item): item is Record<string, unknown> => item !== null)
  const auditSummary = [
    { label: 'Phase 7 remediation', value: stringValue(phase7Audit?.status) },
    { label: 'Verified slices', value: `${numberValue(phase7Audit?.verified_slices)}/${numberValue(phase7Audit?.total_slices)}` },
    { label: 'Phase 8 queued', value: String(numberValue(phase8Audit?.queued)) },
    { label: 'Phase 8 in progress', value: String(numberValue(phase8Audit?.in_progress)) },
    { label: 'Evidence boundary', value: stringValue(auditBoundary?.summary, 'audit-ledger evidence, not live runtime/service status') },
  ]
  const hermesProvenance = (sourceProvenance ?? []).filter((record) => {
    const haystack = [record.domainId, record.label, ...record.sourcePaths].join(' ').toLowerCase()
    return haystack.includes('hermes') || haystack.includes('dispatch') || haystack.includes('gateway')
  })
  const statusUrl = dashboardStatus?.health.url ?? dashboardUrl ?? DEFAULT_HERMES_DASHBOARD_URL
  const runtimeReady = dashboardStatus?.health.runtimeReady ?? false
  const runtimeLaunched = dashboardStatus?.health.runtimeLaunched ?? false
  const runtimeAvailable = dashboardStatus?.health.runtimeAvailable ?? false
  const processState = runtimeLaunched
    ? 'owned process'
    : runtimeReady
      ? 'verified listener'
      : runtimeAvailable
        ? 'pending'
        : 'not listening'

  return (
    <ModuleCard title="Hermes Dashboard" eyebrow="Real embedded Hermes surface" accent="mint" tag={tag}>
      <div className="hermes-dashboard-embed">
        <div className="hermes-dashboard-embed__status" data-state={launchState}>
          <div>
            <strong>{launchState === 'ready' ? 'Live Hermes dashboard embedded' : launchState === 'launching' ? 'Preparing Hermes dashboard' : 'Hermes dashboard unavailable'}</strong>
            <p>{launchMessage}</p>
            <div className="hermes-dashboard-embed__chips" aria-label="Hermes dashboard status">
              <span>{launchState}</span>
              <span>{processState}</span>
              <span>{statusUrl}</span>
            </div>
          </div>
          <div className="hermes-dashboard-embed__actions">
            <button
              type="button"
              className="action-button"
              onClick={() => {
                setDashboardUrl(null)
                window.setTimeout(() => {
                  setDashboardUrl(statusUrl)
                  setIframeKey((key) => key + 1)
                }, 0)
              }}
            >
              Attach In ARDA
            </button>
            <button type="button" className="action-button action-button--primary" onClick={() => setIframeKey((key) => key + 1)}>
              Reload Surface
            </button>
          </div>
        </div>

        <iframe
          key={iframeKey}
          className="hermes-dashboard-embed__frame"
          title="Hermes Agent Dashboard"
          src={dashboardUrl ?? 'about:blank'}
          loading="eager"
          referrerPolicy="no-referrer"
          onError={() => {
            setLaunchState('error')
            setLaunchMessage(`Embedded Hermes dashboard frame failed to load ${dashboardUrl ?? DEFAULT_HERMES_DASHBOARD_URL}`)
          }}
        />

        {launchState !== 'ready' ? (
          <div className="hermes-dashboard-embed__overlay">
            <strong>{launchState === 'launching' ? 'Booting embedded Hermes dashboard…' : 'Could not load Hermes dashboard'}</strong>
            <p>{launchMessage}</p>
          </div>
        ) : null}
      </div>

      <details className="hermes-dashboard-embed__details">
        <summary>Harness contract and fallback diagnostics</summary>
        <div className="split-stack" style={{ marginTop: 16 }}>
          <div>
            <div className="module-subtitle">Capability Readiness</div>
            <LineList items={(summary.length > 0 ? summary : HERMES_DASHBOARD_PRINCIPLES).map((item) => ({
              label: item.label,
              value: item.value,
            }))} />
          </div>
          <div>
            <div className="module-subtitle">Integration Contract</div>
            <LineList items={HERMES_DASHBOARD_PRINCIPLES} />
          </div>
        </div>

        {hermesProvenance.length > 0 ? (
          <div style={{ marginTop: 16 }}>
            <div className="module-subtitle">Hermes Source Freshness</div>
            <div className="source-provenance-list">
              {hermesProvenance.slice(0, 4).map((record) => (
                <article className="source-provenance-list__item" key={record.domainId}>
                  <div>
                    <strong>{record.label}</strong>
                    <span>{record.sourceKind} / {record.generatedAtUtc ?? record.observedAtUtc ?? 'timestamp unknown'}</span>
                  </div>
                  <span className="source-provenance-list__actions">
                    <SourceRefreshAffordance record={record} compact />
                    <DataFreshnessBadge record={record} compact />
                  </span>
                </article>
              ))}
            </div>
          </div>
        ) : null}

        <div className="split-stack" style={{ marginTop: 16 }}>
          <div>
            <div className="module-subtitle">Hermes Tool Lanes — live data or design intent</div>
            <div className="document-list compact">
              {visibleTools.slice(0, 6).map((tool) => (
                <article className="document-list__item" key={`${tool.tool}-${tool.lane}`}>
                  <strong>{tool.tool}</strong>
                  <span>{tool.lane} / {tool.state}</span>
                  <p>{tool.readiness} / {tool.nextAction}</p>
                </article>
              ))}
            </div>
          </div>
          <div>
            <div className="module-subtitle">Runtime Surfaces — live data or design intent</div>
            <div className="document-list compact">
              {visibleSurfaces.slice(0, 6).map((surface) => (
                <article className="document-list__item" key={`${surface.tool}-${surface.status}`}>
                  <strong>{surface.tool}</strong>
                  <span>{surface.status} / {surface.ok}</span>
                  <p>{surface.detail}</p>
                </article>
              ))}
            </div>
          </div>
        </div>

        <div className="split-stack" style={{ marginTop: 16 }}>
          <div>
            <div className="module-subtitle">Audit Readiness Projection</div>
            <LineList items={auditSummary} />
            <p className="systems-panel__note">
              {stringValue(auditBoundary?.roadmap, 'Phase 8 hardening and future embodied roadmap work remain separate from closed Phase 7 remediation.')}
            </p>
          </div>
          <div>
            <div className="module-subtitle">Next Hardening Items</div>
            <div className="document-list compact">
              {(auditItems.length > 0 ? auditItems : [{
                id: 'audit-readiness-clear',
                title: 'No queued Phase 8 hardening item exposed',
                status: 'complete',
                scope: 'audit readiness projection',
              }]).slice(0, 4).map((item) => (
                <article className="document-list__item" key={stringValue(item.id, stringValue(item.title))}>
                  <strong>{stringValue(item.id)} · {stringValue(item.title, 'Untitled hardening item')}</strong>
                  <span>{stringValue(item.status)}</span>
                  <p>{stringValue(item.scope, 'scope unavailable')}</p>
                </article>
              ))}
            </div>
            {auditEvidenceSources.length > 0 ? (
              <p className="systems-panel__note">Evidence: {auditEvidenceSources.join(' · ')}</p>
            ) : null}
          </div>
        </div>
      </details>
    </ModuleCard>
  )
}
