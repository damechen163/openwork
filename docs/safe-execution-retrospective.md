# Safe-execution 里程碑全程回顾（备份）

> 2026-08-12 · 从"框架"到"真实 AI 任务闭环"的完整过程记录。
> 本文是工程日志，记录计划、决策、实施步骤、踩坑与修复，供后续备份与复盘。

---

## 1. 目标（requirement.md）

一周内唯一目标：**把 OpenWork 从"已经有框架"推进到"真的能跑一个 AI 任务"**。

四项优先任务：
1. 跑通真实 Sandbox：非 root、只读 rootfs、默认断网、无 Docker Socket、timeout/cancel/cleanup 正常、Ubuntu CI 真实验证
2. 跑通至少一个真实 Runtime：Claude Code 或 Codex 二选一，Prompt 走 stdin，Runtime 在 OpenWork 管理下执行，不允许 FakeSandbox 冒充
3. 接通 `openwork run`：最终执行
   `openwork run --runtime codex/claude-code --workspace ./samples/sales "..."`，
   真实生成 `sales-analysis.csv`、`summary.md`、Artifact SHA256、Audit 记录、最终状态 `SUCCEEDED`
4. 收尾 Vertical Slice PR：修 fmt/clippy/CI，合并代码，不开新大架构模块

**验收标准（唯一门）**：
> 在一台干净机器上，用户执行一次 `openwork run`，Claude Code 或 Codex 能通过 OpenWork 的 Sandbox 真正完成销售数据分析，并返回两个可下载的结果文件。

附加约束：不用 Docker，用 **podman**。

---

## 2. 现状探索（实施前摸清的地基）

### 2.1 已存在（冻结契约层，约 1 万行 Rust）

`openwork-execution` crate 是 "Frozen M1 contracts"，全部类型已实现并带校验：

| 契约 | 位置 | 说明 |
| --- | --- | --- |
| `SandboxBackend` trait | `execution/src/lib.rs:1036` | health/execute/cancel/cleanup，同步、Send+Sync |
| `SandboxRequest` | lib.rs:883 | 强制 digest-pinned 镜像（无 tag）、非 root 用户、network 仅 `Disabled`、input/output 目录必须不同 |
| `SandboxResult` | lib.rs:944 | termination 五态 + **exit_code 一致性校验**（TimedOut/Cancelled/OOM 必须 `None`） |
| `SandboxLimits` | lib.rs:711 | cpu 1-64000ms、mem 1MiB-64GiB、pid 1-4096、timeout 1-3600s、输出 ≤16MiB |
| 状态机 | lib.rs:330 | Queued→Planning→Running→Succeeded/Failed/Cancelled/TimedOut，revision CAS |
| ArtifactScanner | artifact/mod.rs | 流式 SHA-256、100MiB 上限、symlink/越界拒绝、size 漂移检测 |
| 审计链 | lib.rs:464 | 哈希链事件、脱敏、原子追加 |
| 执行状态机 | orchestrator/mod.rs | create_run（只存 prompt_sha256）/transition/record_artifacts，接 InMemory store |

### 2.2 完全缺失（本次要实现的）

- ❌ 任何 `SandboxBackend` 实现（全仓零 `impl`）
- ❌ `openwork run` 命令（CLI 只有 install/status/doctor/runtime）
- ❌ ClaudeRuntime/CodexRuntime 的 `run()`（返回 unsupported），`capabilities.run = false`
- ❌ `SystemCommandRunner` 的 stdin 支持（硬编码 `Stdio::null()`）
- ❌ `samples/` 目录
- ❌ Rust 工具链（本机未装）

### 2.3 环境事实

- Arch Linux on WSL2，root 用户，24 核
- podman 6.0.2（cgroup v2 ✓），rootful 可用
- claude 2.1.223 **已登录**（oauth_token，firstParty）→ 选中为 runtime
- codex 0.145.0 已登录（ChatGPT）→ 备选
- docker.io registry 可达（拉镜像无问题）
- rust-toolchain.toml 要求 Rust 1.95.0（edition 2024）

---

## 3. 架构决策

