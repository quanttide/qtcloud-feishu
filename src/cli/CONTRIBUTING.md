# 贡献指南

本文件记录 `src/cli` 的开发方法：**实验室先跑通，配置与执行器分层落位**。

## 方法

分三步，顺序固定。

### 实验室先跑通

逻辑先在量潮飞书适配仓库的实验室（`examples/quanttide-feishu-lab`）用 Python 验证：流程与定义
写成 YAML、数据来自夹具、`tests/smoke.py` 逐项断言输出。实验室没跑通的逻辑不进本仓库。

### 配置与执行器分层

跑通的逻辑分三处落位，各有各的话：

| 层 | 落哪 | 例 |
| --- | --- | --- |
| 基础单元 | `src/resources/`（本仓库，Rust） | `im_chat_members` = `im +chat-members-list` |
| 定义（我们的话 → 一串飞书活动） | 场景侧的 YAML，本仓库不留拷贝 | `看谁进群了` = 一步 `im_chat_members` |
| 流程（状态、迁移、检查、动作） | 场景侧的 YAML，本仓库不留拷贝 | 检查写 `定义: 看谁进群了` |

本仓库只认属于平台的那部分：**基础单元进 `src/`，我们的话留在场景侧**。招聘的定义与流程文件在
实验室（`examples/quanttide-feishu-lab/examples/qtrecurit/`），本仓库不复刻一份——配置只有一份，
改一处两边都变。

### 共用代码才提炼

同一段代码出现第二个调用方才搬进 `src/`。提炼只搬代码与数据，不改行为：搬完实跑，输出不变。
`examples/qtrecurit.rs` 曾是流程调度的可执行规格（自带内嵌流程 YAML 与能力层）；三个子命令落地后
它的逻辑已进 `src/`（`engine.rs` / `flow.rs`），样例随之退役——先有样例，再有模块，不预先设计抽象。

## 跑一遍

本仓库不再自带夹具与流程配置，跑的时候指到实验室那一侧：

```bash
LAB=../../../examples/quanttide-feishu-lab

# 基础单元：走实验室的假 lark-cli（离线），每页落盘到 /tmp/mirror
cargo run -- --lark-cli $LAB/tests/fake_lark_cli/bin/lark-cli --store /tmp/mirror \
  unit mail_list -p folder=收件箱 -p mailbox=hr@example.com

# 定义：三步的定义文件就是实验室那份
cargo run -- define 读投递邮件 --definitions $LAB/examples/qtrecurit/定义.yaml \
  -p 邮箱=hr@example.com -p 文件夹=收件箱

# 流程：一轮推到底
cargo run -- --lark-cli $LAB/tests/fake_lark_cli/bin/lark-cli --store /tmp/mirror \
  flow run --flow $LAB/examples/qtrecurit/recruitment.yaml \
  --definitions $LAB/examples/qtrecurit/定义.yaml --state /tmp/state.json --today 2026-10-13
```

## 跨语言对照

同一份流程与定义，实验室（Python）与本 CLI（Rust）跑同一批夹具，推进序列与状态文件应当一致：

```bash
cd $LAB && rm -f /tmp/py-state.json
PYTHONPATH=src .venv/bin/python examples/qtrecurit/run.py run --mock --state /tmp/py-state.json
cd -

cargo run -- --lark-cli $LAB/tests/fake_lark_cli/bin/lark-cli --store /tmp/mirror \
  flow run --flow $LAB/examples/qtrecurit/recruitment.yaml \
  --definitions $LAB/examples/qtrecurit/定义.yaml --state /tmp/rust-state.json --today 2026-10-13

diff /tmp/py-state.json /tmp/rust-state.json    # 应当没有差异
```

`--today` 给同一天，两边的 `since` 才比得上；`message_id` 由 lark-cli 决定（同一个假 CLI），
两边一致。

## 提交前

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --all-targets`（单元测试与 `tests/cli.rs` 的端到端一并跑）
4. 跨语言对照跑一遍，状态文件对得上
5. 变更记入 `CHANGELOG.md` 的 `[Unreleased]`
