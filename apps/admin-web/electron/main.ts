import { app, BrowserWindow, ipcMain } from 'electron'
import { spawn, type ChildProcess } from 'node:child_process'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

// The OpenWork GUI runs as root inside WSL2; Chromium's sandbox cannot start
// under root, so it is disabled explicitly.
app.commandLine.appendSwitch('no-sandbox')

const __dirname = path.dirname(fileURLToPath(import.meta.url))

let mainWindow: BrowserWindow | null = null
let activeRun: ChildProcess | null = null

function resolveOpenworkBin(): string {
  const configured = process.env.OPENWORK_BIN
  if (configured && configured.trim().length > 0) return configured
  return 'openwork'
}

interface CliOutcome {
  ok: boolean
  exitCode: number | null
  stdout: string
  stderr: string
}

function runCli(args: string[]): Promise<CliOutcome> {
  return new Promise((resolve) => {
    const child = spawn(resolveOpenworkBin(), args, {
      env: { ...process.env },
    })
    let stdout = ''
    let stderr = ''
    child.stdout.on('data', (chunk) => {
      stdout += chunk
    })
    child.stderr.on('data', (chunk) => {
      stderr += chunk
    })
    child.on('error', (error) => {
      resolve({ ok: false, exitCode: null, stdout, stderr: error.message })
    })
    child.on('close', (code) => {
      resolve({ ok: code === 0, exitCode: code, stdout, stderr })
    })
  })
}

function createWindow(): void {
  mainWindow = new BrowserWindow({
    width: 1280,
    height: 860,
    minWidth: 960,
    minHeight: 640,
    backgroundColor: '#0f1419',
    title: 'OpenWork Admin',
    webPreferences: {
      preload: path.join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false,
    },
  })

  const devUrl = process.env.VITE_DEV_SERVER_URL
  if (devUrl) {
    void mainWindow.loadURL(devUrl)
  } else {
    void mainWindow.loadFile(path.join(__dirname, '../dist/index.html'))
  }
}

function registerIpc(): void {
  ipcMain.handle('openwork:version', async () => runCli(['--version']))

  ipcMain.handle('openwork:status', async () => runCli(['status', '--json']))

  ipcMain.handle('openwork:doctor', async () => runCli(['doctor', '--json']))

  ipcMain.handle(
    'openwork:installPlan',
    async (_event, options: { runtime?: string; version?: string }) => {
      const args = ['install', '--dry-run', '--json']
      if (options.runtime) args.push('--runtime', options.runtime)
      if (options.version) args.push('--version', options.version)
      return runCli(args)
    },
  )

  ipcMain.handle(
    'openwork:installExecute',
    async (_event, options: { runtime?: string; version?: string }) => {
      const args = ['install', '--execute', '--yes', '--json']
      if (options.runtime) args.push('--runtime', options.runtime)
      if (options.version) args.push('--version', options.version)
      return runCli(args)
    },
  )

  ipcMain.handle('openwork:runtimeList', async () =>
    runCli(['runtime', 'list', '--json']),
  )

  ipcMain.handle('openwork:runtimeInfo', async (_event, id: string) =>
    runCli(['runtime', 'info', id, '--json']),
  )

  ipcMain.handle(
    'openwork:run',
    async (
      event,
      options: {
        workspace: string
        prompt: string
        runtime: string
        timeout: number
        sandboxTimeout: number
      },
    ) => {
      if (activeRun) {
        return {
          ok: false,
          exitCode: null,
          stdout: '',
          stderr: 'a run is already in progress',
        }
      }
      const args = [
        'run',
        '--workspace',
        options.workspace,
        '--runtime',
        options.runtime,
        '--timeout',
        String(options.timeout),
        '--sandbox-timeout',
        String(options.sandboxTimeout),
        '--json',
        options.prompt,
      ]
      return new Promise<CliOutcome>((resolve) => {
        const child = spawn(resolveOpenworkBin(), args, {
          env: { ...process.env },
        })
        activeRun = child
        let stdout = ''
        let stderr = ''
        child.stdout.on('data', (chunk) => {
          const text = chunk.toString()
          stdout += text
          for (const line of text.split('\n')) {
            if (line.trim().length > 0) {
              event.sender.send('run:output', { stream: 'stdout', line })
            }
          }
        })
        child.stderr.on('data', (chunk) => {
          const text = chunk.toString()
          stderr += text
          for (const line of text.split('\n')) {
            if (line.trim().length > 0) {
              event.sender.send('run:output', { stream: 'stderr', line })
            }
          }
        })
        child.on('error', (error) => {
          activeRun = null
          event.sender.send('run:done', {
            ok: false,
            exitCode: null,
            stdout,
            stderr: error.message,
          })
          resolve({ ok: false, exitCode: null, stdout, stderr: error.message })
        })
        child.on('close', (code) => {
          activeRun = null
          event.sender.send('run:done', {
            ok: code === 0,
            exitCode: code,
            stdout,
            stderr,
          })
          resolve({ ok: code === 0, exitCode: code, stdout, stderr })
        })
      })
    },
  )

  ipcMain.handle('openwork:runCancel', async () => {
    if (activeRun) {
      activeRun.kill('SIGINT')
      return { ok: true }
    }
    return { ok: false }
  })
}

app.whenReady().then(() => {
  registerIpc()
  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
