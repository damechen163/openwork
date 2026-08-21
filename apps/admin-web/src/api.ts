import type {
  CliOutcome,
  DoctorReport,
  InstallExecutionReport,
  InstallPlan,
  OpenWorkError,
  RunReport,
  RuntimeSummary,
  StatusReport,
} from './types'
import type { OpenworkApi, RunOptions } from '../electron/preload'

declare global {
  interface Window {
    openwork: OpenworkApi
  }
}

/** Parses a CLI JSON stdout, raising a structured error when the call failed. */
function parse<T>(outcome: CliOutcome, label: string): T {
  if (!outcome.ok) {
    let error: OpenWorkError | null = null
    try {
      error = JSON.parse(outcome.stdout) as OpenWorkError
    } catch {
      error = null
    }
    throw new Error(
      error?.message ??
        `${label} failed (exit ${outcome.exitCode ?? 'n/a'}): ${outcome.stderr.trim()}`,
    )
  }
  try {
    return JSON.parse(outcome.stdout) as T
  } catch {
    throw new Error(`${label} returned invalid JSON`)
  }
}

export const api = {
  async version(): Promise<string> {
    const outcome = await window.openwork.version()
    return outcome.stdout.trim()
  },

  async status(): Promise<StatusReport> {
    return parse(await window.openwork.status(), 'status')
  },

  async doctor(): Promise<DoctorReport> {
    return parse(await window.openwork.doctor(), 'doctor')
  },

  async installPlan(options: { runtime?: string; version?: string } = {}): Promise<InstallPlan> {
    return parse(await window.openwork.installPlan(options), 'install plan')
  },

  async installExecute(options: {
    runtime?: string
    version?: string
  } = {}): Promise<InstallExecutionReport> {
    return parse(await window.openwork.installExecute(options), 'install')
  },

  async runtimeList(): Promise<RuntimeSummary[]> {
    return parse(await window.openwork.runtimeList(), 'runtime list')
  },

  async runtimeInfo(id: string): Promise<RuntimeSummary> {
    return parse(await window.openwork.runtimeInfo(id), `runtime ${id}`)
  },

  async run(options: RunOptions): Promise<RunReport> {
    const outcome = await window.openwork.run(options)
    // The CLI prints the report as the last JSON on stdout; if it failed
    // before producing one, parse the error instead.
    const lastJson = outcome.stdout.trim().split('\n').pop() ?? ''
    try {
      return JSON.parse(lastJson) as RunReport
    } catch {
      let error: OpenWorkError | null = null
      try {
        error = JSON.parse(outcome.stdout) as OpenWorkError
      } catch {
        error = null
      }
      throw new Error(
        error?.message ??
          `run failed (exit ${outcome.exitCode ?? 'n/a'}): ${outcome.stderr.trim()}`,
      )
    }
  },

  runCancel(): Promise<{ ok: boolean }> {
    return window.openwork.runCancel()
  },

  onRunOutput(listener: (line: { stream: string; line: string }) => void): () => void {
    return window.openwork.onRunOutput(listener)
  },

  onRunDone(listener: (outcome: CliOutcome) => void): () => void {
    return window.openwork.onRunDone(listener)
  },
}
