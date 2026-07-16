import { render, screen } from '@testing-library/react'
import SetupConsoleReadinessPanel from './SetupConsoleReadinessPanel'

describe('SetupConsoleReadinessPanel', () => {
  it('projects zero-active-blocker portability status as a setup-console KPI', () => {
    render(
      <SetupConsoleReadinessPanel
        readiness={{
          gate_status: 'pass',
          mode: 'read_only',
          summary: { pass: 8, warn: 0, fail: 0 },
          portability_status: {
            status: 'pass',
            active_blocker_findings: 0,
            label: 'zero active portability blockers',
          },
          checks: [],
        }}
        guidedSession={{
          steps: [{ step_id: 'review_apply', status: 'human_gated' }],
          next_actions: ['review_apply: Run apply-config without approval to refresh projections.'],
        }}
        privateConfigStage={{
          entries: [{ key: 'ARDA_PROFILE' }],
          missing_required: [],
        }}
        servicePlan={{
          actions: [
            {
              action_id: 'onboarding.write_private_config_baseline',
              title: 'Write private non-secret config baseline',
              requires_human_gate: true,
            },
          ],
        }}
      />,
    )

    expect(screen.getByText('Portability blockers')).toBeInTheDocument()
    expect(screen.getByText('0')).toBeInTheDocument()
    expect(screen.getByText('zero active portability blockers')).toBeInTheDocument()
    expect(screen.getByText('First Light next')).toBeInTheDocument()
    expect(screen.getByText('Write private non-secret config baseline requires scoped human approval before private env writes.')).toBeInTheDocument()
  })
})
