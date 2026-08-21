import { useCallback, useEffect, useState } from 'react'
import { api } from '../api'
import type { RuntimeSummary } from '../types'

export default function Runtime(): React.JSX.Element {
  const [runtimes, setRuntimes] = useState<RuntimeSummary[] | null>(null)
  const [selected, setSelected] = useState<RuntimeSummary | null>(null)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      const list = await api.runtimeList()
      setRuntimes(list)
      setError(null)
      if (list.length > 0) {
        setSelected(await api.runtimeInfo(list[0].metadata.id))
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const select = async (id: string): Promise<void> => {
    try {
      setSelected(await api.runtimeInfo(id))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  return (
    <div className="page">
      <h1 className="page-title">Runtimes</h1>
      <p className="page-subtitle">
        Registered agent runtimes: detection, version, authentication, and
        capabilities.
      </p>

      {error && <div className="empty-state">Failed: {error}</div>}
      {!runtimes && !error && (
        <div className="empty-state">
          <span className="spinner" />
          Loading runtimes…
        </div>
      )}

      {runtimes && runtimes.length === 0 && (
        <div className="empty-state">No runtimes registered.</div>
      )}

      {runtimes && runtimes.length > 0 && (
        <div className="card">
          <table className="table">
            <thead>
              <tr>
                <th>Runtime</th>
                <th>State</th>
                <th>Version</th>
                <th>Auth</th>
                <th>Run</th>
              </tr>
            </thead>
            <tbody>
              {runtimes.map((runtime) => (
                <tr
                  key={runtime.metadata.id}
                  onClick={() => void select(runtime.metadata.id)}
                  style={{ cursor: 'pointer' }}
                >
                  <td>
                    <div>{runtime.metadata.name}</div>
                    <div className="muted mono">{runtime.metadata.id}</div>
                  </td>
                  <td>
                    <span className={`badge ${stateBadge(runtime.detection.state)}`}>
                      {runtime.detection.state}
                    </span>
                  </td>
                  <td className="mono">{runtime.version ?? '—'}</td>
                  <td>
                    <span className={`badge ${authBadge(runtime.auth)}`}>
                      {runtime.auth}
                    </span>
                  </td>
                  <td className="mono">{runtime.capabilities.run ? '✓' : '✗'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {selected && (
        <div className="card">
          <div className="card-title">{selected.metadata.name} — details</div>
          <dl className="kv">
            <dt>ID</dt>
            <dd>{selected.metadata.id}</dd>
            <dt>Upstream</dt>
            <dd>{selected.metadata.upstream}</dd>
            <dt>License</dt>
            <dd>{selected.metadata.license}</dd>
            <dt>Distribution</dt>
            <dd>{selected.metadata.distribution}</dd>
            <dt>Executable</dt>
            <dd>{selected.detection.executable ?? '—'}</dd>
            <dt>Version</dt>
            <dd>{selected.version ?? '—'}</dd>
            <dt>Auth</dt>
            <dd>{selected.auth}</dd>
            <dt>Details</dt>
            <dd>{selected.detection.details ?? '—'}</dd>
            <dt>Capabilities</dt>
            <dd className="mono">
              install:{selected.capabilities.install ? '✓' : '✗'} · update:
              {selected.capabilities.update ? '✓' : '✗'} · run:
              {selected.capabilities.run ? '✓' : '✗'} · cancel:
              {selected.capabilities.cancel ? '✓' : '✗'}
            </dd>
          </dl>
        </div>
      )}
    </div>
  )
}

function stateBadge(state: string): string {
  switch (state) {
    case 'Healthy':
      return 'ok'
    case 'Broken':
      return 'error'
    default:
      return 'muted'
  }
}

function authBadge(auth: string): string {
  switch (auth) {
    case 'Authenticated':
      return 'ok'
    case 'Unauthenticated':
      return 'warn'
    default:
      return 'muted'
  }
}
