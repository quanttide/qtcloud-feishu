# CHANGELOG

## [Unreleased]

### Added

- 新增 CLI 与 Studio 两份 ROADMAP（`src/<端>/ROADMAP.md`，与 README 并列），含带日期的设计偏差反思
- CLI 迁入邮件发送通道（qtcloud-connect 迁入方案第 5 步的库能力）：新增 `mail_draft_create` / `mail_draft_send` 两个动作，`mail_send` 与起草均支持 `body_file`；动作共十六个
- `Backend` 加操作日志：写动作成功后往 `<state>/qtcloud-feishu/ops.jsonl` 追加一条（只记元数据，正文类参数排除；写失败报错并给出待补记条目，失败动作不写日志）
- `Backend` 加超时（缺省三十秒，`LARK_CLI_TIMEOUT` 可覆盖）与 `plan`（dry-run：只拼命令行，不跑子进程、不落盘、不写日志）
- 新增 `src/studio`：Flutter 工作台骨架（惯例同 qtcloud-connect），与 CLI 共用文件契约；起点页为待确认队列（数据源未接，按钮禁用）
- 新增 `src/cli`：Rust CLI 骨架（命令 `qtcloud-feishu`，clap 解析）
- `src/cli` 按「基础单元 + 串联器 + 执行器」落地三个子命令：`units` / `unit`、`definitions` / `define`、
  `flow status` / `flow run`；流程与定义是外部 YAML 配置，落盘契约与实验室一致

## [0.0.1] - 2026-10-11

初始版本。
