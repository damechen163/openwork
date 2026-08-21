import { useCallback, useEffect, useState } from 'react'
import { api } from '../api'
import type { DoctorReport } from '../types'

export default function Doctor(): React.JSX.Element {
  const [report, setReport] = useState<DoctorReport | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [running, setRunning] = useState(false)

  const run = useCallback(async () => {
    setRunning(true)
    try {
      setReport(await api.doctor())
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setRunning(false)
    }
  }, [])

  useEffect(() => {
    void run()
  }, [run])

  const failCount = report?.checks.filter((c) => c.status === 'FAIL').length ?? 0

  return (
    <div className="page">
      <h1 className="page-title">Doctor</h1>
      <p className="page-subtitle">
        Structured host diagnostics. {failCount > 0 ? `${failCount} failing checks.` : 'All checks passing.'}
      </p>

      <div className="card">
        <div className="card-title">
          Checks
          <span style={{ float: 'right' }}>
            <button
              className="btn ghost"
              onClick={() => void run()}
              disabled={running}
            >
              {running ? 'Running…' : '⟳ Re-run'}
            </button>
          </span>
        </div>
        {error && <div className="empty-state">Failed: {error}</div>}
        {!report && !error && (
          <div className="empty-state">
            <span className="spinner" />
            Running diagnostics…
          </div>
        )}
        {report?.checks.map((check) => (
          <div key={check.id} className="check-row">
            <span className={`badge ${statusBadge(check.status)}`}>
              {check.status}
            </span>
            <div className="check-main">
              <div className="check-id">{check.id}</div>
              <div className="check-summary">{check.summary}</div>
              {check.details && <div className="check-detail">{check.details}</div>}
              {check.remediation && (
                <div className="check-remediation">remediation: {check.remediation}</div>
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

function statusBadge(status: string): string {
  switch (status) {
    case 'PASS':
      return 'ok'
    case 'WARN':
      return 'warn'
    case 'FAIL':
      return 'error'
    default:
      return 'muted'
  }
}
