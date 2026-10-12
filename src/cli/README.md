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

子命令尚未定义，当前仅提供帮助与版本信息。

## 示例

```bash
cargo run --example qtrecurit            # 量潮招聘流程调度器：一轮推到底，打印每人位置
cargo run --example qtrecurit -- --step  # 只推一步
```

`qtrecurit` 是实验室（`examples/quanttide-feishu-lab`）同款逻辑的 Rust 重写：流程 YAML 内嵌示例内，
检查成立才推、动作先做失败不挪格、人 / 半自动只记待办，输出与 Python 版逐行相同。

## 许可

[Apache License 2.0](../../LICENSE)
