// sigil: REPAIR
import { Copy, FileSearch, FolderOpen, ShieldCheck } from 'lucide-react'
import { useMemo, useState } from 'react'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { buildSourceActionContract, formatSourceActionPacket } from '../../../lib/sourceActionContract'
import { safeTauriInvoke } from '../../../lib/tauriGuard'

interface SourceActionContractPanelProps {
  record: ArdaSourceProvenance
}

async function copyText(value: string): Promise<boolean> {
  if (!navigator.clipboard?.writeText) return false
  await navigator.clipboard.writeText(value)
  return true
}

interface SourceRevealResult {
  ok?: boolean
  sourcePath?: string
  resolvedPath?: string
  message?: string
}

type CopySourceActionId = 'copy_source_paths' | 'copy_source_packet'

export default function SourceActionContractPanel({ record }: SourceActionContractPanelProps) {
  const contract = useMemo(() => buildSourceActionContract(record), [record])
  const [message, setMessage] = useState<string | null>(null)

  const runCopyAction = async (actionId: CopySourceActionId) => {
    const value = actionId === 'copy_source_paths'
      ? contract.packet.sourcePaths.join('\n')
      : formatSourceActionPacket(contract)
    const ok = await copyText(value)
    setMessage(ok ? `${actionId} copied` : 'Clipboard unavailable in this runtime')
  }

  const revealNativeSource = async (sourcePath: string) => {
    if (!sourcePath || sourcePath === 'no-source-path') {
      setMessage('No source path available to reveal')
      return
    }

    try {
      const result = await safeTauriInvoke<SourceRevealResult>('reveal_source_path', { sourcePath })
      setMessage(result.message ?? `Revealed ${result.resolvedPath ?? sourcePath}`)
    } catch (error) {
      setMessage(`Native reveal unavailable: ${String(error)}`)
    }
  }

  return (
    <section className="planning-action-contract source-action-contract" aria-label="Source action contract">
      <div className="module-subtitle"><FileSearch size={14} /> {contract.title}</div>
      <article className="document-list__item">
        <div className="document-list__title-row">
          <strong>{contract.sourceId}</strong>
          <span>{contract.safeActions.length} safe / {contract.gatedActions.length} gated</span>
        </div>
        <p>Source actions are limited to local copy/export and scoped native reveal; external sharing remains gated.</p>
        <dl className="operating-surface-capability-contract">
          <div><dt>State</dt><dd>{contract.packet.state} / {contract.packet.sourceKind}</dd></div>
          <div><dt>Sources</dt><dd>{contract.packet.sourcePaths.join(', ') || 'No source paths'}</dd></div>
          <div><dt>Safe lane</dt><dd>copy local source paths, copy packet, or reveal a scoped workspace path</dd></div>
        </dl>

        <div className="world-terminal-action-contract__actions">
          {contract.safeActions.map((action) => {
            if (action.id === 'reveal_native_source') {
              return (
                <button
                  type="button"
                  className="refresh-button"
                  key={action.id}
                  onClick={() => void revealNativeSource(action.target)}
                  title={action.target}
                >
                  <FolderOpen size={12} /> {action.label}
                </button>
              )
            }
            const copyActionId = action.id as CopySourceActionId
            return (
              <button
                type="button"
                className="refresh-button"
                key={action.id}
                onClick={() => void runCopyAction(copyActionId)}
                title={action.target}
              >
                <Copy size={12} /> {action.label}
              </button>
            )
          })}
        </div>

        <div className="world-terminal-action-contract__governed">
          {contract.gatedActions.map((action) => (
            <div className="planning-action-contract__command" key={action.id}>
              <ShieldCheck size={14} />
              <code>{action.label}: gated / {action.reason}</code>
            </div>
          ))}
        </div>
      </article>
      {message ? <p className="planning-action-contract__message">{message}</p> : null}
    </section>
  )
}
