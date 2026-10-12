//! 入口：qtcloud-feishu 命令行骨架。

use clap::{CommandFactory, Parser};

/// 量潮飞书云命令行工具
#[derive(Parser)]
#[command(name = "qtcloud-feishu", version, about = "量潮飞书云命令行工具")]
struct Cli {}

fn main() {
    // 子命令尚未定义：先校验参数并展示帮助，能力落地后再扩展。
    let _ = Cli::parse();
    let mut cmd = Cli::command();
    cmd.print_help().expect("渲染帮助失败");
}
