//! 入口：qtcloud-feishu 命令行骨架。
//!
//! 子命令尚未定义——命令面等形状定了再接。机制已经在这里跑通并有测试：
//!
//! ```text
//! resources/  基础单元：六资源十三个动作（登记就是契约）
//! engine.rs   串联器：读定义，把定义展开成一串基础单元的活动
//! flow.rs     执行器：读流程与定义推格（检查成立才推、动作先做失败不挪格）
//! store.rs    落盘契约：raw/<资源>/<动作>/<页序>.json + 附件 + 游标
//! backend.rs  后端：拼 argv、跑 lark-cli、解信封、落盘
//! ```

use clap::{CommandFactory, Parser};

#[allow(dead_code)] // 命令面还没接上：现在只有测试在调
mod backend;
#[cfg(test)]
mod e2e;
#[allow(dead_code)]
mod engine;
#[allow(dead_code)]
mod flow;
#[allow(dead_code)]
mod resources;
#[allow(dead_code)]
mod store;
#[cfg(test)]
mod test_support;

/// 量潮飞书云命令行工具
#[derive(Parser)]
#[command(name = "qtcloud-feishu", version, about = "量潮飞书云命令行工具")]
struct Cli {}

fn main() {
    // 子命令尚未定义：先校验参数并展示帮助，命令面定形后再扩展。
    let _ = Cli::parse();
    let mut cmd = Cli::command();
    cmd.print_help().expect("渲染帮助失败");
}
