import { contextBridge, ipcRenderer } from 'electron'

interface CliOutcome {
  ok: boolean
  exitCode: number | null
  stdout: string
  stderr: string
}

export interface RunOptions {
  workspace: string
  prompt: string
  runtime: string
  timeout: number
  sandboxTimeout: number
}

export interface RunOutputLine {
  stream: 'stdout' | 'stderr'
  line: string
}

const api = {
  version: (): Promise<CliOutcome> => ipcRenderer.invoke('openwork:version'),
  status: (): Promise<CliOutcome> => ipcRenderer.invoke('openwork:status'),
  doctor: (): Promise<CliOutcome> => ipcRenderer.invoke('openwork:doctor'),
  installPlan: (options: {
    runtime?: string
    version?: string
  }): Promise<CliOutcome> => ipcRenderer.invoke('openwork:installPlan', options),
  installExecute: (options: {
    runtime?: string
    version?: string
  }): Promise<CliOutcome> =>
    ipcRenderer.invoke('openwork:installExecute', options),
  runtimeList: (): Promise<CliOutcome> =>
    ipcRenderer.invoke('openwork:runtimeList'),
  runtimeInfo: (id: string): Promise<CliOutcome> =>
    ipcRenderer.invoke('openwork:runtimeInfo', id),
  run: (options: RunOptions): Promise<CliOutcome> =>
    ipcRenderer.invoke('openwork:run', options),
  runCancel: (): Promise<{ ok: boolean }> =>
    ipcRenderer.invoke('openwork:runCancel'),
  onRunOutput: (listener: (line: RunOutputLine) => void): (() => void) => {
    const handler = (_event: unknown, line: RunOutputLine): void =>
      listener(line)
    ipcRenderer.on('run:output', handler)
    return () => {
      ipcRenderer.removeListener('run:output', handler)
    }
  },
  onRunDone: (listener: (outcome: CliOutcome) => void): (() => void) => {
    const handler = (_event: unknown, outcome: CliOutcome): void =>
      listener(outcome)
    ipcRenderer.on('run:done', handler)
    return () => {
      ipcRenderer.removeListener('run:done', handler)
    }
  },
}

export type OpenworkApi = typeof api

contextBridge.exposeInMainWorld('openwork', api)
