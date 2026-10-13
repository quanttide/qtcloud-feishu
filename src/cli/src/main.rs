//! 入口：qtcloud-feishu 命令行。
//!
//! 三件事，对应三层：
//!
//! ```text
//! units / unit          基础单元：飞书有什么、怎么取（十三个动作）
//! definitions / define  定义：我们的话 → 一串飞书活动
//! flow status|run       流程：状态、迁移、检查、动作（写的是定义的名字）
//! ```
//!
//! 值都不写死在代码里：飞书坐标（邮箱、群、表格）走参数或环境变量（`LARK_MAILBOX` 等），
//! 我们的话（定义、流程）走配置文件。

mod backend;
mod engine;
mod flow;
mod resources;
mod store;
#[cfg(test)]
mod test_support;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde_json::{Map, Value, json};

use backend::Backend;
use engine::Engine;

#[derive(Parser)]
#[command(name = "qtcloud-feishu", version, about = "量潮飞书云命令行工具")]
struct Cli {
    /// lark-cli 命令（缺省取 PATH 里的，或环境变量 LARK_CLI）
    #[arg(long, global = true, value_name = "命令")]
    lark_cli: Option<String>,
    /// 落盘根目录：`raw/<资源>/<动作>/<页序>.json` 与 `cursor/`
    #[arg(long, global = true, value_name = "目录")]
    store: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 列基础单元的动作（十三个）
    Units,
    /// 跑一个基础单元的动作
    Unit {
        /// 动作名（见 units）
        action: String,
        /// 参数：`名=值`，值按 JSON 读，读不通当字符串
        #[arg(short = 'p', long = "param", value_name = "名=值")]
        params: Vec<String>,
        /// 拉到 has_more=false（一页一页取，每页落盘）
        #[arg(long)]
        page_all: bool,
        /// 拉到底后把游标写到 `cursor/<这个键>.json`（列表类动作用资源名当键）
        #[arg(long, value_name = "键")]
        cursor: Option<String>,
    },
    /// 列定义（我们的话 → 一串飞书活动）
    Definitions {
        /// 定义文件（YAML）
        #[arg(long, value_name = "文件")]
        definitions: PathBuf,
    },
    /// 跑一条定义
    Define {
        /// 定义名（见 definitions）
        name: String,
        /// 定义文件（YAML）
        #[arg(long, value_name = "文件")]
        definitions: PathBuf,
        /// 上下文变量：`名=值`；`候选人.邮箱=…` 会拼成嵌套的
        #[arg(short = 'p', long = "param", value_name = "名=值")]
        params: Vec<String>,
        /// 把每一步的原样产出也打印出来（排查"这句话是怎么变成活动的"）
        #[arg(long)]
        steps: bool,
    },
    /// 流程：状态、迁移、检查、动作
    Flow {
        #[command(subcommand)]
        command: FlowCommand,
    },
}

