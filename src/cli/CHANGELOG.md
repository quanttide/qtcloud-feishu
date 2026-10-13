# CHANGELOG

## [Unreleased]

### Added

- 初始化 CLI 骨架：`qtcloud-feishu` 命令，clap 参数解析与帮助输出。
- 新增示例 `qtrecurit`：量潮招聘流程调度器（实验室逻辑的 Rust 重写）。
- 提炼 `mock` 能力模块（`src/mock.rs`）：示例与后续命令共用的夹具实现，配套库目标 `src/lib.rs`。
- 新增 `CONTRIBUTING.md`：样例驱动的开发方法。

### Changed

- 能力层并回样例：删掉库目标 `src/lib.rs` 与契约 / 夹具模块 `src/mail_adapter.rs`、`src/mock.rs`，
  改为 `examples/qtrecurit.rs` 的内联模块 `mail_adapter` / `mock`，示例自包含；行为与跨语言对照不动。
- 重写 `docs/index.md`：CLI 文档入口，写明目标、设计哲学落点与形态差距。
- Mail 适配器化：契约模块再改名 mail → `src/mail_adapter.rs`，trait `Capabilities` →
  `MailAdapter`；行为与跨语言对照不动。
- 能力层重构：契约抽为 `Capabilities` trait（`src/mail.rs`，由 capabilities 改名而来），`Mock` 改为构造注入
  （`Mock::default()` 保实验室夹具），`send_mail` 返回 `Result` 并留存已发记录；示例签名收
  `&impl Capabilities`，补「动作失败不挪格」「迟到追认」两条测试。
