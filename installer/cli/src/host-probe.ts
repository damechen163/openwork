import { spawnSync } from "node:child_process";
import { arch, cpus, platform, totalmem } from "node:os";
import { statfsSync } from "node:fs";

export interface HostProbe {
  readonly architecture: () => string;
  readonly composeVersion: () => string | null;
  readonly cpuCount: () => number;
  readonly diskFreeBytes: () => bigint;
  readonly dockerVersion: () => string | null;
  readonly memoryBytes: () => number;
  readonly operatingSystem: () => NodeJS.Platform;
}

function commandVersion(
  command: string,
  args: readonly string[],
): string | null {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
    timeout: 5_000,
  });

  if (result.status !== 0) {
    return null;
  }

  const output = result.stdout.trim();
  return output.length > 0 ? output : null;
}

export const systemHostProbe: HostProbe = {
  architecture: arch,
  composeVersion: () => commandVersion("docker", ["compose", "version"]),
  cpuCount: () => cpus().length,
  diskFreeBytes: () => {
    const statistics = statfsSync(process.cwd(), { bigint: true });
    return statistics.bavail * statistics.bsize;
  },
  dockerVersion: () => commandVersion("docker", ["--version"]),
  memoryBytes: totalmem,
  operatingSystem: platform,
};
