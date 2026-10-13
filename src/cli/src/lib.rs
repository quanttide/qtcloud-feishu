//! qtcloud-feishu：基础单元 + 串联器 + 执行器。
//!
//! ```text
//! resources/  基础单元：六资源十四个动作（登记就是契约，按飞书资源分文件）
//! engine.rs   串联器：读定义，把定义展开成一串基础单元的活动
//! flow.rs     执行器：读流程与定义推格（检查成立才推、动作先做失败不挪格）
//! store.rs    落盘契约：raw/<资源>/<动作>/<页序>.json + 附件 + 游标
//! paths.rs   本机放哪：按 XDG 基目录规范取 data / state / config / cache
//! backend.rs  后端：拼 argv、跑 lark-cli、解信封、落盘
//! ```
//!
//! 命令面还没定（`main.rs` 是骨架）；库目标是给测试与将来的命令面用的。
//! 测试全在 `tests/`（含假 lark-cli 与夹具，测试自己写到临时目录），`src/` 只放实现。

pub mod backend;
pub mod engine;
pub mod flow;
pub mod paths;
pub mod resources;
pub mod store;
