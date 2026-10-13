# CHANGELOG

## [Unreleased]

### Added

- 新增 `src/cli`：Rust CLI 骨架（命令 `qtcloud-feishu`，clap 解析）
- `src/cli` 按「基础单元 + 串联器 + 执行器」落地三个子命令：`units` / `unit`、`definitions` / `define`、
  `flow status` / `flow run`；流程与定义是外部 YAML 配置，落盘契约与实验室一致

## [0.0.1] - 2026-10-11

初始版本。