**核心矛盾**：冻结契约 `SandboxNetworkPolicy` 只有 `Disabled` 一种 —— 沙箱默认断网；而 AI 模型 API 需要网络。两者必须分离。

**用户拍板的编排方案（方案 A）**：
- **Runtime（Claude Code）在宿主侧跑**：有网络访问模型 API，按 prompt 协议只写分析脚本 `analyze.py`
- **分析计算在 podman 沙箱内执行**：断网/只读 rootfs/非 root，产出结果文件
- OpenWork 作为调度者串联两个阶段，全程走冻结状态机

其他方案（runtime 完全在容器内跑 = 要改冻结契约+违反 ADR-0007；agent 直接分析+沙箱复验 = 沙箱不参与核心分析）被否决。

**Runtime 选择**：Claude Code（已登录、可直接用；codex.rs 结构平行，后续可实现）。

---

## 4. 实施计划（8 步）

1. 环境：`pacman -S rustup` 装 1.95.0，基线编译
2. runtime crate 扩展：CommandSpec 加 stdin/capture_bytes + SystemCommandRunner 写 stdin + RuntimeRunRequest timeout
3. ClaudeRuntime::run 实现 + capabilities/manifest 翻 true
4. 新 crate `openwork-sandbox`：PodmanSandboxBackend + MockSandboxBackend + 单测 + 真实集成测试（#[ignore]）
5. CLI `run` 子命令 + run_loop 调度器 + 装配 + 调度器测试
6. samples/sales：sales.csv + README.md（任务契约）
7. **真机闭环验证（唯一验收门）**
8. CI sandbox job + fmt/clippy/test 全绿 + 文档

---

## 5. 实施过程与踩坑记录

### 5.1 环境准备

`pacman -S rustup` → `rustup default 1.95.0`（后台下载 ~5 分钟）。

### 5.2 runtime crate 扩展

- `CommandSpec` 加 `stdin: Option<String>` + `capture_bytes: usize`，用 `#[serde(skip_serializing_if, default)]` 保证 InstallPlan JSON 输出字节级不变（防止破坏 cli.rs 的 install dry-run 测试断言）
- `SystemCommandRunner` 改造：stdin 存在时 `Stdio::piped()` + 独立写线程（写完即 drop，避免大 prompt 与 stdout 管道互相阻塞死锁；broken pipe 忽略）
- `RuntimeRunRequest` 加 `timeout_seconds: Option<u64>`（默认 300s）

**坑 1：edition 2024 的 let 绑定 shadow 规则**。`let (backend, runner) = backend();` 在同一个函数内第二次出现时，RHS 的 `backend()` 解析到**新的 pattern 绑定**而不是函数（Rust 2024 变更），报 E0618。修复：测试辅助函数全部改名（`backend()` → `make_backend()` 等）。

**坑 2**：`read_stream` 硬编码 1MiB 捕获上限，与沙箱契约 16MiB 不符 —— 参数化 `capture_bytes` 后，runtime 既有测试里单参数调用 `read_stream` 全部编译失败（顺手修复）。

**坑 3**：`CommandSpec` 结构体字面量（installer/src/lib.rs:694 测试 fixture）加字段后编译失败，同步补字段；compatibility.rs 的 `RuntimeRunRequest` 字面量同样。

### 5.3 ClaudeRuntime::run

```rust
claude -p --output-format text --verbose --max-turns 60
       --allowedTools Write,Edit --permission-mode acceptEdits
```

- prompt 走 stdin（CommandSpec.with_stdin），cwd = working_directory
- 事件序列对齐 MockRuntime：Started → Output（redact_text 脱敏）→ Completed
- 映射：取消→RunCancelled、超时→RunTimedOut、非零退出→ExecutionFailed
- `cancel()`：登记 run 的 CancellationToken 并置位
- `capabilities()` / runtime/manifests/claude-code.json：`run: true, cancel: true`（已验证不破坏 manifest 测试）

### 5.4 openwork-sandbox crate

`PodmanSandboxBackend`：所有宿主操作（podman/chown/mkdir）走注入的 `CommandRunner`，单测用 FakeRunner 零真实副作用。

podman run 精确参数（安全基线全在）：

