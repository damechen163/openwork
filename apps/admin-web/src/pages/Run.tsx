import { useEffect, useRef, useState } from 'react'
import { api } from '../api'
import type { RunReport, RuntimeSummary } from '../types'

interface OutputLine {
  stream: 'stdout' | 'stderr'
  line: string
}

export default function Run(): React.JSX.Element {
  const [workspace, setWorkspace] = useState('')
  const [prompt, setPrompt] = useState('')
  const [runtime, setRuntime] = useState('claude-code')
  const [runtimes, setRuntimes] = useState<RuntimeSummary[]>([])
  const [timeout, setTimeoutSec] = useState(300)
  const [sandboxTimeout, setSandboxTimeout] = useState(60)
  const [running, setRunning] = useState(false)
  const [lines, setLines] = useState<OutputLine[]>([])
  const [report, setReport] = useState<RunReport | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [phase, setPhase] = useState<string | null>(null)
  const terminalRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    void api
      .runtimeList()
      .then((list) => {
        setRuntimes(list)
        const runnable = list.find((r) => r.capabilities.run)
        if (runnable) setRuntime(runnable.metadata.id)
      })
      .catch(() => undefined)
  }, [])

  useEffect(() => {
    const offOutput = api.onRunOutput((line) => {
      const entry: OutputLine = {
        stream: line.stream === 'stderr' ? 'stderr' : 'stdout',
        line: line.line,
      }
      setLines((prev) => [...prev, entry])
      const progress = line.line.match(/^\[openwork\] (.*)$/)
      if (progress) setPhase(progress[1])
    })
    const offDone = api.onRunDone(() => {
      setRunning(false)
    })
    return () => {
      offOutput()
      offDone()
    }
  }, [])

  useEffect(() => {
    const terminal = terminalRef.current
    if (terminal) terminal.scrollTop = terminal.scrollHeight
  }, [lines])

  const start = async (): Promise<void> => {
    if (!workspace.trim() || !prompt.trim()) {
      setError('Workspace and prompt are required.')
      return
    }
    setError(null)
    setLines([])
    setReport(null)
    setPhase(null)
    setRunning(true)
    try {
      const result = await api.run({
        workspace: workspace.trim(),
        prompt: prompt.trim(),
        runtime,
        timeout,
        sandboxTimeout,
      })
      setReport(result)
      setPhase(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
      setPhase(null)
    } finally {
      setRunning(false)
    }
  }

  const cancel = async (): Promise<void> => {
    await api.runCancel()
  }

  const statusBadge =
    report && !running ? (
      <span className={`badge ${statusClass(report.status)}`}>{report.status}</span>
    ) : running ? (
      <span className="badge warn">
        <span className="spinner" />
        running
      </span>
    ) : null

  return (
    <div className="page">
      <h1 className="page-title">Run Task</h1>
      <p className="page-subtitle">
        Execute an AI task: the runtime writes an analysis script, the podman
        sandbox executes it (no network, read-only rootfs, non-root), and the
        outputs are recorded as artifacts.
      </p>

      <div className="card">
        <div className="form-group">
          <label className="form-label" htmlFor="run-workspace">
            Workspace
          </label>
          <input
            id="run-workspace"
            className="form-input mono"
            placeholder="e.g. /tmp/ow-demo/sales"
            value={workspace}
            onChange={(event) => setWorkspace(event.target.value)}
          />
        </div>

        <div className="form-group">
          <label className="form-label" htmlFor="run-prompt">
            Prompt
          </label>
          <textarea
            id="run-prompt"
            className="form-textarea"
            placeholder="Read README.md and implement analyze.py per the contract…"
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
          />
        </div>

        <div className="form-row">
          <div className="form-group">
            <label className="form-label" htmlFor="run-runtime">
              Runtime
            </label>
            <select
              id="run-runtime"
              className="form-select"
              value={runtime}
              onChange={(event) => setRuntime(event.target.value)}
            >
              {runtimes.map((runtime) => (
                <option key={runtime.metadata.id} value={runtime.metadata.id}>
                  {runtime.metadata.name} ({runtime.metadata.id})
                </option>
              ))}
            </select>
          </div>
          <div className="form-group">
            <label className="form-label" htmlFor="run-timeout">
              Runtime timeout (s)
            </label>
            <input
              id="run-timeout"
              type="number"
              min={1}
              className="form-input"
              value={timeout}
              onChange={(event) => setTimeoutSec(Number(event.target.value))}
            />
          </div>
        </div>

        <div className="form-group">
          <label className="form-label" htmlFor="run-sandbox-timeout">
            Sandbox timeout (s, 1–3600)
          </label>
          <input
            id="run-sandbox-timeout"
            type="number"
            min={1}
            max={3600}
            className="form-input"
            value={sandboxTimeout}
            onChange={(event) => setSandboxTimeout(Number(event.target.value))}
          />
        </div>

        <div style={{ display: 'flex', gap: 10 }}>
          <button className="btn primary" onClick={() => void start()} disabled={running}>
            {running ? 'Running…' : '▶ Run'}
          </button>
          {running && (
            <button className="btn danger" onClick={() => void cancel()}>
              ■ Cancel
            </button>
          )}
        </div>
      </div>

      {error && (
        <div className="card">
          <div className="empty-state">Failed: {error}</div>
        </div>
      )}

      {(running || lines.length > 0 || report) && (
        <div className="card">
          <div className="card-title">
            Output {statusBadge}
            {phase && <span className="muted" style={{ marginLeft: 10 }}>{phase}</span>}
          </div>
          <div className="terminal" ref={terminalRef}>
            {lines.length === 0 && <span className="terminal-empty">waiting for output…</span>}
            {lines.map((entry, index) => (
              <div key={index} className={`line-${entry.stream}`}>
                {entry.line}
              </div>
            ))}
          </div>
        </div>
      )}

      {report && (
        <div className="card">
          <div className="card-title">
            Run {report.run_id} —{' '}
            <span className={`badge ${statusClass(report.status)}`}>
              {report.status}
            </span>
          </div>
          {report.artifacts.length === 0 && (
            <div className="empty-state">No artifacts recorded.</div>
          )}
          {report.artifacts.map((artifact) => (
            <div key={artifact.id} className="artifact-row">
              <span className="badge muted">{artifact.media_type}</span>
              <div className="artifact-path">{artifact.path}</div>
              <div className="artifact-hash">
                {artifact.size_bytes} B · sha256:{artifact.sha256.slice(0, 16)}…
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

function statusClass(status: string): string {
  switch (status) {
    case 'Succeeded':
      return 'ok'
    case 'Cancelled':
    case 'TimedOut':
      return 'warn'
    default:
      return 'error'
  }
}
