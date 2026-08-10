import type { DoctorCheck, DoctorReport } from "./types.js";
import type { HostProbe } from "./host-probe.js";

const GIBIBYTE = 1024 ** 3;

function humanBytes(bytes: bigint | number): string {
  return `${(Number(bytes) / GIBIBYTE).toFixed(1)} GiB`;
}

export function runDoctor(
  probe: HostProbe,
  now: () => Date = () => new Date(),
): DoctorReport {
  const operatingSystem = probe.operatingSystem();
  const architecture = probe.architecture();
  const dockerVersion = probe.dockerVersion();
  const composeVersion = probe.composeVersion();

  const checks: DoctorCheck[] = [
    operatingSystem === "linux"
      ? {
          id: "host.os",
          message: "Supported host operating system detected.",
          status: "pass",
          value: operatingSystem,
        }
      : {
          id: "host.os",
          message: "This host is not supported for installation.",
          remediation:
            "Run the installer on a supported Linux host. Development commands may run elsewhere.",
          status: "fail",
          value: operatingSystem,
        },
    architecture === "x64" || architecture === "arm64"
      ? {
          id: "host.arch",
          message: "Supported CPU architecture detected.",
          status: "pass",
          value: architecture,
        }
      : {
          id: "host.arch",
          message: "Unsupported CPU architecture.",
          remediation: "Use a linux/amd64 or linux/arm64 host.",
          status: "fail",
          value: architecture,
        },
    {
      id: "host.cpu",
      message:
        "CPU capacity recorded; release thresholds remain benchmark-driven.",
      status: "pass",
      value: `${probe.cpuCount()} logical CPUs`,
    },
    {
      id: "host.memory",
      message:
        "Memory capacity recorded; release thresholds remain benchmark-driven.",
      status: "pass",
      value: humanBytes(probe.memoryBytes()),
    },
    {
      id: "host.disk",
      message:
        "Free disk capacity recorded; release thresholds remain benchmark-driven.",
      status: "pass",
      value: humanBytes(probe.diskFreeBytes()),
    },
    dockerVersion === null
      ? {
          id: "docker.engine",
          message: "Docker Engine is unavailable.",
          remediation:
            "Install and start Docker Engine, then rerun openwork doctor.",
          status: "fail",
        }
      : {
          id: "docker.engine",
          message: "Docker Engine is available.",
          status: "pass",
          value: dockerVersion,
        },
    composeVersion === null
      ? {
          id: "docker.compose",
          message: "Docker Compose is unavailable.",
          remediation:
            "Install the Docker Compose plugin, then rerun openwork doctor.",
          status: "fail",
        }
      : {
          id: "docker.compose",
          message: "Docker Compose is available.",
          status: "pass",
          value: composeVersion,
        },
  ];

  const status = checks.some((check) => check.status === "fail")
    ? "blocked"
    : checks.some((check) => check.status === "warn")
      ? "degraded"
      : "healthy";

  return {
    checks,
    generatedAt: now().toISOString(),
    schemaVersion: "openwork.doctor/v1",
    status,
  };
}
