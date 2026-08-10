#!/usr/bin/env node

import { pathToFileURL } from "node:url";

import { runDoctor } from "./doctor.js";
import type { HostProbe } from "./host-probe.js";
import { systemHostProbe } from "./host-probe.js";
import { createDryRunPlan } from "./install.js";
import type { DoctorReport } from "./types.js";
import { OPENWORK_VERSION } from "./version.js";

export interface CliContext {
  readonly now: () => Date;
  readonly probe: HostProbe;
  readonly stdout: (line: string) => void;
}

const defaultContext: CliContext = {
  now: () => new Date(),
  probe: systemHostProbe,
  stdout: (line) => process.stdout.write(`${line}\n`),
};

function doctorText(report: DoctorReport): string {
  const lines = [`OpenWork doctor: ${report.status}`];
  for (const check of report.checks) {
    const value = check.value === undefined ? "" : ` (${check.value})`;
    lines.push(
      `[${check.status.toUpperCase()}] ${check.id}: ${check.message}${value}`,
    );
    if (check.remediation !== undefined) {
      lines.push(`  Remediation: ${check.remediation}`);
    }
  }
  return lines.join("\n");
}

function usage(): string {
  return [
    "Usage: openwork <command> [options]",
    "",
    "Commands:",
    "  version",
    "  doctor [--json]",
    "  install --dry-run [--json]",
  ].join("\n");
}

export function runCli(
  argv: readonly string[],
  context: CliContext = defaultContext,
): number {
  const [command] = argv;
  const json = argv.includes("--json");

  if (command === "version") {
    context.stdout(
      json ? JSON.stringify({ version: OPENWORK_VERSION }) : OPENWORK_VERSION,
    );
    return 0;
  }

  if (command === "doctor") {
    const report = runDoctor(context.probe, context.now);
    context.stdout(json ? JSON.stringify(report, null, 2) : doctorText(report));
    return report.status === "blocked" ? 1 : 0;
  }

  if (command === "install") {
    if (!argv.includes("--dry-run")) {
      context.stdout(
        "Mutating install is intentionally unavailable in Phase 0. Use: openwork install --dry-run",
      );
      return 2;
    }
    const report = runDoctor(context.probe, context.now);
    const plan = createDryRunPlan(report);
    context.stdout(
      json
        ? JSON.stringify(plan, null, 2)
        : [
            "OpenWork install dry-run",
            `Host status: ${report.status}`,
            ...plan.steps.map((step) => `- ${step.id}: ${step.description}`),
          ].join("\n"),
    );
    return report.status === "blocked" ? 1 : 0;
  }

  context.stdout(usage());
  return 2;
}

const entrypoint = process.argv[1];
if (
  entrypoint !== undefined &&
  import.meta.url === pathToFileURL(entrypoint).href
) {
  process.exitCode = runCli(process.argv.slice(2));
}