#[derive(Subcommand)]
enum FlowCommand {
    /// 看谁在哪一格
    Status {
        #[arg(long, value_name = "文件")]
        flow: PathBuf,
        #[arg(long, value_name = "文件")]
        definitions: PathBuf,
        #[arg(long, value_name = "文件")]
        state: PathBuf,
        /// 本机附件目录（定义里的 `{{附件目录}}`）
        #[arg(long, value_name = "目录")]
        attachments: Option<PathBuf>,
        /// 今天（YYYY-MM-DD，缺省本机日期）
        #[arg(long, value_name = "日期")]
        today: Option<String>,
    },
    /// 推一轮
    Run {
        #[arg(long, value_name = "文件")]
        flow: PathBuf,
        #[arg(long, value_name = "文件")]
        definitions: PathBuf,
        #[arg(long, value_name = "文件")]
        state: PathBuf,
        #[arg(long, value_name = "目录")]
        attachments: Option<PathBuf>,
        #[arg(long, value_name = "日期")]
        today: Option<String>,
        /// 只推一步
        #[arg(long)]
        step: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let backend = Backend::new(cli.lark_cli.clone(), cli.store.clone());
    match cli.command {
        Command::Units => {
            println!("{:<28} {:<8} 参数", "动作", "资源");
            for action in resources::all() {
                let params: Vec<String> = action
                    .param_names()
                    .iter()
                    .map(|name| {
                        if action.needs_any.contains(name) {
                            format!("{name}*")
                        } else {
                            (*name).to_string()
                        }
                    })
                    .collect();
                println!(
                    "{:<28} {:<8} {}",
                    action.name,
                    action.resource,
                    params.join(", ")
                );
            }
            println!("\n* 表示必填（写一个就够的那些）。取值见 docs/index.md。");
        }
        Command::Unit {
            action,
            params,
            page_all,
            cursor,
        } => {
            let params = parse_params(&params)?;
            if page_all {
                let pages = resources::pull(&backend, &action, &params, cursor.as_deref(), 100)?;
                let payloads: Vec<Value> = pages.iter().map(|page| page.payload.clone()).collect();
                print_json(&Value::Array(payloads))?;
                return Ok(());
            }
            let payload = backend.call(&action, params)?;
            match payload {
                resources::Payload::Page(page) => print_json(&page.payload)?,
                resources::Payload::Files(files) => {
                    let rows: Vec<Value> = files
                        .iter()
                        .map(|file| {
                            json!({
                                "attachment_id": file.attachment_id,
                                "filename": file.filename,
                                "path": file.path.display().to_string(),
                            })
                        })
                        .collect();
                    print_json(&Value::Array(rows))?;
                }
            }
        }
        Command::Definitions { definitions } => {
            let defs = engine::load(&definitions)?;
            for definition in defs.values() {
                println!(
                    "{:<16} {}（{} 步）",
                    definition.name,
                    definition.note,
                    definition.steps.len()
                );
            }
        }
        Command::Define {
            name,
            definitions,
            params,
            steps,
        } => {
            let defs = engine::load(&definitions)?;
            let engine = Engine::new(&backend, defs);
            let context = parse_context(&params)?;
            let outcome = engine.call(&name, &context)?;
            let mut out = Map::new();
            if steps {
                let by_step: Map<String, Value> = outcome.steps.clone().into_iter().collect();
                out.insert("步骤".to_string(), Value::Object(by_step));
            }
            if !outcome.rows.is_empty() {
                out.insert(
                    "行".to_string(),
                    Value::Array(
                        outcome
                            .rows
                            .iter()
                            .map(|r| Value::Object(r.clone()))
                            .collect(),
                    ),
                );
            }
            if !outcome.single.is_empty() {
                out.insert("单条".to_string(), Value::Object(outcome.single.clone()));
            }
            if let Some(cursor) = &outcome.cursor {
                out.insert("游标".to_string(), Value::String(cursor.clone()));
            }
            print_json(&Value::Object(out))?;
        }
        Command::Flow { command } => match command {
            FlowCommand::Status {
                flow,
                definitions,
                state,
                attachments,
                today,
            } => {
                let today = today.unwrap_or_else(local_today);
                let flow = flow::load(&flow)?;
                println!("流程：{}", flow.name);
                let defs = engine::load(&definitions)?;
                let engine = Engine::new(&backend, defs);
                let context = flow::context_of(
                    &flow,
                    attachments.map(|dir| {
                        let mut extra = Map::new();
                        extra.insert(
                            "附件目录".to_string(),
                            Value::String(dir.display().to_string()),
                        );
                        extra
                    }),
                );
                let state = flow::load_state(&state);
                let state = if state.is_empty() {
                    let rebuilt = flow::derive(&engine, &flow, &context, &today)?;
                    println!("本地无状态：从飞书侧原始数据重建（{} 人）", rebuilt.len());
                    rebuilt
                } else {
                    state
                };
                for (who, entry) in &state {
                    println!("  {who:<28} {}（{} 起）", entry.state, entry.since);
                }
            }
            FlowCommand::Run {
                flow,
                definitions,
                state,
                attachments,
                today,
                step,
            } => {
                let today = today.unwrap_or_else(local_today);
                let state_path = state;
                let flow = flow::load(&flow)?;
                println!("流程：{}", flow.name);
                let defs = engine::load(&definitions)?;
                let engine = Engine::new(&backend, defs);
                let context = flow::context_of(
                    &flow,
                    attachments.map(|dir| {
                        let mut extra = Map::new();
                        extra.insert(
                            "附件目录".to_string(),
                            Value::String(dir.display().to_string()),
                        );
                        extra
                    }),
                );
                let mut state = flow::load_state(&state_path);
                if state.is_empty() {
                    state = flow::derive(&engine, &flow, &context, &today)?;
                    println!("本地无状态：从飞书侧原始数据重建（{} 人）", state.len());
                }
                let report = flow::advance(&engine, &flow, &mut state, &context, &today, step)?;
                for (who, from, to, effect) in &report.advanced {
                    let tail = if effect.is_empty() {
                        String::new()
                    } else {
                        format!("（{effect}）")
                    };
                    println!("  ✓ {who:<28} {from} → {to}{tail}");
                }
                for (who, migration, why) in &report.waiting {
                    println!("  ⋯ {who:<28} {migration} {why}");
                }
                for (who, from, to, trigger, executor) in &report.todos {
                    println!("  ☐ {who:<28} {from} → {to}｜{trigger}（{executor}）");
                }
                if report.is_empty() {
                    println!("  这一轮没有可推的迁移");
                }
                flow::save_state(&state_path, &state)
                    .map_err(|err| format!("状态落盘失败：{err}"))?;
            }
        },
    }
    Ok(())
}

fn local_today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn print_json(value: &Value) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    println!("{text}");
    Ok(())
}

/// `名=值` → 参数表（值按 JSON 读，读不通当字符串）。
fn parse_params(pairs: &[String]) -> Result<Map<String, Value>, String> {
    let mut params = Map::new();
    for pair in pairs {
        let (key, value) = split_pair(pair)?;
        params.insert(key, value);
    }
    Ok(params)
}

/// `名=值` → 上下文（`候选人.邮箱=…` 拼成嵌套的）。
fn parse_context(pairs: &[String]) -> Result<Map<String, Value>, String> {
    let mut context: Map<String, Value> = Map::new();
    for pair in pairs {
        let (key, value) = split_pair(pair)?;
        let parts: Vec<&str> = key.split('.').collect();
        let mut cursor = &mut context;
        for part in &parts[..parts.len() - 1] {
            cursor = cursor
                .entry((*part).to_string())
                .or_insert_with(|| Value::Object(Map::new()))
                .as_object_mut()
                .ok_or_else(|| format!("{key} 里的 {part} 已经是别的值了"))?;
        }
        cursor.insert(parts[parts.len() - 1].to_string(), value);
    }
    Ok(context)
}

fn split_pair(pair: &str) -> Result<(String, Value), String> {
    let (key, raw) = pair
        .split_once('=')
        .ok_or_else(|| format!("参数要写成「名=值」：{pair}"))?;
    let value =
        serde_json::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
    Ok((key.trim().to_string(), value))
}
