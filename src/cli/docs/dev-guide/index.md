# qtcloud-feishu CLI 飞书功能迁入指南

本目录收 qtcloud-connect 飞书能力的迁入方案：每项功能一篇，先写现状与边界，再写落到 qtcloud-feishu 的实现方式与迁移步骤。本文只做总览与排序，细节在各篇里。

## 背景

qtcloud-connect 的 CLI 里有一批直接调用 lark-cli 的能力：邮件发送、群通知、聊天搜索、邮件投递统计、聊天知识提取。这些能力依赖飞书平台细节，与共识业务无关。qtcloud-feishu 的定位是飞书适配层，平台部分迁到这里，qtcloud-connect 只留共识、Provider 与 Studio。

迁移不是原样搬代码。qtcloud-connect 的 `main.rs` 目前只接共识、通知与邮件三个子命令，聊天搜索、邮件拉取、报告与知识提取只以库和示例存在。现状描述以 qtcloud-connect v0.1.0（`9795fad`）为准。qtcloud-feishu 已经有自己的骨架：基础单元按资源登记、串联器读定义、执行器读流程、落盘按原样与游标。迁入的功能要长在这套骨架上，避免仓库里出现第二套命令行实现。

## 迁移原则

- 适配层只做去平台化，不做领域化：飞书字段、动作、落盘与游标进本仓；岗位分类、报告排版、话术与提示词留在领域仓。依据见[平台意图](../../../../../../data/intention/index.md)。
- 登记即契约：新增动作只改它所属的资源文件；资源与动作的数量表述、测试断言跟着改。
- 流程是数据：步骤、话术、状态与执行者写进 YAML，代码只保留平台机制。
- 写回人工确认：发信默认草稿，人确认后才发送；写操作失败不假装成功。
- 命令面仍不定义：`main.rs` 保持骨架，迁入以库能力、定义与测试落地，子命令等形状定形后再接。

## 功能清单

各篇提到的 `chat.rs`、`mail.rs` 一类文件，未注明路径时指 qtcloud-connect 的 `src/cli/src/`；`resources/im.rs` 一类指本 CLI 的 `src/`。

- [聊天记录搜索](im-search.md)：关键词跨群搜索。扩展现有动作并补一条定义，是聊天知识提取的输入。
- [群通知](im-notice.md)：群名与成员名解析后发 @ 消息。拆成三个基础单元加两条短定义。
- [邮件投递统计](mail-report.md)：收件箱与 Spam 的拉取、日期过滤与岗位统计。拉取迁入，分类与报告留在领域层。
- [邮件发送通道](mail-send.md)：草稿、直发、附件、超时与发送日志。两个新动作加一条定义，确认制由执行器表达。
- [聊天知识提取](im-extract.md)：搜索结果交给模型提取结构化知识。只交付输入侧，模型与提示词不进入本仓。

## 顺序与依赖

先补平台动作，再迁功能：

1. 扩 `im_messages_search` 的 `query`，新增 `im_chat_search`、`contact_search_user`、`im_messages_send`，同步资源与动作数量。
2. 聊天记录搜索：只依赖上一步的搜索参数。
3. 群通知：依赖群搜索、人员搜索与消息发送三个动作。
4. 邮件投递统计：拉取动作已在仓库里，补定义、多文件夹游标与领域接线。
5. 邮件发送通道：新增草稿动作、操作日志、超时与 dry-run。
6. 聊天知识提取：依赖搜索动作，交付物是字段契约与领域侧接线说明。

顺序按依赖排，不按业务优先级排。邮件发送最常用，但它需要的执行器改动最多，放在平台动作稳定之后。

## 共性改动

资源从六个变成七个，动作从十四个变成十九个。本 CLI 的 `src/lib.rs`、`src/resources/mod.rs`、`src/resources/action.rs` 的文件头与 `tests/resources.rs` 的数量断言要一起改。

假 lark-cli 的分支顺序要留意：`+messages-search`、`+messages-send` 必须排在 `+messages` 之前，否则会被宽匹配吞掉；`+draft-send` 要单独一条分支，现有模式都不覆盖它。

引擎选择器要补下标：群通知需要 `群.chats[0].chat_id` 这种取第一条命中的写法，现在的 `pick` 只认 `a.b`、`a[].b` 与备用取值。

所有新动作沿用既有落盘规则：`raw/<资源>/<动作>/<页序>.json`，原样保存，正文类参数不进日志。

## 验收

每篇文档末尾给本功能的判据，统一入口是本仓测试：

```bash
cd src/cli
cargo fmt --check
cargo test --locked
cargo clippy --locked -- -A warnings
```

端到端用例走假 lark-cli，断言拼出的命令行与落盘路径；真 CLI 只做一次冒烟，不写进自动化。

## 收尾

五项功能都通过判据后，qtcloud-connect 侧删除对应实现：`chat.rs`、`email.rs`、`extract.rs`、`mail.rs`、`notice.rs`、`report.rs`、`config.rs`、`cli_config.rs`，以及调用这些模块的两个示例。README、CHANGELOG 与 `lib.rs` 的导出同步收敛，CLI 只留 `consensus`。

发布顺序：先合 qtcloud-feishu 并打版本，再合 qtcloud-connect 的删除提交，最后更新根仓子模块指针。

## 待定

- 命令面形状：`units`、`define`、`flow` 是否恢复，决定发送日志与 dry-run 的对外入口。
- 草稿确认的落点：由命令面交互承担，还是写进流程；后者需要执行器保存草稿 id。
- 身份：哪些动作固定 `--as user`，哪些允许 bot；群链接与消息发送已确认要用用户身份。
- 超时与重试：超时值、是否重试写操作、幂等键的使用范围。
- 敏感数据：聊天原文进入镜像后的目录权限，以及外发模型时的脱敏责任。
