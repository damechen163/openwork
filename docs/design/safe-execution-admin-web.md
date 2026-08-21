# Safe Execution 闭环与 Admin Web GUI 设计

> 设计记录：M1 "Safe Execution" 里程碑（`openwork run` 真实 AI 任务闭环）
> 与 `apps/admin-web` Electron 图形界面。状态：**已实现并经真机验证**。

## 1. 目标与背景

仓库原有 Bootstrap Runtime 里程碑（安装器 CLI + 治理契约）已经冻结了 M1
安全执行契约（`openwork-execution`："Frozen M1 contracts for safe task execution"），
但没有任何实现。本设计与实现的目标（一周验收标准，见 requirement.md）：

> 在一台干净机器上，用户执行一次 `openwork run`，Claude Code 或 Codex 能通过
> OpenWork 的 Sandbox 真正完成销售数据分析，并返回两个可下载的结果文件。

真机验收结果：`Run 019ff213…: Succeeded`，产出 `sales-analysis.csv` + `summary.md`，
均有 SHA-256 入库，审计链完整。

## 2. 整体架构

三层结构，严格的边界方向（ADR-0017：契约不依赖实现，实现向内依赖契约）：

```
┌─────────────────────────────────────────────────────────────┐
│ 展示层  apps/admin-web/（Electron + React + TypeScript）      │
│   Router → 5 页面 → api.ts（typed IPC）→ preload → main 进程  │
│   main 进程 spawn openwork CLI（--json），run 输出流式转发     │
└──────────────────────────┬──────────────────────────────────┘
                           │ spawn
┌──────────────────────────▼──────────────────────────────────┐
│ 执行层  crates/openwork-cli/src/run.rs（run_loop 调度器）     │
│   状态机：Queued→Planning→Running→Succeeded/Failed/TimedOut/ │
│           Cancelled（revision CAS + 单调时间戳）              │
│   runtime 宿主侧执行（有网络）→ 沙箱容器执行（断网）→ 产物/sha  │
└───┬──────────────┬──────────────────────────┬───────────────┘
    │              │                          │
┌───▼──────────┐ ┌─▼──────────────────┐ ┌─────▼──────────────┐
│ 契约层       │ │ openwork-runtime   │ │ openwork-sandbox   │
│ openwork-    │ │ ClaudeRuntime::run │ │ PodmanSandboxBacked│
│ execution    │ │ （stdin prompt）    │ │ （新建 crate）      │
│ （冻结契约）  │ │ cancel()           │ │ MockSandboxBackend │
└──────────────┘ └────────────────────┘ └────────────────────┘
```

**核心分工原则**：AI Runtime 需要网络访问模型 API，沙箱则必须默认断网
（契约 `SandboxNetworkPolicy` 只有 `Disabled`）。因此：

- **Runtime（宿主侧，有网络）**：Claude Code 在 workspace 中分析数据集，
  按任务契约编写 `analyze.py` 脚本；经 `.claude/settings.json` 脚手架约束
  只允许 Write/Edit、拒绝 Bash，**agent 只能写脚本、无法执行命令**。
- **Sandbox（容器内，断网）**：OpenWork 将脚本与数据挂载进一次性 podman
  容器，真实执行分析并产出结果文件。
- **记录（宿主侧）**：产物经 ArtifactScanner 流式 SHA-256 → store → 审计链。

## 3. Safe Execution 闭环

### 3.1 安全基线（ADR-0007 落地）

| 契约要求 | 实现 |
| --- | --- |
| 非 root | `--user 1000:1000`，rootful podman 前置 `chown -R` |
| 只读 rootfs | `--read-only` + `--tmpfs /tmp:rw,size=64m,mode=1777` |
| 默认断网 | `--network=none` |
| 无 Docker socket | 不挂任何 socket；无 `-v /var/run/...` |
| 资源限制 | `--pids-limit --memory --cpus --timeout`（契约值直通） |
| 超时/取消/清理 | podman `--timeout`（双保险）+ 宿主 watchdog；补 `podman stop -t 2`；`podman rm -f` 幂等 cleanup |
| 镜像 pin | `DigestPinnedImageRef`：必须 `@sha256:` 全量 pin，拒绝 tag |
| 挂载边界 | `ApprovedMountDirectory::under_root()` canonicalize 证明在批准根下 |

### 3.2 状态机（契约冻结）

```
Queued → Planning → Running → Succeeded
                         │→ Failed / Cancelled / TimedOut
```
- 每次 transition 用上一轮的 `revision` 做 CAS，时间戳 `UtcTimestamp::now()` 单调。
- 审计事件由 store 原子附带：`RuntimeSelected / RuntimeStarted / ArtifactCreated /
  RunCompleted / RunFailed`。

### 3.3 关键实现决策与踩坑

