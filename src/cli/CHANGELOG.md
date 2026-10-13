# CHANGELOG

## [Unreleased]

### Added

- 基础单元第 14 个动作 `im_chat_link`（`im chats link`，取群分享链接）：`validity_period` 走
  `--data '{"validity_period": "…"}'`（`week` / `year` / `permanently`）。边界（实测）：
  调用者得是群成员；**外部群机器人身份取不到**（API 232033），要么用用户身份。
  判据加两条：按登记拼出命令行、要永久有效时假 lark-cli 跟着回永久那份。
- 端到端加一条：两步定义「先取群链接（`as: 链接`）→ 发信（正文里填 `{{链接.share_link}}`）」，
  用假 lark-cli 记下的 argv 证明链接真的填进了正文。

- `src/paths.rs`：本机落盘位置按 XDG 基目录规范取——镜像（`raw/` 与 `cursor/`）落
  `<XDG_DATA_HOME>/qtcloud-feishu/`、流程状态落 `<XDG_STATE_HOME>/qtcloud-feishu/<名字>.json`、
  定义与流程落 `<XDG_CONFIG_HOME>/qtcloud-feishu/<名字>.yaml`、可重建产物落 `<XDG_CACHE_HOME>/qtcloud-feishu/`。
  变量没设或为空用缺省、给了相对路径当没设、`HOME` 也缺就报错（不猜）。判据 `tests/paths.rs` 七项；
  后端加 `Backend::with_mirror()`（不给 `store_root` 仍是不落盘的纯读）。

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

- 样例 `examples/qtrecurit.rs`：流程 YAML 提出独立文件 `examples/pipelines/send-invite-email.yaml`，源码改用
  `include_str!` 嵌入（编译期常量，行为不变）；YAML 现可直接编辑、高亮、diff，仍与实验室
  `tests/fixtures/flows/recruitment.yaml` 同步拷贝。

- `README.md` / `docs/index.md` 退回你自己的原文——此前我改写过一版，描述的是已被撤下的命令面；
  README 另补一节「落盘位置（XDG）」。

- 样例 `examples/qtrecurit.rs`：动作可以先「先取」一样东西再执行——发群二维码之前先取群邀请链接，
  按 `{{邀请链接}}` 填进邮件正文模板（模板仍在样例的流程里）。契约加第四种能力 `chat_link`
  （mock 给确定性链接），新增一条测试钉住"正文里带上了链接"。
  两条已知边界：真取链接要**用户身份**（本机 user token 缺失，需 `lark-cli auth login`）；
  且**外部群机器人身份取不到**（API 232033）——实训基地群是外部群。

- **测试全部搬出 `src/`**：`src/` 只放实现，六份测试与假 lark-cli、夹具都在 `tests/`
  （`tests/support/mod.rs` 是共用的假 CLI 与夹具，测试自己写到临时目录；argv 由假 CLI 记日志，
  不为实现开测试口子）。为此加了库目标 `src/lib.rs`——集成测试只能通过库调实现；
  `src/main.rs` 仍是骨架（命令面未定，命令行入口暂不接库）。

- **命令面撤下**：`main.rs` 回到骨架（子命令未定义），不暴露 `unit` / `define` / `flow` 之类子命令——
  命令面等形状定了再接。基础单元、串联器、执行器、落盘、后端都留在 `src/`，端到端测试收进
  `src/e2e.rs`（真进程 + 假 lark-cli），不经过命令行。
- 退役 `examples/qtrecurit.rs`：流程调度逻辑从样例提炼进 `src/`，样例不再自带一份内嵌流程 YAML
  与能力层；跨语言对照改为「同一份配置，实验室（Python）与本 CLI（Rust）跑同一批夹具」
- `chrono` / `serde` / `serde_json` / `serde_yaml` 由开发依赖转为运行依赖（CLI 现在自己读配置）；
  新增 `ureq`（按 lark-cli 给的 download_url 取附件字节）

## [0.0.1] - 2026-10-11

初始版本。
