# qtcloud-feishu CLI

量潮飞书云命令行工具（Rust），命令名 `qtcloud-feishu`。

## 构建

```bash
cd src/cli
cargo build --release
```

## 三层，各说各的话

| 命令 | 说什么 | 写在哪 |
| :-- | :-- | :-- |
| `units` / `unit` | **基础单元**：飞书有什么、怎么取（十三个动作） | `src/resources/`，动作登记就是契约 |
| `definitions` / `define` | **定义**：我们的话 → 一串飞书活动 | 定义文件（YAML，场景侧） |
| `flow status` / `flow run` | **流程**：状态、迁移、检查、动作（写的是定义的名字） | 流程文件（YAML，场景侧） |

流程不认识飞书，只认识定义；定义才认识飞书；基础单元只认飞书坐标。换后端（假 lark-cli ↔
真 lark-cli）不动定义与流程，改流程不动代码。

## 用法

```bash
# 基础单元：看有哪些动作，跑一个（--store 时每取一页落一份原样响应）
qtcloud-feishu units
qtcloud-feishu unit mail_list -p folder=收件箱 -p mailbox=hr@example.com --store ./mirror
qtcloud-feishu unit mail_list -p folder=收件箱 --page-all --cursor mail --store ./mirror

# 定义：看有哪些定义，跑一条（-p 给上下文变量，`名.子名=` 会拼成嵌套的）
qtcloud-feishu definitions --definitions ./定义.yaml
qtcloud-feishu define 读投递邮件 --definitions ./定义.yaml -p 邮箱=hr@example.com -p 文件夹=收件箱

# 流程：看谁在哪一格，推一轮
qtcloud-feishu flow status --flow ./recruitment.yaml --definitions ./定义.yaml --state ./state.json
qtcloud-feishu flow run    --flow ./recruitment.yaml --definitions ./定义.yaml --state ./state.json --step
```

`--lark-cli <命令>` 指定用哪个 lark-cli（缺省取 PATH 里的，或 `LARK_CLI`）。PATH 前面放一个
假 lark-cli 就是离线自测；放真的就打真飞书。飞书坐标（邮箱、群、表格）不写在代码里，走参数或
环境变量 `LARK_MAILBOX` / `LARK_CHAT_ID` / `LARK_BASE_TOKEN` / `LARK_TABLE_ID`。

落盘：`<store>/raw/<资源>/<动作>/<页序>.json`（未解信封的原样响应）、
`<store>/raw/mail/attachments/<附件id>/<原文件名>`、`<store>/cursor/<键>.json`。

## 许可

[Apache License 2.0](../../LICENSE)
