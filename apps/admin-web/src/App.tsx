import { useCallback, useEffect, useState } from 'react'
import { api } from './api'
import Dashboard from './pages/Dashboard'
import Install from './pages/Install'
import Doctor from './pages/Doctor'
import Runtime from './pages/Runtime'
import Run from './pages/Run'
import { StatusBar } from './components/StatusBar'

type Page = 'dashboard' | 'install' | 'doctor' | 'runtime' | 'run'

const NAV: { page: Page; label: string; icon: string }[] = [
  { page: 'dashboard', label: 'Dashboard', icon: '◈' },
  { page: 'run', label: 'Run Task', icon: '▶' },
  { page: 'install', label: 'Install', icon: '⬇' },
  { page: 'doctor', label: 'Doctor', icon: '✚' },
  { page: 'runtime', label: 'Runtimes', icon: '◉' },
]

export default function App(): React.JSX.Element {
  const [page, setPage] = useState<Page>('dashboard')
  const [version, setVersion] = useState<string>('')
  const [connected, setConnected] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const refreshMeta = useCallback(async () => {
    try {
      setVersion(await api.version())
      await api.status()
      setConnected(true)
      setError(null)
    } catch (err) {
      setConnected(false)
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [])

  useEffect(() => {
    void refreshMeta()
  }, [refreshMeta])

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">◆</span>
          <span className="brand-name">OpenWork</span>
        </div>
        <nav className="nav">
          {NAV.map((item) => (
            <button
              key={item.page}
              className={`nav-item ${page === item.page ? 'active' : ''}`}
              onClick={() => setPage(item.page)}
            >
              <span className="nav-icon">{item.icon}</span>
              {item.label}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer">
          <div className="bin-path" title="OpenWork binary">
            {window.openwork ? 'CLI bridge ready' : 'bridge unavailable'}
          </div>
          {version && <div className="version">v{version}</div>}
        </div>
      </aside>

      <div className="main">
        <StatusBar connected={connected} onRefresh={() => void refreshMeta()} />
        {error && (
          <div className="banner error">
            <span>CLI bridge error: {error}</span>
          </div>
        )}
        <main className="content">
          {page === 'dashboard' && <Dashboard />}
          {page === 'install' && <Install />}
          {page === 'doctor' && <Doctor />}
          {page === 'runtime' && <Runtime />}
          {page === 'run' && <Run />}
        </main>
      </div>
    </div>
  )
}
