import { useEffect, useRef } from 'react'
import type { OnboardingSnapshot } from '../lib/tauri-core-compat'

interface OnboardingPanelProps {
  snapshot: OnboardingSnapshot | null
  error: string | null
  onClose: () => void
}

export default function OnboardingPanel({ snapshot, error, onClose }: OnboardingPanelProps) {
  const panelRef = useRef<HTMLElement>(null)
  const readiness = snapshot?.readiness
  const servicePlan = snapshot?.servicePlan
  const actionableChecks = readiness?.checks.filter(check => check.status !== 'pass') ?? []

  useEffect(() => {
    const panel = panelRef.current
    if (!panel) return

    const focusableSelector = 'button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])'
    const focusable = () => Array.from(panel.querySelectorAll<HTMLElement>(focusableSelector))
    focusable()[0]?.focus()

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault()
        onClose()
        return
      }
      if (event.key !== 'Tab') return

      const controls = focusable()
      if (controls.length === 0) return

      event.preventDefault()
      const activeIndex = controls.indexOf(document.activeElement as HTMLElement)
      const nextIndex = event.shiftKey
        ? activeIndex <= 0
          ? controls.length - 1
          : activeIndex - 1
        : activeIndex < 0 || activeIndex === controls.length - 1
          ? 0
          : activeIndex + 1
      controls[nextIndex].focus()
    }

    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [onClose])

  return (
    <section
      ref={panelRef}
      id="operator-onboarding-status"
      role="dialog"
      aria-modal="true"
      aria-label="Operator onboarding status"
      aria-live="polite"
      className="absolute inset-x-4 top-4 z-50 mx-auto max-h-[70vh] max-w-3xl overflow-y-auto rounded border border-[#f4e9d8]/30 bg-black/90 p-5 font-mono text-[#f4e9d8] shadow-2xl backdrop-blur"
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-[10px] uppercase tracking-[0.3em] text-[#f4e9d8]/50">
            Approval-gated first run
          </p>
          <h2 className="mt-1 text-lg">Environment readiness</h2>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="rounded border border-white/20 px-3 py-1 text-xs text-white/70 hover:bg-white/5"
        >
          CLOSE
        </button>
      </div>

      {error && (
        <p role="alert" className="mt-4 rounded border border-red-400/40 bg-red-950/30 p-3 text-sm text-red-200">
          {error}
        </p>
      )}

      {!error && !snapshot && <p className="mt-4 text-sm text-white/60">Loading onboarding contracts…</p>}

      {snapshot && (
        <div className="mt-5 grid gap-4 md:grid-cols-2">
          <article className="rounded border border-white/15 bg-white/[0.03] p-4 md:col-span-2">
            <h3 className="text-sm uppercase tracking-wider">First-run sequence</h3>
            <ol className="mt-3 grid gap-2 text-xs sm:grid-cols-2 lg:grid-cols-3">
              {[
                ['01', 'Compatibility detection'],
                ['02', 'Prerequisite checks'],
                ['03', 'Provider setup'],
                ['04', 'Service plan'],
                ['05', 'Readiness review'],
                ['06', 'Guided setup'],
              ].map(([index, title]) => (
                <li key={index} className="rounded border border-white/10 p-2">
                  <span className="mr-2 text-white/40">{index}</span>{title}
                </li>
              ))}
            </ol>
            <p className="mt-3 text-xs text-white/60">
              Mutation boundary: {snapshot.mutation_policy}. No setup action runs automatically.
            </p>
          </article>

          <article className={`rounded border p-4 ${snapshot.compatibility.status === 'supported' ? 'border-emerald-400/30 bg-emerald-950/10' : 'border-red-400/40 bg-red-950/20'}`}>
            <h3 className="text-sm uppercase tracking-wider">01 · Compatibility</h3>
            <p className="mt-2 text-xs">{snapshot.compatibility.pretty_name} · {snapshot.compatibility.architecture}</p>
            <p className="mt-2 text-xs text-white/70">{snapshot.compatibility.message}</p>
            <p className="mt-2 text-[10px] text-white/45">Required: {snapshot.compatibility.supported_profile}</p>
          </article>

          <article className="rounded border border-white/15 bg-white/[0.03] p-4">
            <h3 className="text-sm uppercase tracking-wider">02 · Prerequisites</h3>
            <dl className="mt-3 grid grid-cols-2 gap-2 text-xs">
              {Object.entries(snapshot.prerequisites.summary).map(([status, count]) => (
                <div key={status} className="contents">
                  <dt className="capitalize text-white/50">{status}</dt><dd>{count}</dd>
                </div>
              ))}
            </dl>
          </article>

          <article className="rounded border border-white/15 bg-white/[0.03] p-4 md:col-span-2">
            <h3 className="text-sm uppercase tracking-wider">03 · Providers</h3>
            {snapshot.providers.providers.length > 0 ? (
              <ul className="mt-3 grid gap-2 sm:grid-cols-2">
                {snapshot.providers.providers.map(provider => (
                  <li key={provider.provider_id} className="rounded border border-white/10 p-3 text-xs">
                    <span>{provider.provider_name}</span>
                    <p className="mt-1 text-white/55">
                      {provider.enabled ? 'enabled' : 'disabled'} · {provider.missing_env.length > 0 ? `missing ${provider.missing_env.join(', ')}` : 'configuration present'}
                    </p>
                  </li>
                ))}
              </ul>
            ) : <p className="mt-2 text-xs text-white/60">No provider configuration is enabled.</p>}
          </article>

          <article className="rounded border border-white/15 bg-white/[0.03] p-4">
            <h3 className="text-sm uppercase tracking-wider">04 · Service plan</h3>
            {servicePlan ? (
              <>
                <p className="mt-2 text-xs text-white/60">
                  {servicePlan.profile} · {servicePlan.machine_role} · {servicePlan.gate_status}
                </p>
                <ul className="mt-3 space-y-3">
                  {servicePlan.actions.map(action => (
                    <li key={action.action_id} className="border-l border-white/20 pl-3 text-xs">
                      <div className="flex flex-wrap items-center gap-2">
                        <span>{action.title}</span>
                        {action.requires_human_gate && (
                          <span className="rounded border border-amber-400/40 px-1.5 py-0.5 text-[9px] uppercase text-amber-200">
                            Human gate
                          </span>
                        )}
                      </div>
                      <p className="mt-1 text-white/50">{action.description}</p>
                    </li>
                  ))}
                </ul>
              </>
            ) : (
              <p className="mt-3 text-xs text-amber-200">Service plan unavailable.</p>
            )}
          </article>

          <article className="rounded border border-white/15 bg-white/[0.03] p-4">
            <h3 className="text-sm uppercase tracking-wider">05 · Readiness</h3>
            {readiness ? (
              <dl className="mt-3 grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-xs">
                <dt className="text-white/50">Gate</dt>
                <dd>{readiness.gate_status}</dd>
                <dt className="text-white/50">Mode</dt>
                <dd>{readiness.mode}</dd>
                <dt className="text-white/50">Pass</dt>
                <dd>{readiness.summary.pass ?? 0}</dd>
                <dt className="text-white/50">Warnings</dt>
                <dd>{readiness.summary.warn ?? 0}</dd>
                <dt className="text-white/50">Failures</dt>
                <dd>{readiness.summary.fail ?? 0}</dd>
                <dt className="text-white/50">Mutation</dt>
                <dd className="break-words">{readiness.mutation_policy}</dd>
              </dl>
            ) : (
              <p className="mt-3 text-xs text-amber-200">Readiness profile unavailable.</p>
            )}
          </article>

          {actionableChecks.length > 0 && (
            <article className="rounded border border-white/15 bg-white/[0.03] p-4 md:col-span-2">
              <h3 className="text-sm uppercase tracking-wider">Actionable diagnostics</h3>
              <ul className="mt-3 space-y-3">
                {actionableChecks.map(check => (
                  <li
                    key={check.check_id}
                    className={`rounded border p-3 text-xs ${
                      check.status === 'fail'
                        ? 'border-red-400/40 bg-red-950/20'
                        : 'border-amber-300/30 bg-amber-950/15'
                    }`}
                  >
                    <div className="flex flex-wrap justify-between gap-2">
                      <span>{check.title}</span>
                      <span className="uppercase text-white/60">
                        {check.status} · {check.severity}
                      </span>
                    </div>
                    <p className="mt-1 font-mono text-[10px] text-white/45">{check.check_id}</p>
                    {check.evidence.length > 0 && (
                      <p className="mt-2 text-white/60">Evidence: {check.evidence.join('; ')}</p>
                    )}
                    <p className="mt-1 text-white/85">Recovery: {check.recommendation}</p>
                  </li>
                ))}
              </ul>
            </article>
          )}

          <article className="rounded border border-white/15 bg-white/[0.03] p-4 md:col-span-2">
            <h3 className="text-sm uppercase tracking-wider">06 · Guided setup</h3>
            <ol className="mt-3 space-y-2">
              {snapshot.guided.steps.map(step => (
                <li key={step.step_id} className="rounded border border-white/10 p-3 text-xs">
                  <div className="flex justify-between gap-3"><span>{step.title}</span><span className="uppercase text-white/50">{step.status}</span></div>
                  <p className="mt-1 text-white/65">{step.prompt}</p>
                  <p className="mt-1">Next: {step.next_action}</p>
                </li>
              ))}
            </ol>
          </article>

          <article className="rounded border border-white/15 bg-white/[0.03] p-4 md:col-span-2">
            <h3 className="text-sm uppercase tracking-wider">Degradation and recovery</h3>
            <ul className="mt-3 grid gap-2 sm:grid-cols-2">
              {snapshot.recovery.map(item => (
                <li key={item.condition_id} className={`rounded border p-3 text-xs ${item.detected ? 'border-amber-300/40 bg-amber-950/15' : 'border-white/10'}`}>
                  <div className="uppercase">{item.condition_id} · {item.detected ? 'detected' : 'guidance'}</div>
                  <p className="mt-1 text-white/60">{item.summary}</p>
                  <p className="mt-1">Recovery: {item.action}</p>
                </li>
              ))}
            </ul>
          </article>

          {snapshot.optionalServices.length > 0 && (
            <article className="rounded border border-white/15 bg-white/[0.03] p-4 md:col-span-2">
              <h3 className="text-sm uppercase tracking-wider">Optional runtime services</h3>
              <ul className="mt-3 space-y-2">
                {snapshot.optionalServices.map(service => (
                  <li key={service.service_id} className="text-xs">
                    {service.service_id} · opt-in · disabled · does not block required services
                  </li>
                ))}
              </ul>
            </article>
          )}
        </div>
      )}
    </section>
  )
}