```
podman run --name openwork-run-<run_id>
  --network=none --read-only --tmpfs /tmp:rw,size=64m,mode=1777
  --user=1000:1000 --workdir /workspace
  --pids-limit N --memory N --cpus N --timeout N
  --env=PYTHONDONTWRITEBYTECODE=1
  -v <workspace>:/workspace:ro -v <output>:/workspace/output:rw
  docker.io/library/python@sha256:<pin>
  /usr/local/bin/python3 /workspace/analyze.py /workspace/output
```

执行流程：mkdir output → `chown -R 1000:1000`（前置）→ podman run → inspect 容器状态 → 映射 termination → 枚举产物 → `podman rm -f` → `chown -R 0:0`（恢复）。

**坑 4：契约 pin 镜像不允许 tag**。`python:3.12-slim@sha256:...` 非法（`valid_oci_segment` 不允许 `:`），必须 `docker.io/library/python@sha256:...`（digest 已唯一标识）。

**坑 5：chown 需要 `-R`**。集成测试第一次跑 happy path 报 `PermissionError` —— output 子目录没被重新归属（chown 只改了顶层）。

**坑 6（重要）：podman 6 超时语义变了**。`podman run --timeout=N` 超时杀容器后：客户端返回 **255**（不是文档/旧版的 124），且容器 `State.ExitCode = -1`（conmon 杀掉后状态不可用）。可靠映射：**总是 inspect**，用 `run_output.timed_out || exit==124 || (exit==255 && inspect_exit==-1)` 判定 TimedOut。

**坑 7：杀 podman client ≠ 停容器**。conmon 托管下容器继续存活 → timeout/cancel 必须补 `podman stop -t 2`，否则容器名冲突 + 资源泄漏。

**坑 8**：`RunId` 等 ID 类型没有 `Display`（容器名需要）→ 给契约宏 `uuid_v7_id!` 加 Display impl（纯增量，不改契约语义）。

**坑 9：podman 6 的 inspect 字段名变化**。`{{.ImageDigest}}` 不存在了，用 `podman images --digests --format "{{.Repository}}:{{.Tag}} {{.Digest}}"` 解析。

真实集成测试（3/3 通过，~23s）：
- happy path：脚本在隔离容器执行，产出两个文件，断言内容正确
- timeout：sleep 30 + timeout 5 → TimedOut
- OOM：bytearray(1GiB) + mem 64MiB → OutOfMemory

### 5.5 CLI run 子命令 + 调度器

- CLI 加 `Run` 子命令（--runtime/--workspace/--timeout/--sandbox-timeout/--json + prompt）
- **cli 需新增 lib target**（`src/lib.rs` re-export `run` 模块），否则集成测试无法引用 bin crate 的模块
- `run_loop` 纯函数调度器：前置检查（detect Healthy + capabilities.run + backend.health）→ create_run(Queued) → Planning → Running → runtime.run → 检查 analyze.py → 构造 SandboxRequest → backend.execute → 产物校验（两个约定文件）→ record_artifacts → Succeeded；失败路径映射 Failed/Cancelled/TimedOut
- Ctrl-C：ctrlc handler → token.cancel() + backend.cancel(run_id)（run_id 经回调注册）
- CancellationGuard 的 Drop 保证任何退出路径都 `backend.cleanup()`

**坑 10**：`ActorId` 非 Copy，多处 clone；`ArtifactScanner`/`ExecutionOrchestrator`/`InMemoryExecutionStore` 在子模块（`artifact::`/`orchestrator::`/`store::`）不在根；`artifacts`/`get_run` 是 `ExecutionStore` trait 方法需导入 trait；`ArtifactSizeBytes` 没有 `as_u64`（用 serde 序列化代替）。

**坑 11**：run_loop 的执行失败被折叠成状态而非 Err —— 测试一度用 `unwrap_err` 断言失败。修正：断言 `report.status`；CLI 按 status 映射退出码（Succeeded=0/Cancelled/TimedOut/其他）。

