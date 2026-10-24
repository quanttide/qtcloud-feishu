# 邮件投递统计迁入

## 现状

`email.rs` 的 `LarkCliFetcher` 调 `mail +triage --mailbox … --max 50 --format json`，按 `page_token` 最多翻二十页，只留下主题与日期。`config.rs` 从 qtcloud-profile 的 `connect/rules.json` 读岗位规则，按关键词、排除词与优先级把主题归到岗位；`report.rs` 过滤日期、生成标题、统计岗位分布与投递趋势、列出未识别样本。这套入口在 connect 里没有命令，只被库与测试使用。

这些代码里只有拉邮件是飞书的事，分类规则与报告排版是招聘域的事。

## 目标

拉取部分迁入本仓，且大部分能力已经存在：

- `mail_list`（`mail +triage`）按文件夹列摘要，支持文件夹过滤、分页与游标。
- `mail_messages`（`mail +messages`）按 id 取正文与附件元数据。
- `mail_attachment_download` 取附件原件。
- 定义「读投递邮件」把摘要与正文串起来，起发件人、日期、主题、message_id 四列，增量按日期。

分类、统计与报告留在领域层（qtrecurit 或报表仓）：规则文件、岗位名、标题文案、趋势表都不进适配层。适配层只保证两件事：原样镜像可按页重建，拍平后的行字段稳定。

## 实现方案

### 定义与游标

定义沿用测试夹具里的「读投递邮件」，补一份示例到 `examples/pipelines/mail-triage.yaml`。投递可能落在收件箱、沟通面试中与 Spam，调用方逐个文件夹跑。分页游标由 `resources::pull` 按调用方给的键存，每个文件夹一个，形如 `mail-收件箱`、`mail-Spam`；增量的日期游标由调用方按文件夹各存一份，取定义产出的 `Outcome.cursor`。`pull` 已经接受调用方给的游标键，接口不用改；把两种游标的约定写进定义说明与示例即可。

文件夹过滤走 `mail_list` 的 `folder` 参数，登记层已经处理成 `--filter {"folder": …}`，与 connect 的调用一致。

### 日期范围

`report.rs` 的日期过滤属于消费侧逻辑，不迁。适配层的增量只按游标比日期；需要固定区间时，调用方把起止日期放进上下文，自己在行上过滤，或临时把游标置零重拉。定义里不新增日期参数，避免把报表口径写进平台层。

### 领域接线

领域侧改为读适配层的行数据：一种是从库直接取 `Outcome.rows`，另一种是读 `raw/` 镜像与游标文件自行重建。两种都不需要适配层知道岗位规则。

建议 qtrecurit 保留现有分类与排版代码，只把输入从 `LarkCliFetcher` 换成「读投递邮件」的产出；规则文件位置与 `QTRECURIT_PROFILE` 环境变量不动。报告的现有测试可以照原样跑，喂一份固定行数据即可。

### 正文与附件

投递统计只数邮件，不需要正文；`mail_messages` 返回的 `attachments` 是元数据，原件由 `mail_attachment_download` 取，落在 `raw/mail/attachments/`。附件清单类工作复用这条下载动作。

## 迁移步骤

1. 把「读投递邮件」定义从测试夹具提成示例，补文件夹与游标键说明。
2. 加多文件夹端到端用例：两次不同游标键，断言 `cursor/mail-收件箱.json` 与 `cursor/mail-Spam.json` 各自独立。
3. 在 qtrecurit 侧把输入换成行数据，保留分类与排版测试，加一条用固定行的对照测试。
4. 删除 connect 的 `email.rs`、`report.rs`、`config.rs`、`cli_config.rs` 与 `lib.rs` 的对应导出。

## 验收判据

- 文件夹进入 `--filter {"folder": …}`，分页能续，游标整段拉完才前进。
- 行字段含发件人、日期、主题与 message_id，附件元数据在正文页里可见。
- 同一份固定行数据，qtrecurit 的报告结果与 connect 现有测试一致：两封投递、岗位分布、未识别样本。
- 本仓不出现岗位名、规则文件路径与报告文案。

## 风险与待定

- `+triage` 单页上限 50、总数上限 400，文件夹历史很深时要有分批策略。
- Spam 的附件下载受限，connect 的注释已记录，需要先移回收件箱。
- 日期格式不统一，消费侧解析保持宽松。
- 报表的导出格式与落点未定：JSON、CSV 或 Markdown，落 cache 还是领域仓。
