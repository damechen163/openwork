# OpenWork

面向中小企业的开源 AI 工作环境安装系统。

一家公司安装一次，全体员工即可获得统一的私有 AI 助手，并由身份、权限、
策略和审批决定它能读取什么、调用什么以及执行什么。

[English](README.md) · [快速开始](docs/getting-started.md) ·
[为客户部署](docs/deploy-for-client.md) · [开发 Capability Pack](docs/packs/build-your-first-pack.md)

> 当前状态：`v0.1.0-alpha.0` Phase 0。安装器仅实现 `version`、`doctor` 和
> 不产生变更的 `install --dry-run`，尚不会启动正式服务。

## 员工未来可以完成

- 在授权范围内查询企业知识；
- 在隔离 Sandbox 内分析表格、生成文档；
- 使用只读凭证分析允许访问的业务数据；
- 仅在策略允许或审批通过时调用业务工具。

## 为 AI 实施商设计

- 一套部署服务一家企业；
- 通过版本化 Capability Pack 与 Adapter 扩展，不侵入核心；
- 以统一方式诊断、备份、升级、回滚和售后。

Community 核心采用 Apache-2.0，允许提供商业实施服务，但必须同时遵守第三方
组件许可证。详见[许可证说明](docs/licensing.md)。

## Phase 0 开发验证

```bash
corepack enable
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm test
pnpm test:integration
pnpm build
node installer/cli/dist/cli.js doctor --json
node installer/cli/dist/cli.js install --dry-run --json
```

当前安装主机契约仅支持 Linux `amd64`/`arm64`。硬件门槛将在公开 benchmark 后确定。
