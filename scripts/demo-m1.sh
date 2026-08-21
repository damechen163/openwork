#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)
engine_bin=${OPENWORK_DOCKER_BIN:-}
if [ -z "$engine_bin" ]; then
  engine_bin=$(command -v docker || true)
fi
case "$engine_bin" in
  /*) ;;
  *)
    echo "Docker-compatible CLI not found; set OPENWORK_DOCKER_BIN to an absolute path." >&2
    exit 2
    ;;
esac

sales_image=${OPENWORK_SALES_SANDBOX_IMAGE:-docker.io/library/busybox@sha256:9db7b59979c38555a39def84a31fb98b5296952f9e3afd4f6f11f05b07adfab0}
output_root=${OPENWORK_DEMO_OUTPUT_ROOT:-$repo_dir/openwork-demo-output}

cd "$repo_dir"
cargo build -p openwork-cli --locked

doctor_exit=0
target/debug/openwork doctor --json || doctor_exit=$?
if [ "$doctor_exit" -ne 0 ] && [ "$doctor_exit" -ne 11 ]; then
  echo "OpenWork doctor failed; fix the reported host issue before running the demo." >&2
  exit "$doctor_exit"
fi

target/debug/openwork demo sales \
  --engine-bin "$engine_bin" \
  --image "$sales_image" \
  --output-root "$output_root" \
  --json

# This deterministic test exercises L0-L4 policy, exact approval, single-use
# ActionClaim, MockActionExecutor, tamper/replay denial, and the audit hash chain.
# It never sends an email or connects to an external service.
cargo test -p openwork-e2e --test control_plane --locked -- --nocapture

echo "M1 demo complete. No external email was sent; the L3 action used MockActionExecutor."
