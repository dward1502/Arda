import type { OnboardingSnapshot } from '../lib/tauri-core-compat'

interface OnboardingPanelProps {
  snapshot: OnboardingSnapshot | null
  error: string | null
  onClose: () => void
}

export default function OnboardingPanel({ snapshot, error, onClose }: OnboardingPanelProps) {
  const readiness = snapshot?.readiness
  const servicePlan = snapshot?.servicePlan

  return (
    <section
      aria-label="Operator onboarding status"
      aria-live="polite"
      className="absolute inset-x-4 top-4 z-50 mx-auto max-h-[70vh] max-w-3xl overflow-y-auto rounded border border-[#f4e9d8]/30 bg-black/90 p-5 font-mono text-[#f4e9d8] shadow-2xl backdrop-blur"
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-[10px] uppercase tracking-[0.3em] text-[#f4e9d8]/50">
            Read-only onboarding
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
          <article className="rounded border border-white/15 bg-white/[0.03] p-4">
            <h3 className="text-sm uppercase tracking-wider">Readiness</h3>
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
                <dt className="text-white/50">Mutation</dt>
                <dd className="break-words">{readiness.mutation_policy}</dd>
              </dl>
            ) : (
              <p className="mt-3 text-xs text-amber-200">Readiness profile unavailable.</p>
            )}
          </article>

          <article className="rounded border border-white/15 bg-white/[0.03] p-4">
            <h3 className="text-sm uppercase tracking-wider">Service plan</h3>
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
        </div>
      )}
    </section>
  )
}
