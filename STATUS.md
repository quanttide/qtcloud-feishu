# STATUS

仓库当前快照，供 Agent 与协作者快速了解现状。快照有时效，以最新提交为准。

> 快照日期：2026-10-29
> 最新 commit：b1eb417

## 仓库形态

飞书适配的可部署应用，挂在适配仓 `quanttide-feishu/apps/` 下。仓库只有一个 crate：

| 路径 | 说明 |
|------|------|
| `src/cli/` | 命令行工具（Rust，crate `qtcloud-feishu-cli`，命令 `qtcloud-feishu`） |
| `src/studio/` | 工作台（Flutter，web 静态产物） |

## Studio 进度

骨架刚立（2026-10-29）：`flutter create` 工程 + 起点页待确认队列（`lib/screens/draft_queue.dart`），数据源未接。

- 定位：与 CLI 共用同一套文件契约（资源登记、流程定义、落盘、游标），CLI 执行落盘、Studio 看与确认；不实现第二套飞书调用
- 起点页选待确认队列：草稿确认制是唯一需人参与才闭环的界面，且只消费落盘草稿记录，契约最窄；按钮先禁用，写回随落盘契约一起定
- 后续界面方向：流程看板（推格状态）、能力目录（从登记生成表单）
- 验证：`flutter analyze` / `flutter test` / `flutter build web --release --base-href /`

## CLI 进度

**命令面未定义**：`main.rs` 保持骨架，子命令等形状定形后再接。库目标先行为测试与将来的命令面服务。

### 骨架（已成形）

基础单元 + 串联器 + 执行器：

| 模块 | 职责 |
|------|------|
| `src/resources/` | 基础单元：六资源十四个动作，登记即契约，按飞书资源分文件（action/base/doc/im/mail/task/vc） |
| `engine.rs` | 串联器：读定义，把定义展开成一串基础单元的活动 |
| `flow.rs` | 执行器：读流程与定义推格（检查成立才推、动作先做失败不挪格） |
| `store.rs` | 落盘契约：`raw/<资源>/<动作>/<页序>.json` + 附件 + 游标 |
| `paths.rs` | 本机路径：按 XDG 基目录规范取 data / state / config / cache |
| `backend.rs` | 后端：拼 argv、跑 lark-cli、解信封、落盘 |

- 测试全在 `tests/`（含假 lark-cli 与夹具），测试自写临时目录；`src/` 只放实现
- 示例：`examples/qtrecurit.rs` + `examples/pipelines/send-invite-email.yaml`（招聘邀约流程，入口用群邀请链接）

### 下一步：qtcloud-connect 飞书功能迁入

方案在 `src/cli/docs/dev-guide/`（总览 index + 五篇功能文档），目标是承接 qtcloud-connect 的平台层能力，长在现有骨架上，不出现第二套命令行实现：

1. 补平台动作：扩 `im_messages_search` 的 `query`，新增 `im_chat_search`、`contact_search_user`、`im_messages_send`——资源从六个变七个，动作从十四个变十九个
2. 聊天记录搜索（im-search）
3. 群通知（im-notice）：依赖群搜索、人员搜索、消息发送
4. 邮件投递统计（mail-report）：拉取迁入，分类与报告留领域层
5. 邮件发送通道（mail-send）：草稿确认制，新增草稿动作、操作日志、超时与 dry-run
6. 聊天知识提取（im-extract）：只交付输入侧字段契约，模型与提示词不进本仓

迁移原则见 dev-guide 总览：适配层只做去平台化不做领域化、流程是数据、写回人工确认、命令面仍不定义。

### 验收命令

```bash
cd src/cli
cargo fmt --check
cargo test --locked
cargo clippy --locked -- -A warnings
```
