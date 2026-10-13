# 贡献指南

本文件记录 `src/cli` 的开发方法：**样例驱动**——逻辑先做成能跑的样例，跑对了再进库。

## 方法

方法分三步走，顺序固定。

### 实验室先跑通

逻辑先在量潮飞书适配仓库的实验室（`examples/quanttide-feishu-lab`）用 Python 验证：流程写成
YAML、数据来自夹具、`scripts/smoke.py` 逐项断言输出。实验室没跑通的逻辑不进本仓库。

### 样例再写一遍

把跑通的逻辑重写进 `examples/`，作为可执行的规格：流程 YAML 与样例同目录（`pipelines/send-invite-email.yaml`，与实验室同步拷贝），
实跑输出与实验室参照逐行对照——`cargo run --example qtrecurit` 与实验室
`recruitment.cli run --mock --drain` 的输出，除状态落盘路径一行与尾部状态块外应当逐行相同，
`message_id` 也要一致。对不上就是逻辑写错了，不是对照方式不对。

### 共用代码才提炼

同一段代码出现第二个调用方才搬进 `src/`。提炼只搬代码与数据，不改行为：搬完实跑，输出不变。
mock 能力层一度因预期与后续子命令共用而提炼成 `src/mail_adapter.rs`（契约）与 `src/mock.rs`（夹具），
经 `src/lib.rs` 暴露；后来第二个调用方始终没出现，又并回样例的内联模块 `mail_adapter` / `mock`——
先有样例，再有模块，不预先设计抽象。

## 跑样例

```bash
cargo run --example qtrecurit            # 一轮推到底，打印每人位置
cargo run --example qtrecurit -- --step  # 只推一步
```

样例状态落在 `target/qtrecurit/state.json`：第一轮推格，第二轮看待办与等条件；删掉它即回到
从原始数据重建。样例依赖放 `[dev-dependencies]`，二进制的依赖面不因样例变大。

## 提交前

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --all-targets`（示例内测试一并跑）
4. 实跑样例，与实验室参照对得上
5. 变更记入 `CHANGELOG.md` 的 `[Unreleased]`

## 同步拷贝

样例里的流程 YAML 与夹具数据是实验室的拷贝。实验室改了流程或数据，这边要跟着改——两边输出
对不上，先怀疑拷贝过期。