| # | 决策/坑 | 依据 |
| --- | --- | --- |
| 1 | podman 6.x 超时 kill：client 返回 **255**（非旧版 124），容器 `inspect` 得 `State.ExitCode = -1`；映射 TimedOut 时 **exit_code 必须 None**（`SandboxResult::validate()` 强制一致性） | 真机实测 |
| 2 | 只杀 podman client **不会停容器**（conmon 托管）→ 超时/取消必须补 `podman stop -t 2` | conmon 语义 |
| 3 | chown 必须 `-R`（否则输出子目录留在 root 名下，沙箱内 PermissionError） | 真机实测 |
| 4 | `claude -p` 非交互无法应答审批 → 显式 `--allowedTools Write,Edit --permission-mode acceptEdits`；**不用** `--dangerously-skip-permissions`（它会放开 Bash） | 非交互语义 |
| 5 | 沙箱输出上限：runner 原硬编码 1MiB 截断 → `CommandSpec.capture_bytes`（沙箱 16MiB 契约界限） | 契约上限 |
| 6 | `ApprovedMountDirectory::under_root` 拒绝 `canonical == root` → approved root 必须是 workspace **父目录** | 契约不变量 |
| 7 | `CommandSpec` 新字段 serde 兼容：`skip_serializing_if + default`（InstallPlan JSON 输出字节级不变） | 兼容性 |
| 8 | 进度回调 `on_progress(&str)` 走 **stderr** 行（`[openwork] phase…`），`--json` 输出保持机器可解析 | JSON 语义 |

### 3.4 真机验证（本机 WSL2 + podman 6 + claude 2.1.223）

- 集成测试 `podman_integration.rs`（`#[ignore]`，CI 显式跑）：**happy path / timeout / OOM 全过**。
- CLI 调度器测试 `run_loop.rs`：5 个场景（成功/缺脚本/非零退出/超时/预取消）全过。
- 全过程：`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --check`、`cargo test --workspace` 全绿。

## 4. Admin Web GUI

### 4.1 技术选型

Electron + React 19 + TypeScript + Vite + vite-plugin-electron（无 UI 框架，手写深色 admin 主题）。

### 4.2 安全边界

- `contextIsolation: true`、`nodeIntegration: false`、`sandbox: false`（仅因 WSL2 root 运行）。
- renderer 唯一能力是 preload 暴露的 `window.openwork.*`（typed IPC），主进程持有
  `child_process.spawn` 全部 CLI 交互；`OPENWORK_BIN` 环境变量覆盖 PATH 查找。
- 无任何 renderer 端文件系统访问。

### 4.3 页面清单

| 页面 | CLI 命令 | 交互 |
| --- | --- | --- |
| Dashboard | `status --json` | 安装状态/平台/运行时健康度卡片 |
| Run Task | `run --json` | 表单 → **实时三阶段进度终端**（runtime→sandbox→recording）→ 产物表（路径/大小/SHA-256）+ 取消按钮 |
| Install | `install --dry-run` → `--execute --yes` | 计划预览 → 显式确认 → 执行报告 |
| Doctor | `doctor --json` | PASS/WARN/FAIL/SKIP + remediation |
| Runtimes | `runtime list/info --json` | 检测状态/版本/认证/capabilities 详情 |

### 4.4 GUI 关键踩坑

| # | 坑 | 解决 |
| --- | --- | --- |
| 1 | Electron 二进制 postinstall 下载失败（HTTP2 超时） | VPN/网络恢复后重试 `node node_modules/electron/install.js` |
| 2 | root 下 Chromium sandbox 拒绝启动 | `app.commandLine.appendSwitch('no-sandbox')` + `ELECTRON_DISABLE_SANDBOX=1` |
| 3 | WSL2 缺 GTK/X 库 13 个 | pacman 装 `gtk3 libcups libxcomposite ... mesa` |
| 4 | `type: module` 下 preload 报 `require is not defined` | preload 构建为 **CJS**（`.cjs`），ESM preload 有坑 |
| 5 | `ELECTRON_RUN_AS_NODE=1` 残留导致以 Node 模式运行 | 运行时 unset |
| 6 | 验证脚本轮询条件错误导致"报告未渲染"假象 | 假 IPC 快速复验：报告卡 + 产物行渲染正确 |

## 5. 文件清单（本次新增/修改）

```
crates/
├─ openwork-sandbox/            [新增 crate]
│  ├─ src/lib.rs                PodmanSandboxBackend + 单测
│  ├─ src/mock.rs               MockSandboxBackend
│  └─ tests/podman_integration.rs  [ignore] 真实容器集成
├─ openwork-runtime/src/lib.rs  CommandSpec.stdin/capture_bytes；RuntimeRunRequest.timeout
├─ openwork-runtime/src/claude.rs  run()/cancel() 实现；capabilities 翻 true
├─ openwork-cli/src/run.rs      [新增] run_loop 调度器
├─ openwork-cli/src/main.rs     run 子命令 + 装配 + 进度
└─ openwork-execution/src/lib.rs  ID 类型加 Display
apps/admin-web/                 [新增] Electron GUI（见 apps/admin-web/README.md）
samples/sales/                  [新增] 任务样例（sales.csv + 任务契约 README）
docs/demo/safe-execution.md     [新增] 演示步骤与验收清单
.github/workflows/ci.yml        [新增] sandbox-integration job
```

## 6. 遗留与后续

- 运行状态在内存 store（`store/mod.rs` 注释 "memory now Postgres later"）。
- CI 无法跑真实 claude（无登录凭证），闭环验收为真机可复现步骤（docs/demo/safe-execution.md）。
- 后续里程碑：control-api（-API 服务化）、Postgres 持久化、Codex run() 等价打通
  （脚本相同结构，适配器已具备）。
