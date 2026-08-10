import { describe, expect, it, vi } from "vitest";

import { runCli } from "../src/cli.js";
import type { HostProbe } from "../src/host-probe.js";

const healthyProbe: HostProbe = {
  architecture: () => "arm64",
  cpuCount: () => 6,
  diskFreeBytes: () => 80n * 1024n ** 3n,
  dockerVersion: () => "Docker version 29.2.0",
  composeVersion: () => "Docker Compose version v5.0.2",
  memoryBytes: () => 8 * 1024 ** 3,
  operatingSystem: () => "linux",
};

describe("runCli", () => {
  it("prints the package version", () => {
    const stdout = vi.fn();
    const exitCode = runCli(["version"], {
      now: () => new Date(0),
      probe: healthyProbe,
      stdout,
    });

    expect(exitCode).toBe(0);
    expect(stdout).toHaveBeenCalledWith("0.1.0-alpha.0");
  });

  it("prints doctor JSON", () => {
    const stdout = vi.fn();
    const exitCode = runCli(["doctor", "--json"], {
      now: () => new Date("2026-08-10T00:00:00.000Z"),
      probe: healthyProbe,
      stdout,
    });

    expect(exitCode).toBe(0);
    expect(JSON.parse(stdout.mock.calls[0]?.[0] as string)).toMatchObject({
      status: "healthy",
    });
  });

  it("supports install dry-run and rejects mutating install in Phase 0", () => {
    const stdout = vi.fn();

    expect(
      runCli(["install", "--dry-run", "--json"], {
        now: () => new Date(0),
        probe: healthyProbe,
        stdout,
      }),
    ).toBe(0);
    expect(JSON.parse(stdout.mock.calls[0]?.[0] as string)).toMatchObject({
      dryRun: true,
      mutationsPerformed: false,
    });
    expect(
      runCli(["install"], {
        now: () => new Date(0),
        probe: healthyProbe,
        stdout,
      }),
    ).toBe(2);
  });
});
