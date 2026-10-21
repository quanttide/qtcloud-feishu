# 与真 CLI 的差距

执行器与真 `lark-cli` 之间还差什么，一处记全。替换掉原来的 `src/cli/TODO.md`（2026-10-21）。

判断依据来自适配仓 `quanttide-feishu` 的规范，不在本仓：

- 案例集 `docs/gallery/qtrecurit/send-invite-email.md`（v0.1.0，招聘流程的原始记录）；
- 手册 `docs/handbook/mail/mailbox.md`（进人、识别、去重、记状态、核对）、`docs/handbook/mail/batch-send.md`（分轮、节流、跳过、失败隔离、报告）、`docs/handbook/group/chat-link.md`（群邀请链接）。

跨仓不写相对链接，取参数以那几份原文为准。本仓的判据是两步：`cargo test --all-targets`，加实跑 `cargo run --example qtrecurit`（`CONTRIBUTING.md` 已同步为这两步）。

五组按依赖排序——前一组不做完，后一组落不了地。以实验室（Python 版）为对象的对照项不列，实验室已收为骨架，正本在本仓。

## 一、四步管道在真 CLI 上跑得通

**裸负载**：真 `lark-cli mail +triage --format json` 的顶层只有 `count` / `has_more` / `mailbox_id` / `messages` / `page_token`，没有 `ok` 也没有 `data`。而 `src/backend.rs` 的 `run_json` 要求 `ok == true`、`Page::new` 只读 `data`，第一步就报错。测试夹具是 `{"ok":true,"data":{…}}`，所以测试全绿、真跑不通。改法：无 `ok` 但有分页字段的按成功处理，负载没有 `data` 就取顶层；夹具与原样落盘契约不动。

**缺三个动作**（登记即契约，各改所属资源文件）：

| 动作 | 命令 | 参数 | 落盘 | 为什么需要 |
|:--|:--|:--|:--|:--|
| `mail_message_modify` | `mail +message-modify --as user` | `mailbox` / `message_ids`(csv) / `add_folder` / `remove_label_ids`(csv) | `raw/mail/modify/` | 案例第 4 步记状态，也是不变式 I1 的出口。用快捷方式而不写案例里那条 raw `user_mailbox.messages modify --data`：lark-cli 自己的规矩是「有快捷就用快捷」，且一条命令同样完成已读+归档（手册「一次 modify 双动作」） |
| `mail_folders_list` | `mail user_mailbox.folders list` | `user_mailbox_id` / `folder_type` | `raw/mail/folders/` | 拿归档文件夹 id |
| `im_chat_search` | `im +chat-search` | `query` / `page_size` | `raw/im/chat-search/` | 定 `chat_id`；现在只有 `LARK_CHAT_ID` 环境变量兜底 |

**`--as user` 缺位**：`im_chat_link`、`mail_send`、新的 modify 三处都要补，照 `resources/vc.rs` 的先例直接进 `command` 常量。判据：`tests/resources.rs` 的 argv 断言各加一行。

**`mail_list` 的进人取法**：进人不带 filter（真 CLI 不带 filter 即 `INBOX`，与案例第 1 步一致），要指文件夹走新增的 `folder_id`；现有 `folder` 参数不动。另需加 `is_unread` → `{"is_unread": true}`，手册「未读即未处理」靠它成立。

## 二、状态可重建（I1 能核对）

**认归档文件夹**：`src/flow.rs::derive` 的收集节要加一个「记状态用的归档文件夹」声明位，落在这个文件夹里的人直接置为第一条迁移的「到」格。案例规格里没有这一位——它的「范围」只管补拉留档，进人仍只认收件箱。代码按声明读，业务专名不进 Rust。

**同人取最早**：同人重复投递案例只留最早那条、其余随此人归档；现在 `derive` 用 `or_insert_with`，取的是列表第一条。

## 三、批量（手册 `batch-send.md` 五节）

- **分轮**：流程 YAML 加 `批量` 节，键名照规范（分轮 / 取链 / 跳过 / 失败 / 间隔 / 报告），不另起名；`advance` 改成按轮循环——每轮固定人数、轮内按投递日期从晚到早、轮末按失败清单补跑。
- **节流**：两层，每封之间在规范给的区间内取随机间隔、轮间再停；只在打真 CLI 时 sleep，mock 走 0（案例实测 2 秒仍撞 `Concurrent write conflict`）。
- **三类跳过**：已发过（归档文件夹查得到同人同主题）、姓名解析不出、缺邮箱——进跳过清单，与失败清单分开记。
- **失败隔离**：`flow.rs` 里 `holds` / `perform` 的 `?` 改成记清单后继续下一人，单人失败不拖累整轮。现在是整个 run 中止，方向与规范相反。
- **取链每轮一次、轮内复用、过期重取**：把「取链接」拆成独立定义，执行器在轮首调一次、产出注入轮上下文，迁移的动作定义只管发信（`{{邀请链接}}` 从上下文取）。流程与定义的格式都不动。
- **报告**：一轮结束按规范三项出——每人发送时间 + 成功清单 + 失败清单，落 `src/paths.rs` 已定的 state / cache 目录。规范说字段格式无定式，就按这三项，不另创。

## 四、命令面

`src/main.rs` 接子命令，用 `src/cli/CHANGELOG.md` 记过的那三个名字（`units` / `unit`、`definitions` / `define`、`flow status` / `flow run`，沿用仓内已有记录），加批量参数；`src/cli/README.md` 里「子命令尚未定义」那句随之改。

## 五、测试与待裁定

**假 lark-cli 与夹具补三条命令**，并且要能演 `+triage` 的裸负载（否则还是 mock 绿、真跑断）；`tests/resources.rs` 的 `assert_eq!(…, 14)` 随之改。提交前仍走 `CONTRIBUTING.md` 现有四条门禁，不加新的。

**写操作的真响应形状未验**：`+send` / `+message-modify` / `im chats link` 此前只跑过 `--dry-run`，第一刀落地时先补这一验。验形状不必真发信——`lark-cli mail +send` 默认只存草稿，加 `--confirm-send` 才发。

**raw 还是快捷**：案例第 4 步写的是 raw `mail user_mailbox.messages modify --data`，真 CLI 1.0.80 有等价快捷 `+message-modify`。事实源归用户，改不改回案例待定。

**中文文件夹名会被当 `folder_id`**：实测 `{"folder":"收件箱"}` → `folder_id=收件箱`、`{"folder":"inbox"}` → `INBOX`；系统文件夹是 `inbox` / `sent` / `spam`（id `INBOX` / `SENT` / `SPAM`），自定义文件夹才是中文名配数字 id。是否回案例核对待定。

**姓名解析是业务约定**：主题「岗位-姓名-学校-时长」取第二段，等出现多岗位格式再定型。本轮把形状定为「来源字段 + 分隔符 + 第几段」，写进流程收集节，键名随流程文件一起定，规则不写进代码。
