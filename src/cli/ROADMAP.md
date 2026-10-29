# qtcloud-feishu CLI 路线图

CLI 的实施路线：阶段目标、顺序依据、后置代价。方向与判据（为什么做飞书适配）不在这，在 `data/roadmap`；单个功能的迁入方案在本目录各篇。

> 更新日期：2026-10-29

## 阶段一：邮件发送通道迁入库能力 —— 已完成（2026-10-29）

- 达成标志：`mail_draft_create` / `mail_draft_send` 两动作落地；写动作 `logged` 操作日志（fail-closed）；三十秒超时；`plan` dry-run。测试 10 套全绿。
- 遗留：`mail_send` 起草/直发的 `to` 仍是逗号串，方案风险项要求真 CLI 冒烟后再定是否改 `Arg::Csv`。

## 阶段二：草稿确认闭环 —— 已完成（2026-10-29）

- 达成标志：`DRAFT_FLOW` 测试流程跑通「系统起草 → 停格等人 → 确认后系统发草稿」；draft_id 跨进程存回读得出；给 Studio 的数据契约已定（下）。
- 定形结论：**确认是一次状态变更，不是动作**。判据：确认本身不碰飞书，登记成动作就是假动作；状态变更写进 state 文件，state 文件本身就是留痕（谁、哪天、轨迹多一条）。

机制：

- `flow::Entry` 加 `artifacts`（产物表，如草稿 id）与 `confirmed`（确认日期，空串＝未确认）；serde 默认值，旧 state 文件兼容
- 迁移的检查加一类 `已确认: true`；确认入口唯一：`flow::confirm(state, who, today)`，重复确认报错
- 动作定义的产出单值自动存进 `artifacts`；后续定义用 `{{产物.草稿id}}` 取

遗留：确认目前只能代码入口（`flow::confirm`），命令面没子命令、Studio 没按钮——两者接上才算对用户可用。

## 阶段三：补平台动作 + 两个 im 功能迁入

- 达成标志：`im_chat_search`、`contact_search_user`、`im_messages_send` 登记完成（资源七、动作十九）；聊天记录搜索、群通知两篇迁入方案落地，测试同步。
- 顺序依据：群通知依赖三个新动作，聊天搜索依赖 search 参数扩展——迁入清单排好的依赖序；放在阶段二后是因为这两个功能也要写回确认，机制先行避免迁完再改。
- 后置代价：拖着则 qtcloud-connect 的飞书层冻结期无限延长（它 v0.2.0 起删代码，删除的前提是这里有替代）。

## 阶段四：命令面定形

- 达成标志：`main.rs` 长出子命令（`call`/`plan`/`flow run` 一类），`body` 与 `body_file` 互斥、附件路径转相对路径在这层校验；`--dry-run` 接上 `plan`。
- 顺序依据：迁入原则定的「命令面仍不定义」——等库能力与流程机制定形再接，现在阶段二三会把形状磨出来；命令面是薄壳，早接只会返工。
- 后置代价：一直不定则 dry-run、互斥校验、路径校验永远没有挂靠点，用户只能拿库当脚本调。

## 设计偏差反思（2026-10-29）

对照代码现状逐条核过，不是凭印象：

1. **草稿确认制是半套。** `mail.rs` 文件头写着「起草后停格，人确认后发草稿」，但 `flow::Entry`（`src/flow.rs:104`）只有 `since/state/trail`，没有产物字段——draft_id 存不下来；执行器里没有「确认」这个动作。文档承诺的机制比代码超前，这是当前最大的洞，阶段二就是它。
2. **`body`/`body_file` 互斥今天没人校验。** 登记层不表达（设计如此），命令面没定形——两个都传会原样拼进 argv 交给平台。敞口存在到阶段四，期间靠调用方自觉。
3. **ETXTBSY 重试是为测试打的补丁，落在了生产路径。** 起因是假 lark-cli 的脚本写完即执行，补丁写在 `CliRunner`（`src/backend.rs`）。真 lark-cli 撞不到，但保留有余地价值（真机上刚写完脚本就执行也可能撞）；若将来被判为噪音，挪去测试设施。
4. **审计日志的来路不统一。** ops.jsonl 只有 `Backend::with_mirror` 配置（`src/backend.rs:116`），`Backend::new` 不配——库使用者走 `new` 造后端，写动作就静默不记日志，fail-closed 打了折。命令面定形时统一成必配。
5. **ops.jsonl 无轮转。** 无限增长，长期跑会成为单个大文件。没有好判据前先记着，不预设方案。

## 关联

- 迁入方案总览：[docs/dev-guide/index.md](docs/dev-guide/index.md)
- 邮件通道迁入（含遗留待定）：[docs/dev-guide/mail-send.md](docs/dev-guide/mail-send.md)
- Studio 的对应路线：[../studio/ROADMAP.md](../studio/ROADMAP.md)
