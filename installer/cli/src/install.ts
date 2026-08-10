import type { DoctorReport, InstallDryRunPlan } from "./types.js";

const installSteps = [
  ["preflight", "Validate the supported host contract and required ports."],
  ["profile", "Select an installation profile."],
  ["configuration", "Generate openwork.yaml and non-secret configuration."],
  [
    "secrets",
    "Write secrets to a permission-restricted file without logging values.",
  ],
  ["services", "Start pinned service images."],
  ["migrations", "Apply database migrations."],
  ["admin", "Create the initial administrator."],
  ["packs", "Install selected base capability packs."],
  ["health", "Wait for service health checks."],
  ["model-smoke", "Run a minimal model connectivity test."],
  ["report", "Write the installation report and next-step URL."],
] as const;

export function createDryRunPlan(doctor: DoctorReport): InstallDryRunPlan {
  return {
    doctor,
    dryRun: true,
    mutationsPerformed: false,
    schemaVersion: "openwork.install-plan/v1",
    steps: installSteps.map(([id, description]) => ({ description, id })),
  };
}
