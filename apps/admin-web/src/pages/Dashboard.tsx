import { useCallback, useEffect, useState } from 'react'
import { api } from '../api'
import type { StatusReport } from '../types'

export default function Dashboard(): React.JSX.Element {
  const [status, setStatus] = useState<StatusReport | null>(null)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setStatus(await api.status())
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  if (error) {
    return (
      <div className="page">
        <h1 className="page-title">Dashboard</h1>
        <div className="card">
          <div className="empty-state">Failed to load status: {error}</div>
        </div>
      </div>
    )
  }
  if (!status) {
    return (
      <div className="page">
        <div className="empty-state">
          <span className="spinner" />
          Loading status…
        </div>
      </div>
    )
  }

  const { platform, runtimes } = status
  const healthy = runtimes.filter((r) => r.detection.state === 'Healthy').length
  const authenticated = runtimes.filter((r) => r.auth === 'Authenticated').length

  return (
    <div className="page">
      <h1 className="page-title">Dashboard</h1>
      <p className="page-subtitle">
        Host state and runtime health for this OpenWork installation.
      </p>

      <div className="grid">
        <div className="card">
          <div className="stat-label">Installation</div>
          <div className={`stat-value ${status.state === 'installed' ? 'ok' : 'warn'}`}>
            {status.state}
          </div>
        </div>
        <div className="card">
          <div className="stat-label">Runtimes healthy</div>
          <div className="stat-value ok">
            {healthy}/{runtimes.length}
          </div>
        </div>
        <div className="card">
          <div className="stat-label">Authenticated</div>
          <div className="stat-value">{authenticated}</div>
        </div>
        <div className="card">
          <div className="stat-label">Platform</div>
          <div className="stat-value">
            {platform.architecture} · {platform.support_tier}
          </div>
        </div>
      </div>

      <div className="card">
        <div className="card-title">Host</div>
        <dl className="kv">
          <dt>OS</dt>
          <dd>
            {platform.os} {platform.os_version ?? ''}
          </dd>
          <dt>Environment</dt>
          <dd>{platform.environment}</dd>
          <dt>Shell</dt>
          <dd>{platform.shell ?? '—'}</dd>
          <dt>Package managers</dt>
          <dd>{platform.package_managers.join(', ') || '—'}</dd>
          <dt>Config</dt>
          <dd>{platform.paths.config}</dd>
          <dt>Data</dt>
          <dd>{platform.paths.data}</dd>
          <dt>Bin</dt>
          <dd>{platform.paths.bin}</dd>
          <dt>Prerequisites</dt>
          <dd>
            git: {platform.prerequisites.git_present ? '✓' : '✗'}, docker:{' '}
            {platform.prerequisites.docker_present ? '✓' : '✗'}
          </dd>
        </dl>
      </div>

      <div className="card">
        <div className="card-title">Runtimes</div>
        {runtimes.length === 0 ? (
          <div className="empty-state">No runtimes registered.</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Runtime</th>
                <th>State</th>
                <th>Version</th>
                <th>Auth</th>
                <th>Capabilities</th>
              </tr>
            </thead>
            <tbody>
              {runtimes.map((runtime) => (
                <tr key={runtime.metadata.id}>
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
                  <td className="mono">
                    {runtime.capabilities.run ? 'run ✓' : 'run ✗'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
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
