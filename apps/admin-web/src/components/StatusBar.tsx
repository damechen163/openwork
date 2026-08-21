import { useState } from 'react'
import { api } from '../api'
import type { StatusReport } from '../types'

export function StatusBar({
  connected,
  onRefresh,
}: {
  connected: boolean
  onRefresh: () => void
}): React.JSX.Element {
  const [status, setStatus] = useState<StatusReport | null>(null)
  const [refreshing, setRefreshing] = useState(false)

  const refresh = async (): Promise<void> => {
    setRefreshing(true)
    try {
      setStatus(await api.status())
    } catch {
      setStatus(null)
    } finally {
      setRefreshing(false)
      onRefresh()
    }
  }

  const stateClass =
    status?.state === 'installed' ? 'ok' : status ? 'warn' : 'muted'

  return (
    <header className="topbar">
      <div className="topbar-left">
        <span className={`pill ${stateClass}`}>
          {status ? `state: ${status.state}` : 'state: unknown'}
        </span>
        {status?.platform && (
          <span className="pill muted">
            {status.platform.os} · {status.platform.architecture} ·{' '}
            {status.platform.environment}
          </span>
        )}
        {status?.runtimes && (
          <span className="pill muted">
            {status.runtimes.filter((r) => r.detection.state === 'Healthy').length}/
            {status.runtimes.length} runtimes healthy
          </span>
        )}
      </div>
      <div className="topbar-right">
        <span className={`dot ${connected ? 'ok' : 'error'}`} />
        <span className="muted">{connected ? 'CLI connected' : 'CLI offline'}</span>
        <button className="btn ghost" onClick={() => void refresh()} disabled={refreshing}>
          {refreshing ? '…' : '⟳ Refresh'}
        </button>
      </div>
    </header>
  )
}
