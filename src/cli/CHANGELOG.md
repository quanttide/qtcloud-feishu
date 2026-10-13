# CHANGELOG

## [Unreleased]

### Added

- **基础单元**（`src/resources/`）：六资源十三个动作按飞书资源分文件（`mail.rs` / `im.rs` /
  `vc.rs` / `task.rs` / `doc.rs` / `base.rs`），动作登记就是契约——命令、参数、必填、落盘名各写一处，
  后端按名字一次派发，不逐一写方法
- **串联器**（`src/engine.rs`）：读定义（YAML），把定义展开成一串基础单元的活动——参数模板
  （含「上一步的输出喂给下一步」）、按「出」表把产出起成我们的话、增量游标
- **执行器**（`src/flow.rs`）：读流程与定义，按状态机推格——检查成立才推、动作先做失败不挪格、
  人 / 半自动只记待办、状态可从飞书侧重建
- **落盘契约**（`src/store.rs`）：`raw/<资源>/<动作>/<页序>.json` 原样响应、附件原件、游标
- **命令行三个子命令**：`units` / `unit`（含 `--page-all` 与 `--cursor` 的受控拉取）、
  `definitions` / `define`、`flow status` / `flow run`
- 测试：15 项单元测试（登记拼命令行、模板选择器、增量、状态机三条规矩）+ 9 项端到端测试
  （真二进制 + 测试自己生成的假 lark-cli 与夹具）

### Changed

- 退役 `examples/qtrecurit.rs`：流程调度逻辑从样例提炼进 `src/`，样例不再自带一份内嵌流程 YAML
  与能力层；跨语言对照改为「同一份配置，实验室（Python）与本 CLI（Rust）跑同一批夹具」
- `chrono` / `serde` / `serde_json` / `serde_yaml` 由开发依赖转为运行依赖（CLI 现在自己读配置）；
  新增 `ureq`（按 lark-cli 给的 download_url 取附件字节）

## [0.0.1] - 2026-10-11

初始版本。
