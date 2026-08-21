// TypeScript mirrors of the openwork CLI `--json` outputs.

export interface CliOutcome {
  ok: boolean
  exitCode: number | null
  stdout: string
  stderr: string
}

export interface OpenWorkError {
  code: string
  message: string
  remediation?: string
}

export interface OpenWorkPaths {
  config: string
  data: string
  cache: string
  logs: string
  bin: string
}

export interface PlatformInfo {
  schema_version: number
  os: string
  os_version: string | null
  architecture: string
  environment: string
  support_tier: string
  shell: string | null
  package_managers: string[]
  paths: OpenWorkPaths
  permissions: {
    home_writable: boolean
    install_dir_writable: boolean
    elevated: boolean
  }
  resources: {
    total_memory_bytes: number | null
    available_disk_bytes: number | null
  }
  prerequisites: {
    git_present: boolean
    docker_present: boolean
  }
}

export interface RuntimeMetadata {
  id: string
  name: string
  upstream: string
  license: string
  distribution: string
}

export type DetectionState = 'Missing' | 'Healthy' | 'Broken'

export interface RuntimeDetection {
  state: DetectionState
  executable: string | null
  details: string | null
}

export type AuthStatus = 'Authenticated' | 'Unauthenticated' | 'Unknown'

export interface RuntimeCapabilities {
  install: boolean
  uninstall: boolean
  update: boolean
  authenticate: boolean
  run: boolean
  cancel: boolean
}

export interface RuntimeSummary {
  metadata: RuntimeMetadata
  detection: RuntimeDetection
  version: string | null
  auth: AuthStatus
  capabilities: RuntimeCapabilities
}

export interface StatusReport {
  schema_version: number
  state: string
  platform: PlatformInfo
  runtimes: RuntimeSummary[]
  lockfile: unknown | null
}

export type CheckStatus = 'PASS' | 'WARN' | 'FAIL' | 'SKIP'

export interface DoctorCheck {
  id: string
  status: CheckStatus
  summary: string
  details?: string
  remediation?: string
}

export interface DoctorReport {
  schema_version: number
  generated_at_unix_seconds: number
  checks: DoctorCheck[]
}

export interface InstallStep {
  id: string
  action: string
  path: string
  reason: string
}

export interface InstallPlan {
  schema_version: number
  dry_run: boolean
  steps: InstallStep[]
  warnings: string[]
}

export interface StepResult {
  id: string
  status: string
  detail: string
}

export interface InstallExecutionReport {
  schema_version: number
  completed: boolean
  partial_state: boolean
  steps: StepResult[]
  warnings: string[]
  rollback_warnings: string[]
}

export type RunStatus =
  | 'Queued'
  | 'Planning'
  | 'AwaitingApproval'
  | 'Running'
  | 'Succeeded'
  | 'Failed'
  | 'Cancelled'
  | 'TimedOut'

export interface Artifact {
  schema_version: number
  id: string
  run_id: string
  path: string
  media_type: string
  size_bytes: number
  sha256: string
  created_at: string
}

export interface RunReport {
  run_id: string
  status: RunStatus
  artifacts: Artifact[]
}
