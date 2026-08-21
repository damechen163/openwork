import { useCallback, useEffect, useState } from 'react'
import { api } from '../api'
import type { InstallExecutionReport, InstallPlan } from '../types'

export default function Install(): React.JSX.Element {
  const [plan, setPlan] = useState<InstallPlan | null>(null)
  const [report, setReport] = useState<InstallExecutionReport | null>(null)
  const [runtime, setRuntime] = useState('')
  const [version, setVersion] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const loadPlan = useCallback(async () => {
    setBusy(true)
    setError(null)
    try {
      const options = runtime ? { runtime, version: version || undefined } : {}
      setPlan(await api.installPlan(options))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(false)
    }
  }, [runtime, version])

  useEffect(() => {
    void loadPlan()
  }, [loadPlan])

  const execute = async (): Promise<void> => {
    setBusy(true)
    setError(null)
    setReport(null)
    try {
      const options = runtime ? { runtime, version: version || undefined } : {}
      setReport(await api.installExecute(options))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="page">
      <h1 className="page-title">Install</h1>
      <p className="page-subtitle">
        Preview the managed Bootstrap plan, then apply it with explicit
        consent. Existing runtime installations are preserved.
      </p>

      <div className="card">
        <div className="form-row">
          <div className="form-group">
            <label className="form-label" htmlFor="install-runtime">
              Runtime (optional)
            </label>
            <input
              id="install-runtime"
              className="form-input"
              placeholder="e.g. claude-code"
              value={runtime}
              onChange={(event) => setRuntime(event.target.value)}
            />
          </div>
          <div className="form-group">
            <label className="form-label" htmlFor="install-version">
              Version (optional)
            </label>
            <input
              id="install-version"
              className="form-input"
              placeholder="advisory upstream version"
              value={version}
              onChange={(event) => setVersion(event.target.value)}
            />
          </div>
        </div>
        <button
          className="btn"
          onClick={() => void loadPlan()}
          disabled={busy}
        >
          Preview plan
        </button>
      </div>

      {error && (
        <div className="card">
          <div className="empty-state">Failed: {error}</div>
        </div>
      )}

      {plan && !report && (
        <div className="card">
          <div className="card-title">Plan ({plan.steps.length} steps)</div>
          {plan.steps.map((step) => (
            <div key={step.id} className="check-row">
              <span className="badge muted">{step.action}</span>
              <div className="check-main">
                <div className="check-id">{step.id}</div>
                <div className="check-summary">{step.path}</div>
              </div>
            </div>
          ))}
          {plan.warnings.map((warning) => (
            <div key={warning} className="check-remediation">
              warning: {warning}
            </div>
          ))}
          <div style={{ marginTop: 14 }}>
            <button
              className="btn primary"
              onClick={() => void execute()}
              disabled={busy}
            >
              Execute install (consent)
            </button>
          </div>
        </div>
      )}

      {report && (
        <div className="card">
          <div className="card-title">
            Execution{' '}
            <span className={`badge ${report.completed ? 'ok' : 'error'}`}>
              {report.completed ? 'completed' : 'failed'}
            </span>
            {report.partial_state && <span className="badge warn">partial</span>}
          </div>
          {report.steps.map((step) => (
            <div key={step.id} className="check-row">
              <span className={`badge ${stepBadge(step.status)}`}>
                {step.status}
              </span>
              <div className="check-main">
                <div className="check-id">{step.id}</div>
                <div className="check-summary">{step.detail}</div>
              </div>
            </div>
          ))}
          {report.rollback_warnings.map((warning) => (
            <div key={warning} className="check-remediation">
              rollback: {warning}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

function stepBadge(status: string): string {
  switch (status) {
    case 'done':
    case 'completed':
    case 'success':
      return 'ok'
    case 'failed':
    case 'error':
      return 'error'
    case 'pending':
      return 'muted'
    default:
      return 'warn'
  }
}
