import { useState, type FormEvent } from 'react'
import { FolderKanban, ShieldCheck } from 'lucide-react'
import {
  buildWorkbenchRunDraft,
  validateProjectContract,
  validateWorkbenchDraftInput,
  type ProjectContractValidation,
  type WorkbenchRunDraft,
} from '../../../lib/workbench'

export default function WorkbenchModule() {
  const [projectContractPath, setProjectContractPath] = useState('')
  const [objective, setObjective] = useState('')
  const [errors, setErrors] = useState<string[]>([])
  const [draft, setDraft] = useState<WorkbenchRunDraft | null>(null)
  const [contractValidation, setContractValidation] = useState<ProjectContractValidation | null>(null)
  const [validationError, setValidationError] = useState<string | null>(null)
  const [isValidating, setIsValidating] = useState(false)

  const validateContract = async () => {
    setIsValidating(true)
    setValidationError(null)
    setContractValidation(null)
    try {
      setContractValidation(await validateProjectContract(projectContractPath))
    } catch (error) {
      setValidationError(error instanceof Error ? error.message : String(error))
    } finally {
      setIsValidating(false)
    }
  }

  const prepare = (event: FormEvent) => {
    event.preventDefault()
    const nextErrors = validateWorkbenchDraftInput(projectContractPath, objective)
    setErrors(nextErrors)
    setDraft(nextErrors.length === 0 ? buildWorkbenchRunDraft(projectContractPath, objective) : null)
  }

  return (
    <section className="workbench-module" aria-label="Arda Workbench">
      <div className="module-subtitle"><FolderKanban size={14} /> Workbench Objective Draft</div>
      <p>
        Prepare the bounded project and objective envelope before native validation, attachment,
        approval, or execution.
      </p>
      <form onSubmit={prepare} className="split-stack">
        <label>
          Project contract path
          <input
            value={projectContractPath}
            onChange={(event) => {
              setProjectContractPath(event.target.value)
              setContractValidation(null)
              setValidationError(null)
            }}
            placeholder="/workspace/spec/project-contract/v1/examples/rust-project.json"
          />
        </label>
        <button
          type="button"
          className="refresh-button"
          disabled={isValidating || !projectContractPath.trim()}
          onClick={validateContract}
        >
          {isValidating ? 'Validating project contract…' : 'Validate project contract'}
        </button>
        <label>
          Objective
          <textarea
            value={objective}
            onChange={(event) => setObjective(event.target.value)}
            placeholder="Describe one bounded, verifiable change"
            rows={3}
          />
        </label>
        <button type="submit" className="refresh-button">Prepare governed run</button>
      </form>

      {errors.length > 0 ? (
        <div role="alert" className="planning-action-contract__message">
          {errors.map((error) => <p key={error}>{error}</p>)}
        </div>
      ) : null}

      {validationError ? (
        <div role="alert" className="planning-action-contract__message">
          Project contract validation failed: {validationError}
        </div>
      ) : null}

      {contractValidation ? (
        <section aria-label="Validated project contract" className="document-list split-stack">
          <div className="document-list__title-row">
            <strong>{contractValidation.name}</strong>
            <span>{contractValidation.schemaVersion}</span>
          </div>
          <div className="document-list__title-row">
            <span>{contractValidation.kind} · {contractValidation.runtimeAdapter}</span>
            <span>{contractValidation.permissions.authority}</span>
          </div>
          <div className="document-list__title-row">
            <span>{contractValidation.permissions.networkAllowed ? 'network allowed' : 'network denied'}</span>
            <span>{contractValidation.permissions.filesystemWrite ? 'filesystem write requested' : 'filesystem read-only'}</span>
          </div>
          <small>
            Checks: {contractValidation.checkIds.join(', ') || 'none'} · Commands: {contractValidation.commandIds.join(', ') || 'none'}
          </small>
          <p>Contract validated only; project is not attached and no command was started.</p>
        </section>
      ) : null}

      {draft ? (
        <div className="split-stack">
          <div className="document-list__title-row">
            <strong><ShieldCheck size={14} /> approval required</strong>
            <span>{draft.schemaVersion}</span>
          </div>
          <ol aria-label="Workbench run graph" className="document-list">
            {draft.nodes.map((node) => (
              <li key={node.id} className="document-list__item">
                <div className="document-list__title-row">
                  <strong>{node.kind}</strong>
                  <span>{node.state}</span>
                </div>
                <small>{node.authority}</small>
              </li>
            ))}
          </ol>
          <p>
            This graph is a draft only; no project was attached and no command, mutation, or
            provider job was started.
          </p>
        </div>
      ) : null}
    </section>
  )
}
