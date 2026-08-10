import { describe, expect, it } from "vitest";

import { runDoctor } from "../src/doctor.js";
import type { HostProbe } from "../src/host-probe.js";

const healthyProbe: HostProbe = {
  architecture: () => "x64",
  cpuCount: () => 8,
  diskFreeBytes: () => 100n * 1024n ** 3n,
  dockerVersion: () => "Docker version 29.2.0",
  composeVersion: () => "Docker Compose version v5.0.2",
  memoryBytes: () => 16 * 1024 ** 3,
  operatingSystem: () => "linux",
};

describe("runDoctor", () => {
  it("returns a machine-readable healthy report", () => {
    const report = runDoctor(healthyProbe);

    expect(report.status).toBe("healthy");
    expect(report.checks.every((check) => check.status === "pass")).toBe(true);
    expect(report.checks.map((check) => check.id)).toEqual([
      "host.os",
      "host.arch",
      "host.cpu",
      "host.memory",
      "host.disk",
      "docker.engine",
      "docker.compose",
    ]);
  });

  it("reports actionable failures without throwing", () => {
    const report = runDoctor({
      ...healthyProbe,
      dockerVersion: () => null,
      operatingSystem: () => "darwin",
    });

    expect(report.status).toBe("blocked");
    expect(
      report.checks.find((check) => check.id === "host.os")?.remediation,
    ).toContain("Linux");
    expect(
      report.checks.find((check) => check.id === "docker.engine")?.status,
    ).toBe("fail");
  });
});