调度器测试 5/5（MockRuntime + MockSandboxBackend + 真实状态机）：
happy path（审计链尾 `RunCompleted`、Artifact SHA-256 与手工计算一致）、analyze.py 缺失→Failed、沙箱非零退出→Failed、超时→TimedOut、预取消→Cancelled。

### 5.6 samples/sales

- `sales.csv`：42 行真实销售数据（2026-07，date/region/product/units/revenue）
- `README.md`：任务契约（analyze.py 接口 `argv[1]`=输出目录、只写脚本不执行、只用标准库、确定性输出、精确文件命名）

### 5.7 真机闭环验证（唯一验收门）

第一次跑：`Run ...: Failed`，原因——**claude 非交互模式写文件 "awaiting approval"**（.claude/settings.json 的 write 权限没生效）。

**坑 12（关键）**：claude `-p` 非交互模式无法应答权限审批，settings.json 未自动生效 → 改为命令行参数 `--allowedTools "Write,Edit" --permission-mode acceptEdits`（bash 仍默认拒绝，安全语义保持）。验证后固化进 ClaudeRuntime::run。

最终结果：

```
$ openwork run --workspace /tmp/ow-demo/sales --timeout 300 \
    "Read README.md and implement analyze.py exactly per the task contract. Do not run any commands."
Run 019ff213-bbbf-7d82-ae55-0b0f06313585: Succeeded
artifact sales-analysis.csv text/csv     97 bytes  sha256:c3caced7...e126f90
artifact summary.md      text/markdown  299 bytes  sha256:ea1c68a4...a93eee4
```

产物内容与 Claude 独立核算的预期完全一致（总营收 $47,463.29、east 131/$13,845.00 等）。

---

## 6. 最终验证矩阵

| 项目 | 结果 |
| --- | --- |
| `cargo fmt --all --check` | ✅ |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅（pedantic 全绿，含新 crate） |
| `cargo test --workspace` | ✅ 30 组全过（含新增 5 个 run_loop 测试） |
| podman 真实集成测试（--ignored） | ✅ 3/3（happy path / timeout / OOM） |
| 真机闭环（真实 claude + podman） | ✅ Succeeded + 双产物 + SHA256 + 审计链 |
| 沙箱安全基线 | ✅ 非 root / 只读 rootfs / 断网 / 无 socket / limits / timeout / cancel / cleanup |

## 7. 交付物清单

**新文件**
- `crates/openwork-sandbox/`（Cargo.toml、src/lib.rs、src/mock.rs、tests/podman_integration.rs）
- `crates/openwork-cli/src/lib.rs`、`src/run.rs`、`tests/run_loop.rs`
- `samples/sales/sales.csv`、`samples/sales/README.md`
- `docs/demo/safe-execution.md`（验收步骤）、`docs/demo/safe-execution-retrospective.md`（本文）

**修改**
- `crates/openwork-runtime/src/{lib,claude,compatibility}.rs`（stdin/capture_bytes/run 实现）
- `runtime/manifests/claude-code.json`（run/cancel: true）
- `crates/openwork-execution/src/lib.rs`（uuid_v7_id! 宏加 Display，纯增量）
- `crates/openwork-cli/src/main.rs`（Run 子命令 + 装配 + 退出码映射）
- `Cargo.toml`（members + ctrlc）、`Cargo.lock`
- `.github/workflows/ci.yml`（sandbox-integration job：Ubuntu 装 podman 跑 `--ignored` 套件）
- `crates/openwork-installer/src/lib.rs`（fixture 补 CommandSpec 字段）

**Git**：`feat/run-safe-execution` 分支，commit a670614（未 push）。

## 8. 遗留事项与已知边界

- run 状态存 InMemoryExecutionStore，进程退出即失；Postgres 持久化是后续里程碑（store 注释已预留）
- CI 的 sandbox job 只验证 podman 沙箱；真实 claude 闭环需本机登录凭证，作为本地验收步骤文档化
- CodexRuntime::run 未实现（结构平行，工作量约半个 claude 的实现）
- `.claude/settings.json` 脚手架保留为防御（实际授权由 `--allowedTools` 参数保证）
- 沙箱镜像 digest 已 pin 进常量（2026-08-12 解析），上游 tag 移动时需复核
