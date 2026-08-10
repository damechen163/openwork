export type CheckStatus = "pass" | "warn" | "fail";

export interface DoctorCheck {
  readonly id: string;
  readonly message: string;
  readonly remediation?: string;
  readonly status: CheckStatus;
  readonly value?: string;
}

export interface DoctorReport {
  readonly checks: readonly DoctorCheck[];
  readonly generatedAt: string;
  readonly schemaVersion: "openwork.doctor/v1";
  readonly status: "healthy" | "degraded" | "blocked";
}

export interface InstallStep {
  readonly description: string;
  readonly id: string;
}

export interface InstallDryRunPlan {
  readonly doctor: DoctorReport;
  readonly dryRun: true;
  readonly mutationsPerformed: false;
  readonly schemaVersion: "openwork.install-plan/v1";
  readonly steps: readonly InstallStep[];
}
