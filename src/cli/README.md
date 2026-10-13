# qtcloud-feishu CLI

量潮飞书云命令行工具（Rust），命令名 `qtcloud-feishu`。

## 构建

```bash
cd src/cli
cargo build --release
```

## 用法

```bash
qtcloud-feishu --help
```

子命令尚未定义，当前仅提供帮助与版本信息。库目标（`src/lib.rs`）里三样已经跑通并有测试：
基础单元（`resources/`，六资源十四个动作）、串联器（`engine.rs`，读定义把定义展开成一串活动）、
执行器（`flow.rs`，读流程与定义推格）。命令面定形后从这里接上去。

## 示例

```bash
cargo run --example qtrecurit            # 量潮招聘流程调度器：一轮推到底，打印每人位置
cargo run --example qtrecurit -- --step  # 只推一步
```

`qtrecurit` 是实验室（`examples/quanttide-feishu-lab`）同款逻辑的 Rust 重写：流程 YAML 与样例同目录
（`pipelines/send-invite-email.yaml`，`include_str!` 嵌入），检查成立才推、动作先做失败不挪格、人 / 半自动只记待办，输出与 Python 版逐行相同。

## 落盘位置（XDG）

四类东西各归各的目录，都按 [XDG 基目录规范](https://specifications.freedesktop.org/basedir-spec/latest/)
取，环境变量没设或为空用缺省，给了相对路径当没设：

| 存什么 | 目录 | 环境变量 | 缺省 |
| :-- | :-- | :-- | :-- |
| 飞书原样留档（镜像）：`raw/` 与 `cursor/` | `<data>/qtcloud-feishu/` | `XDG_DATA_HOME` | `~/.local/share` |
| 流程状态（谁在哪一格） | `<state>/qtcloud-feishu/<名字>.json` | `XDG_STATE_HOME` | `~/.local/state` |
| 定义与流程（我们的话） | `<config>/qtcloud-feishu/<名字>.yaml` | `XDG_CONFIG_HOME` | `~/.config` |
| 纯可重建的中间产物 | `<cache>/qtcloud-feishu/` | `XDG_CACHE_HOME` | `~/.cache` |

镜像为什么不放 cache：`raw/` 是一页一份、只加不改的留档，飞书那边给不回当时那一页；流程状态能
从飞书侧重建，但重启后要还在，所以落 state。规则与判据在 `src/paths.rs`（`cargo test --test paths`）。
凭证不在这里——那是 `lark-cli` 自己的 keychain（`lark-cli auth status` 能看）。

## 许可

[Apache License 2.0](../../LICENSE)
