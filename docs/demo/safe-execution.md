# Living acceptance demo

In addition to the deterministic evidence above, the real-host acceptance gate
for `openwork run` is documented here:

## Recorded real-host outcome (verified 2026-08-12 on WSL2)

```
Run 019ff213-...: Succeeded
artifact sales-analysis.csv text/csv     97 bytes sha256:c3caced7...
artifact summary.md      text/markdown  299 bytes sha256:ea1c68a4...
```

Executed once via `openwork run --workspace /tmp/ow-demo/sales --timeout 300 "<prompt>"`.

## Reproduce

See the CLI demo steps and the GUI acceptance path in the following sections.

## Failure modes

| Symptom | Cause | Fix |
| --- | --- | --- |
| `Failed: analyze.py was not produced` | runtime wrote no script | review the prompt/README contract |
| `Failed: sandbox script exited with code N` | script bug | check `stderr` in the report |
| `TimedOut` | runtime or sandbox budget exhausted | raise `--timeout` / `--sandbox-timeout` |
| `SandboxUnavailable` at pre-flight | podman/chown problem | check `podman --version`, run as root or a rootless podman user |
 2ddb29f (feat(run): execute AI tasks through podman sandbox)
