# 群通知迁入

## 现状

`notice.rs` 提供 `qtcloud-connect notice --chat <群名或 id> --at <姓名或 id> --notice <内容>`。它先用 `im +chat-search` 把群名换成 chat_id，搜不到再用 `im +chat-list` 按名称精确匹配；成员先看是不是以 `ou_` 开头，否则用 `contact +search-user` 取第一个结果的 open_id。最后用 `im +messages-send --chat-id … --markdown '<at user_id="…"></at>\n\n正文' --as user` 发送。

现状的薄弱处：名称解析取第一条命中，重名时没有提示；`+chat-list` 的兜底没有测试；发送失败只打印错误，不改变退出码；调用没有超时；`oc_`、`ou_` 前缀判断把平台 id 规则写进了业务实现。

## 目标

迁入后没有通知专用代码，只有三个基础单元加两条短定义：

- `im_chat_search`：`im +chat-search --as user`，按关键词找群，落 `raw/im/chat-search/`。
- `contact_search_user`：`contact +search-user --as user`，按关键词找人，落 `raw/contact/search-user/`，归入新的 contact 资源。
- `im_messages_send`：`im +messages-send --as user`，向群或单聊发文本或 Markdown，落 `raw/im/messages-send/`。

@ 的语法是飞书 Markdown 的 `<at user_id="ou_…"></at>`，写在定义正文里，不做函数封装。id 前缀判断不再进入实现：给 id 的调用走「按 id」定义，给名称的走「按名称」定义，两条都短，比在代码里猜前缀清楚。

## 实现方案

### 基础单元

`resources/im.rs` 加两项，`resources/contact.rs` 新建一项，`resources/mod.rs` 把 contact 加进模块表。资源因此从六个变成七个。

`im_chat_search` 的命令是 `["im", "+chat-search", "--as", "user"]`，参数为 `query`、`page_size`、`page_token`，必填 `query`。`contact_search_user` 的命令是 `["contact", "+search-user", "--as", "user"]`，参数为 `query`、`page_size`（上限 30），必填 `query`；这条命令没有翻页参数，命中多时靠关键词收窄。`im_messages_send` 的命令是 `["im", "+messages-send", "--as", "user"]`，参数为 `chat_id`、`user_id`、`markdown`、`text`、`idempotency_key`，必填项写 `chat_id` 与 `user_id` 的二选一。

内容参数只能给一个。登记层的 `needs_any` 只能保证至少给了一个，`markdown` 与 `text` 同时给出会由 lark-cli 拒绝。命令面做互斥校验，库层保留平台的原样错误。

### 选择器

定义要把第一条搜索结果取出来：`{{群.chats[0].chat_id}}`。引擎现在只认 `a.b`、`a[].b` 与 `|` 备用取值，不支持下标。给 `engine::walk` 加 `[n]` 解析，`pick` 与参数替换自然跟着可用；补单元测试覆盖命中、越界与空表。

### 定义

群通知写成一条定义：先 `im_chat_search` 起名 `群`，再 `contact_search_user` 起名 `成员`，最后 `im_messages_send` 用 `{{群.chats[0].chat_id}}` 与 `{{成员.users[0].open_id}}` 拼 Markdown 正文。示例放 `examples/pipelines/im-notice.yaml`，测试夹具里放同一份的简化版：

```yaml
定义:
  - 名称: 群通知
    步骤:
      - 动作: im_chat_search
        参数: {query: "{{群名}}"}
        as: 群
      - 动作: contact_search_user
        参数: {query: "{{成员}}"}
        as: 成员
      - 动作: im_messages_send
        参数:
          chat_id: "{{群.chats[0].chat_id}}"
          markdown: |-
            <at user_id="{{成员.users[0].open_id}}"></at>

            {{通知}}
```

按 id 的版本另写一条短定义，只保留发送一步，上下文直接给 `群` 与 `成员id`。

### 歧义与失败

名称解析取第一条与 connect 行为一致，但重名时应当能看见。第一阶段让定义把命中的第一条原样返回，命令面将来展示全部命中让人挑；库层不替调用方决定。找不到群或人时 `pick` 取到空，`im_messages_send` 的必填校验先报错，属于 fail-closed。

## 迁移步骤

1. 给引擎加下标选择器与测试，这是唯一一处引擎改动。
2. 新建 contact 资源，登记三个动作；改文件头的资源与动作数量表述、`resources/mod.rs` 的模块表与 `tests/resources.rs` 的数量断言。
3. 假 lark-cli 补 `+chat-search`、`+messages-send`、`contact +search-user` 分支与夹具；分支顺序排在 `+messages` 之前。
4. 写按名称与按 id 两条定义，端到端断言命令行里有 chat_id、open_id 与 Markdown 正文。
5. 删除 connect 的 `notice.rs` 与 `main.rs` 的 `Notice` 子命令，同步 README。

## 验收判据

- 三个动作各自拼出带 `--as user` 的命令行，落盘目录正确。
- 按名称定义能取到 `chats[0].chat_id` 与 `users[0].open_id`，正文里 @ 标签完整。
- 找不到群或人时不发消息，错误来自必填校验。
- 资源与动作数量断言更新后通过；假 lark-cli 的宽匹配没有吞掉新命令。

## 风险与待定

- `+chat-search` 的可见范围由当前用户身份决定；connect 的 `+chat-list` 兜底没有迁入，搜不到就是搜不到，是否补兜底待定。
- 同名群或同名成员较多时取第一条可能发错人，最终应由命令面确认，不能只靠定义。
- 消息 Markdown 的长度、@ 标签与富文本结构以真 CLI 冒烟为准。
- 发送日志见 [邮件发送通道](mail-send.md) 的操作日志方案，群通知复用同一份。
