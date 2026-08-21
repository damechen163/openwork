# OpenWork 部署指南

从零开始在一台 Linux 机器（含 WSL2）上部署 OpenWork：CLI 安全执行闭环 +
Admin Web 图形界面。目标：一次 `openwork run` 调用，真实 AI Runtime 在
隔离 Sandbox 中完成数据分析并产出可下载结果文件。

## 1. 环境要求

| 项 | 要求 |
| --- | --- |
| 操作系统 | Linux（Ubuntu 24.04 / Arch / WSL2 均可）；macOS 支持见 docs/platform-support.md |
| 架构 | x86_64 或 arm64 |
| 内存 | ≥ 8 GiB（推荐 16 GiB） |
| 磁盘 | ≥ 10 GiB 可用（Electron 二进制约 220 MB，python 镜像约 123 MB） |
| 网络 | 可访问 docker.io / claude.ai / github.com（拉镜像、装运行时） |
| 用户权限 | root 或可 sudo（rootful podman 需要） |

### 软件清单

| 软件 | 用途 | 安装命令（Arch） |
| --- | --- | --- |
| Rust ≥ 1.95 | 构建 CLI | `pacman -S rustup && rustup default 1.95.0`（或 rustup.rs 官方） |
| podman ≥ 6 | 沙箱运行时 | `pacman -S podman` |
| Claude Code | AI Runtime（登录后） | `curl -fsSL https://claude.ai/install.sh \| bash`（或 `openwork install --execute --yes --runtime claude-code`） |
| Node.js ≥ 20 | Admin Web | `pacman -S nodejs npm`（Electron 装包时自行下载其二进制） |
| Git | 拉源码 | `pacman -S git` |

## 2. 构建 openwork CLI

```bash
git clone https://github.com/damechen163/openwork.git
cd openwork
cargo build --release -p openwork-cli

# 验证
./target/release/openwork --version
./target/release/openwork status --json
./target/release/openwork doctor --json
```

卸载（可选）：`openwork install --dry-run` 查看会创建的目录。

## 3. 准备 Sandbox Runtime（podman）

1. 确认 podman 可用：

   ```bash
   podman version
   ```

2. 拉取分析用的 python 镜像（已 pin 的 digest 见
   `crates/openwork-sandbox/src/lib.rs` 的 `SANDBOX_IMAGE`）：

   ```bash
   podman pull docker.io/library/python:3.12-slim
   podman info
   ```

   沙箱在执行时以 `--user 1000:1000` 运行，rootful podman 需要在启动前由
   openwork 自动 `chown` 工作区（root 运行）；rootless podman 请以对应
   用户运行且保证 uid/gid 1000 可访问工作区。

3. 检查沙箱健康（CLI）：`openwork run --help`；集成自检
   （要求 podman + 网络可达镜像源）：

   ```bash
   cargo test -p openwork-sandbox --test podman_integration -- --ignored --test-threads 1
   ```

## 4. 安装并认证 Claude Code

```bash
# 推荐：官方安装脚本
curl -fsSL https://claude.ai/install.sh | bash

# 认证（打印模式无需交互）
claude auth status --json   # {"loggedIn": true, ...}
```

也支持 `openwork install --dry-run --json --runtime claude-code` 预览
OpenWork 管理的安装计划，再用 `openwork install --execute --yes --runtime claude-code --json`
执行（不会覆盖已有安装）。Codex 同构支持（`--runtime codex`）。

## 5. 运行真实任务（CLI）

```bash
# 使用仓库自带的销售样例（复制到临时目录，避免污染样例）
rm -rf /tmp/ow-demo && mkdir -p /tmp/ow-demo
cp -r samples/sales /tmp/ow-demo/

./target/release/openwork run \
  --workspace /tmp/ow-demo/sales \
  --timeout 300 \
  "Read README.md and implement analyze.py exactly per the task contract. Do not run any commands."
```

预期输出（节选）：

```
Run 019ff213-…: Succeeded
artifact sales-analysis.csv text/csv     97 bytes sha256:c3caced7…
artifact summary.md      text/markdown  299 bytes sha256:ea1c68a4…
```

产物位于 `<workspace>/output/`。全部结果以 `--json` 输出时可机器解析。

## 6. 部署 Admin Web 图形界面

```bash
cd apps/admin-web
npm install                 # 首次会下载 Electron 二进制（~220MB）
export OPENWORK_BIN=/绝对路径/to/target/release/openwork   # 不在 PATH 时必填
npm run dev                 # 开发模式（Vite + Electron，热重载）
# 或生产模式：
npm run build && npx electron .
```

运行需要图形环境：WSL2 用 WSLg（Windows 11 默认开），或任何 X/Wayland。
**root 用户**下 Electron 库依赖：

```bash
pacman -S gtk3 libcups libxcomposite libxdamage libxfixes libxrandr \
  libxkbcommon alsa-lib at-spi2-atk atk at-spi2-core mesa
```

五页覆盖全部 CLI 操作：Dashboard（状态）、Run Task（表单+实时进度+产物）、
Install（计划预览→确认→报告）、Doctor（诊断）、Runtimes（列表+详情）。

## 7. 验收清单

在干净机器上依次确认：

- [ ] `openwork run` 最终状态 `Succeeded`，退出码 0
- [ ] `<workspace>/output/sales-analysis.csv`、`summary.md` 存在且内容正确
- [ ] CLI 输出含两个产物及 SHA-256
- [ ] 执行期间沙箱容器 `--network=none --read-only --user 1000:1000`
      （`podman ps --all` 自检，或 `podman inspect openwork-run-<uuid>`）
- [ ] GUI（可选）：Run 页跑同一任务，进度三阶段流式显示，产物表带摘要

## 8. 故障排查

| 现象 | 原因 | 解决 |
| --- | --- | --- |
| `runtime not registered` | id 应为 `claude-code`（`claude` 仅在 openwork run 中归一化） | `openwork runtime list` 查看 |
| `Failed: analyze.py was not produced` | agent 没写脚本 | 检查 prompt/README 契约，超时加大 `--timeout` |
| `Failed: sandbox script exited with code N` | 脚本 bug | 报告中有 stderr |
| `SandboxUnavailable` | podman/chown 失败 | `podman version`；root 运行；检查工作区挂载 |
| `RunTimedOut` | 超时 | 加 `--timeout` / `--sandbox-timeout` |
| GUI `CLI offline` | 找不到 openwork 二进制 | 设 `OPENWORK_BIN` 并 Refresh |
| GUI 启动报 sandbox 错误 | root 运行 | 已自动 `--no-sandbox`；若手动启动需 `ELECTRON_DISABLE_SANDBOX=1` |
