import { describe, expect, it } from "vitest";

import { createDryRunPlan } from "../src/install.js";
import type { DoctorReport } from "../src/types.js";

const healthyDoctor: DoctorReport = {
  checks: [],
  generatedAt: "2026-08-10T00:00:00.000Z",
  schemaVersion: "openwork.doctor/v1",
  status: "healthy",
};

describe("createDryRunPlan", () => {
  it("contains every install stage while explicitly performing no mutation", () => {
    const plan = createDryRunPlan(healthyDoctor);

    expect(plan.dryRun).toBe(true);
    expect(plan.mutationsPerformed).toBe(false);
    expect(plan.steps.map((step) => step.id)).toEqual([
      "preflight",
      "profile",
      "configuration",
      "secrets",
      "services",
      "migrations",
      "admin",
      "packs",
      "health",
      "model-smoke",
      "report",
    ]);
  });
});
