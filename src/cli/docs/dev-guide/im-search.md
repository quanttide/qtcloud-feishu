# 聊天记录搜索迁入

## 现状

`chat.rs` 的 `LarkChatFetcher::search(query)` 调 `im +messages-search --query … --as user --page-size 50 --format json`，把结果拍成发送人、内容、时间三个字段返回。它只取第一页，不传 `page_token`，不落盘；结果结构体私有，只服务两个示例。

示例 `examples/cli/qtclass.rs` 与 `examples/cli/qtrecurit.rs` 是这个搜索的实际使用者：把消息拼成 `[时间] 发送人: 内容` 的文本，交给 [聊天知识提取](im-extract.md) 的模型链路。

搜索能力本身与招聘、课堂无关，属于飞书适配层。

## 目标

保留现有 `im_messages_search` 动作，补 `query` 参数，把必填从「发言人或被 @ 人」放宽为「发言人、被 @ 人或关键词三选一」。分页、原样落盘与游标交给现有的 `resources::pull` 与 `store`，不再由业务模块自己拼命令行。

定义「搜聊天记录」把结果起成发送人、内容、时间、message_id 与 chat_id 五列，供下游提取或统计。搜索结果页原样留在 `raw/im/messages-search/`，拍平时丢掉的字段可以回到原文取。

## 实现方案

`resources/im.rs` 的 `im_messages_search.params` 加 `("query", Arg::Flag("--query"))`，`needs_any` 改为 `["sender", "at_chatter", "query"]`。命令段已经带 `--as user`，不动；契约默认 `with_reactions: false` 保留。

示例定义放 `examples/pipelines/im-search.yaml`：

```yaml
定义:
  - 名称: 搜聊天记录
    说明: 按关键词跨群与单聊搜索，行字段供下游拼接与分析
    步骤:
      - 动作: im_messages_search
        参数:
          query: "{{关键词}}"
          page_size: 50
        as: 消息
    出:
      行: 消息.messages
      字段:
        发送人: sender.name
        内容: content
        时间: create_time
        message_id: message_id
        chat_id: chat_id
```

`page_size` 由调用方给，上限 50；`page_token` 由 `pull` 管，定义里不写。结果的 `data.messages` 形状以真 CLI 为准，定义字段先按现有夹具钉住，冒烟后再校正。

时间范围与「@ 我」的过滤暂不登记：lark-cli 有 `--start`、`--end` 与 `--is-at-me`，但现有场景按关键词拉取，时间窗在行上过滤即可。等人真有需求再加参数。

## 迁移步骤

1. 假 lark-cli 补 `+messages-search` 分支与夹具，分支排在 `+messages` 之前。
2. 补参数与必填，写命令行断言：`--query 招聘`、`--as user`、`--page-size 50`。
3. 写定义与端到端用例：行字段、两页夹具翻页、游标落盘。
4. 更新动作说明与 CHANGELOG；这个动作的数量不变（本系列完成后为十九个）。
5. 等 [聊天知识提取](im-extract.md) 的输入契约确认后，删除 connect 的 `chat.rs` 与两个示例。

## 验收判据

- `--query` 进入命令行，`--as user` 保留；三个必填一个都不给时报错。
- 两页夹具能拉到第二页，`cursor/<键>.json` 在整段拉完后才写入。
- 落盘文件是未拆信封的原样响应，定义的行字段从 `data.messages` 正确取出。
- 假 CLI 日志显示 `+messages-search` 走的是自己的分支，没有被 `+messages` 吞掉。

## 风险与待定

- 搜索结果里 `sender` 的结构、`content` 是纯文本还是 JSON 富文本，以真 CLI 冒烟为准；定义字段先按夹具钉住。
- 单页上限 50，自动翻页上限由 `pull` 的 `max_pages` 控制，调用方要给一个数。
- 时间范围与 @ 过滤暂不登记，等真实用法。
- 聊天原文进入镜像后的权限与脱敏，见 [聊天知识提取](im-extract.md)。
