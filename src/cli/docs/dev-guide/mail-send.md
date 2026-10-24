# 邮件发送通道迁入

## 现状

发送通道在 `mail.rs`：`qtcloud-connect mail send` 调本机 lark-cli 的 `mail +send`，默认只生成草稿，带 `--confirm-send` 时直发；`+draft-send` 有封装但没接到命令上。附件用逗号串传 `--attach`，身份固定 `--as user`，邮箱固定为一个招聘邮箱，单次调用三十秒超时。

发送日志默认写 `.quanttide/logs/send.log`，可用 `SEND_LOG_DIR` 改目录。JSON Lines，一条只记时间、收件人、主题、模板名、状态、草稿或消息 id 与备注，不记正文。动作成功后才写；写入失败向标准错误报错并打印待补记的条目，不当作成功。

模板机制只剩 `raw` 一个，`render_template` 与 `parse_vars` 没有命令入口，属于遗留代码。招聘话术已迁 qtrecurit，不随这次迁移。

## 目标

迁入后不保留独立的发送命令实现，而是三个基础单元加定义：

- `mail_send`：现有动作，命令固定带 `--confirm-send`，语义是确认后直发。
- `mail_draft_create`：新增，命令为 `mail +send`，不带确认，语义是起草。
- `mail_draft_send`：新增，命令为 `mail +draft-send`，语义是发送已存在的草稿。

草稿确认制由执行器表达：系统起草后停在一格，人确认后由系统发草稿。命令面未定形之前，第一阶段只交付库能力与用例；把确认写进流程是第二阶段，需要执行器保存草稿 id，列在待定里。

发送日志归位到本仓的操作日志：写动作成功后追加一条到 `<XDG_STATE_HOME>/qtcloud-feishu/ops.jsonl`，字段、隐私规则与 fail-closed 行为照旧。

## 实现方案

### 基础单元

`resources/mail.rs` 增加两项登记，并把 `mail_send` 标为写动作。

`mail_draft_create` 的命令是 `["mail", "+send"]`，参数与 `mail_send` 相同：`to`、`subject`、`body`、`body_file`、`mailbox`、`attachments`；落盘名用 `draft-create`，避免与直发的 `send` 混在一个目录。`mail_draft_send` 的命令是 `["mail", "+draft-send", "--as", "user"]`，参数是 `draft_ids`，用 `Arg::Csv("--draft-id")`，落盘名 `draft-send`。

`body` 与 `body_file` 互斥，登记层不表达互斥关系；命令面做校验，库层保留平台的原样错误。正文文件也必须是当前工作目录下的相对路径。

身份与邮箱不再写死。`mail_send` 与 `mail_draft_create` 的 `mailbox` 由调用方给，缺省取 `LARK_MAILBOX`；`--as user` 保留在命令段里，发信必须用用户身份。

附件是本地路径，lark-cli 要求相对当前工作目录的路径。命令面负责把用户给的路径转成相对路径或直接拒绝，库层不猜。

### 操作日志

`resources/action.rs` 的 `Action` 加一项 `logged: bool`，写动作标为真。`Backend::call` 在动作成功、落盘完成之后追加一条 JSON Lines 到 `paths::ops_log_path()`：时间、动作名、资源名、参数、返回负载里的 id 与状态。参数里排除 `body`、`markdown`、`text`、`content`，主题与收件人保留，符合原有隐私口径。

日志写入失败时返回错误，错误信息里带出待补记的那一条，不静默。动作本身已经发生，错误文案要把这一点说清。

发送日志的「只记元数据」不能只靠自觉：`Action` 的标记决定哪条动作进日志，字段排除规则决定哪些参数能落。新增写动作时两处一起改。

### 超时与 dry-run

`Backend` 加可选超时，默认三十秒，可由 `LARK_CLI_TIMEOUT` 覆盖。实现照 connect 的做法：`spawn` 加通道等待，超时报错。写动作没有超时会无限挂起，这是迁入时必须补的一项。

dry-run 不执行动作：给 `Backend` 加 `plan`，复用 `Action::argv` 返回命令行与落盘名。命令面有 `--dry-run` 时调用它打印；库层不落盘、不写日志。参数校验、位置参数与默认值的逻辑要在 `call` 与 `plan` 之间共用一份，避免两边各写一套。

### 模板

不迁 `render_template` 与 `parse_vars`。话术放进定义 YAML，引擎的 `{{}}` 已覆盖变量替换；业务话术归 qtrecurit，适配层只保留「正文由调用方给」的形态。

## 迁移步骤

1. 假 lark-cli 补 `+draft-send` 分支与夹具，先写命令行为断言。
2. 登记两个新动作，给 `Action` 加 `logged`，改动作数量表述与测试断言。
3. 加操作日志路径与写入，补隐私与失败判据。
4. 给 `Backend` 加超时与 `plan`，用假 CLI 验证超时分支。
5. 写发信定义与端到端用例：起草、直发、正文文件、附件、收件人写法、失败不写日志。
6. 删除 connect 的 `mail.rs`、`main.rs` 的 `Mail` 子命令与 README 的发送通道章节；流程化草稿确认另开一期。

## 验收判据

- 起草拼出 `mail +send` 且不带 `--confirm-send`；直发拼出 `--confirm-send`；发草稿拼出 `mail +draft-send --draft-id`；正文文件拼出 `--body-file`。
- 命令行里不出现固定邮箱；调用方不给 mailbox 时取 `LARK_MAILBOX`。
- 成功后 `ops.jsonl` 有一条记录，正文不在其中；写日志失败返回错误并给出待补记条目。
- 失败时不写日志，错误信息包含 lark-cli 的 message。
- 超时按配置生效；dry-run 只打印，不产生子进程、不落盘、不写日志。

## 风险与待定

- lark-cli 发信没有幂等键，重试可能重复投递。先靠人工确认与日志核对，不自动重试写操作。
- `--to` 的文档写的是重复 flag，现有实现传逗号串；迁入前用真 CLI 冒烟确认，必要时把 `to` 改成 `Arg::Csv` 并按平台要求展开。
- 返回 id 的字段随是否直发而变：草稿是 `draft_id`，直发是 `message_id`。定义里用 `draft_id|message_id` 的备用取值，不在代码里分支。
- 草稿确认进流程需要执行器保存草稿 id，状态结构要加产物字段；这是第二阶段的事，不阻塞第一版删除 connect 代码。
- 操作日志与 `raw/` 的一致性：日志写失败时动作已发生，只能人工补记；这是刻意保留的 fail-closed 行为。
