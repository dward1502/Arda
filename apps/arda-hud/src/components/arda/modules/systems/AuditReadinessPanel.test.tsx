import { render, screen } from '@testing-library/react'
import AuditReadinessPanel from './AuditReadinessPanel'

describe('AuditReadinessPanel', () => {
  it('separates closed Phase 7 remediation from Phase 8 hardening and roadmap boundaries', () => {
    render(
      <AuditReadinessPanel
        readiness={{
          phase7_closeout: { status: 'closed', verified_slices: 8, total_slices: 8 },
          phase8_hardening: { completed: 4, queued: 3, in_progress: 1 },
          boundary: {
            summary: 'Audit evidence only; this is not live runtime/service status.',
            roadmap: 'Future embodied roadmap work remains separate from closed audit remediation.',
          },
          next_items: [
            { id: 'P8-HARD-005', title: 'Project audit readiness state into ARDA/Hermes surfaces', status: 'queued', scope: 'ARDA/Hermes visibility' },
          ],
          evidence_sources: ['audit/PROFESSIONALIZATION_AUDIT_2026-05-25/phase8-hardening-backlog.jsonl'],
        }}
      />,
    )

    expect(screen.getByText('Phase 7 remediation')).toBeInTheDocument()
    expect(screen.getByText('closed')).toBeInTheDocument()
    expect(screen.getByText('Phase 8 hardening queue')).toBeInTheDocument()
    expect(screen.getByText('Audit evidence only; this is not live runtime/service status.')).toBeInTheDocument()
    expect(screen.getByText('Future embodied roadmap work remains separate from closed audit remediation.')).toBeInTheDocument()
    expect(screen.getByText(/P8-HARD-005/)).toBeInTheDocument()
  })
})
