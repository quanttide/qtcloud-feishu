//! 流程：状态、迁移、检查、动作——读两份配置推格。
//!
//! 流程写的是**定义的名字**（`看谁进群了`、`发群二维码`），定义才认识飞书（见 [`crate::engine`]）：
//! 流程不认识飞书，只认识定义。所以这一层里没有一条 lark-cli 命令、没有一个飞书字段名。
//!
//! 推格规则：执行者是「系统」且检查成立才推；动作先做，失败不挪格；人 / 半自动只记待办。
//! 状态落在本地文件（缓存，事实在飞书那边），可从飞书侧重建。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::engine::Engine;
use crate::resources::action::{is_absent, text};

#[derive(Debug, Deserialize)]
pub struct Flow {
    #[serde(rename = "流程", default)]
    pub name: String,
    #[serde(rename = "收集", default)]
    pub collect: Collect,
    #[serde(rename = "状态", default)]
    pub states: Vec<String>,
    #[serde(rename = "迁移", default)]
    pub migrations: Vec<Migration>,
}

/// 收集节：拉哪些地方、用哪条定义看「谁来了」与「谁进群了」。
#[derive(Debug, Default, Deserialize)]
pub struct Collect {
    #[serde(rename = "邮箱", default)]
    pub mailbox: String,
    #[serde(rename = "范围", default)]
    pub scope: Vec<String>,
    #[serde(rename = "进人", default)]
    pub entry: String,
    #[serde(rename = "群成员", default)]
    pub members: String,
}

#[derive(Debug, Deserialize)]
pub struct Migration {
    #[serde(rename = "从")]
    pub from: String,
    #[serde(rename = "到")]
    pub to: String,
    #[serde(rename = "触发", default)]
    pub trigger: String,
    #[serde(rename = "执行者", default)]
    pub executor: String,
    #[serde(rename = "检查")]
    pub check: Option<Check>,
    #[serde(rename = "动作")]
    pub action: Option<Action>,
    #[serde(rename = "副作用", default)]
    pub effect: String,
}

impl Migration {
    /// 只有「系统」执行的迁移，执行器才代劳；人做的是待办。
    pub fn automatic(&self) -> bool {
        self.executor == "系统"
    }
}

/// 一段机器可判的条件，两类互斥：定义检查（跑一条定义后判）与时间检查（纯领域逻辑）。
#[derive(Debug, Deserialize)]
pub struct Check {
    #[serde(rename = "定义")]
    pub definition: Option<String>,
    #[serde(rename = "成立")]
    pub holds: Option<String>,
    #[serde(rename = "计时")]
    pub timing: Option<Timing>,
}

#[derive(Debug, Deserialize)]
pub struct Timing {
    #[serde(rename = "天数", default)]
    pub days: i64,
}

/// 推这条迁移之前要跑的定义（一串飞书活动）；失败就不挪格。
#[derive(Debug, Deserialize)]
pub struct Action {
    #[serde(rename = "定义")]
    pub definition: String,
}

pub fn parse(text: &str) -> Result<Flow, String> {
    serde_yaml::from_str(text).map_err(|err| format!("流程读不通：{err}"))
}

pub fn load(path: &Path) -> Result<Flow, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("读不到流程文件 {}：{err}", path.display()))?;
    parse(&text)
}

// ── 状态：一个候选人现在在哪一格 ─────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Entry {
    pub since: String,
    pub state: String,
    #[serde(default)]
    pub trail: Vec<String>,
}

pub type State = BTreeMap<String, Entry>;

pub fn load_state(path: &Path) -> State {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_state(path: &Path, state: &State) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut json = serde_json::to_string_pretty(state)?;
    json.push('\n');
    std::fs::write(path, json)
}

/// 「YYYY-MM-DD」之间隔了几天。
pub fn days_since(from: &str, to: &str) -> Option<i64> {
    let parse = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
    Some((parse(to)? - parse(from)?).num_days())
}

/// 调用方给的定义上下文 + 流程收集节里的邮箱。
pub fn context_of(flow: &Flow, extra: Option<Map<String, Value>>) -> Map<String, Value> {
    let mut ctx = extra.unwrap_or_default();
    if !flow.collect.mailbox.is_empty() {
        ctx.entry("邮箱".to_string())
            .or_insert_with(|| Value::String(flow.collect.mailbox.clone()));
    }
    ctx
}

fn with_candidate(context: &Map<String, Value>, who: &str) -> Map<String, Value> {
    let mut ctx = context.clone();
    ctx.insert("候选人".to_string(), serde_json::json!({ "邮箱": who }));
    ctx
}

/// 从飞书侧重建每人现在在哪一格：进人只看收件箱，群成员只用来升级状态。
pub fn derive(
    engine: &Engine,
    flow: &Flow,
    context: &Map<String, Value>,
    today: &str,
) -> Result<State, String> {
    if flow.collect.entry.is_empty() || flow.collect.members.is_empty() {
        return Err("收集节要写「进人」与「群成员」各用哪条定义".to_string());
    }
    let members: BTreeSet<String> = engine
        .call(&flow.collect.members, context)?
        .rows
        .iter()
        .map(|row| text(row.get("邮箱").unwrap_or(&Value::Null)).to_lowercase())
        .filter(|who| !who.is_empty())
        .collect();

    let mut applied: BTreeMap<String, String> = BTreeMap::new();
    for folder in &flow.collect.scope {
        if folder != "收件箱" {
            continue; // 漂移 / Spam 是补拉范围，进人仍以收件箱为准
        }
        let mut ctx = context.clone();
        ctx.insert("文件夹".to_string(), Value::String(folder.clone()));
        for row in engine.call(&flow.collect.entry, &ctx)?.rows {
            let who = text(row.get("发件人").unwrap_or(&Value::Null)).to_lowercase();
            if who.contains('@') {
                applied
                    .entry(who)
                    .or_insert_with(|| text(row.get("日期").unwrap_or(&Value::Null)));
            }
        }
    }

    let mut state = State::new();
    for (who, date) in applied {
        let joined = members.contains(&who);
        let (place, since) = if joined {
            ("已进群".to_string(), today.to_string())
        } else {
            (flow.states.first().cloned().unwrap_or_default(), date)
        };
        state.insert(
            who,
            Entry {
                since,
                state: place.clone(),
                trail: vec![place],
            },
        );
    }
    Ok(state)
}

#[derive(Debug, Default)]
pub struct Report {
    /// 谁 从 → 到（副作用）
    pub advanced: Vec<(String, String, String, String)>,
    /// 谁 现在在哪格 → 到哪格｜触发（执行者）
    pub todos: Vec<(String, String, String, String, String)>,
    /// 谁 哪条迁移 为什么不成立
    pub waiting: Vec<(String, String, String)>,
}

impl Report {
    pub fn is_empty(&self) -> bool {
        self.advanced.is_empty() && self.todos.is_empty() && self.waiting.is_empty()
    }
}

fn holds(
    engine: &Engine,
    mig: &Migration,
    who: &str,
    entry: &Entry,
    today: &str,
    context: &Map<String, Value>,
) -> Result<bool, String> {
    let Some(check) = &mig.check else {
        return Ok(true); // 没写检查 = 无条件
    };
    if let Some(timing) = &check.timing {
        // 纯时间检查：不碰飞书
        return Ok(days_since(&entry.since, today).unwrap_or(0) >= timing.days);
    }
    let Some(definition) = &check.definition else {
        return Ok(false);
    };
    let outcome = engine.call(definition, &with_candidate(context, who))?;
    let values: Vec<String> = outcome.values().iter().map(|v| v.to_lowercase()).collect();
    let empty = outcome.rows.is_empty() && values.is_empty();
    match check.holds.as_deref() {
        Some("非空") => Ok(!empty),
        Some("为空") => Ok(empty),
        Some(holds) if holds.starts_with("包含") => {
            Ok(values.iter().any(|v| v == &who.to_lowercase()))
        }
        _ => Ok(false),
    }
}

/// 跑动作那条定义，返回它的产出（拿来做副作用那句话）。
fn perform(
    engine: &Engine,
    mig: &Migration,
    who: &str,
    context: &Map<String, Value>,
) -> Result<String, String> {
    let Some(action) = &mig.action else {
        return Ok(String::new());
    };
    let outcome = engine.call(&action.definition, &with_candidate(context, who))?;
    if !outcome.single.is_empty() {
        return Ok(outcome
            .single
            .values()
            .filter(|value| !is_absent(value))
            .map(text)
            .collect::<Vec<_>>()
            .join("，"));
    }
    Ok(outcome.values().join("，"))
}

/// 推一轮：系统执行且检查成立的推过去（动作先做，失败不挪格），人/半自动记待办。
pub fn advance(
    engine: &Engine,
    flow: &Flow,
    state: &mut State,
    context: &Map<String, Value>,
    today: &str,
    step_only: bool,
) -> Result<Report, String> {
    let mut report = Report::default();
    let people: Vec<String> = state.keys().cloned().collect();
    for who in people {
        let entry = state.get(&who).cloned().unwrap_or_default();
        let migrations: Vec<&Migration> = flow
            .migrations
            .iter()
            .filter(|mig| mig.from == entry.state)
            .collect();
        for mig in migrations {
            if !mig.automatic() {
                report.todos.push((
                    who.clone(),
                    entry.state.clone(),
                    mig.to.clone(),
                    mig.trigger.clone(),
                    mig.executor.clone(),
                ));
                continue;
            }
            if !holds(engine, mig, &who, &entry, today, context)? {
                let label = mig
                    .check
                    .as_ref()
                    .map(|check| {
                        check.definition.clone().unwrap_or_else(|| {
                            if check.timing.is_some() {
                                "计时".into()
                            } else {
                                "无检查".into()
                            }
                        })
                    })
                    .unwrap_or_else(|| "无检查".into());
                report.waiting.push((
                    who.clone(),
                    format!("{}→{}", mig.from, mig.to),
                    format!("检查不成立（{label}）"),
                ));
                continue;
            }
            let result = perform(engine, mig, &who, context)?;
            let effect = match (mig.effect.is_empty(), result.is_empty()) {
                (true, _) => result,
                (false, true) => mig.effect.clone(),
                (false, false) => format!("{}：{}", mig.effect, result),
            };
            if let Some(entry) = state.get_mut(&who) {
                entry.state = mig.to.clone();
                entry.since = today.to_string();
                entry.trail.push(mig.to.clone());
            }
            report
                .advanced
                .push((who.clone(), mig.from.clone(), mig.to.clone(), effect));
            if step_only {
                return Ok(report);
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Backend, Output, Runner};
    use crate::engine::Engine;
    use serde_json::json;

    const FLOW: &str = "流程: 招聘\n收集:\n  邮箱: hr@example.com\n  范围: [收件箱]\n  进人: 读投递邮件\n  群成员: 看谁进群了\n状态: [投递, 二维码已发, 已进群]\n迁移:\n  - 从: 投递\n    到: 二维码已发\n    触发: 收到投递\n    执行者: 系统\n    动作:\n      定义: 发群二维码\n    副作用: 记发送时间\n  - 从: 二维码已发\n    到: 已进群\n    触发: 群里出现该邮箱\n    执行者: 系统\n    检查:\n      定义: 看谁进群了\n      成立: 包含该候选人邮箱\n  - 从: 已进群\n    到: 建清单\n    触发: 人工建清单\n    执行者: 人\n";

    const DEFS: &str = "定义:\n  - 名称: 读投递邮件\n    步骤:\n      - 动作: mail_list\n        参数: {folder: \"{{文件夹}}\"}\n        as: 摘要\n      - 动作: mail_messages\n        参数: {message_ids: \"{{摘要.messages[].message_id}}\"}\n        as: 正文\n    出:\n      行: 正文.messages\n      字段: {发件人: \"from\", 日期: date, message_id: message_id}\n  - 名称: 看谁进群了\n    步骤:\n      - 动作: im_chat_members\n        参数: {chat_id: \"{{群}}\"}\n        as: 成员\n    出:\n      行: 成员.users\n      字段: {邮箱: member_id}\n  - 名称: 发群二维码\n    步骤:\n      - 动作: mail_send\n        参数: {to: \"{{候选人.邮箱}}\", subject: 群二维码, body: 扫码进群}\n    出: {字段: {邮件id: message_id}}\n";

    /// 假 runner：按命令段回预制负载，把 argv 记下来。
    struct Fake {
        seen: std::sync::Mutex<Vec<Vec<String>>>,
        members: Vec<&'static str>,
        mails: Vec<(&'static str, &'static str)>,
        fail_send: bool,
    }

    impl Runner for Fake {
        fn run(&self, _program: &str, args: &[String]) -> Result<Output, String> {
            self.seen.lock().unwrap().push(args.to_vec());
            let joined = args.join(" ");
            let payload = if joined.contains("mail +triage") {
                let messages: Vec<Value> = self
                    .mails
                    .iter()
                    .map(|(who, date)| json!({"message_id": format!("m-{who}"), "from": who, "date": date}))
                    .collect();
                json!({"ok": true, "data": {"messages": messages}})
            } else if joined.contains("mail +messages") {
                let messages: Vec<Value> = self
                    .mails
                    .iter()
                    .map(|(who, date)| {
                        json!({
                            "message_id": format!("m-{who}"), "from": who, "date": date,
                            "subject": "投递", "body_plain_text": "简历"
                        })
                    })
                    .collect();
                json!({"ok": true, "data": {"messages": messages}})
            } else if joined.contains("chat-members-list") {
                let users: Vec<Value> = self
                    .members
                    .iter()
                    .map(|who| json!({"member_id": who, "name": who}))
                    .collect();
                json!({"ok": true, "data": {"users": users}})
            } else if joined.contains("+send") {
                if self.fail_send {
                    json!({"ok": false, "error": {"message": "发信失败"}})
                } else {
                    json!({"ok": true, "data": {"message_id": format!("mock-{}", self.mails.len())}})
                }
            } else {
                json!({"ok": true, "data": {}})
            };
            let success = payload.get("ok").and_then(Value::as_bool).unwrap_or(false);
            Ok(Output {
                stdout: if success {
                    payload.to_string()
                } else {
                    String::new()
                },
                stderr: if success {
                    String::new()
                } else {
                    payload.to_string()
                },
                success,
            })
        }
    }

    fn engine(fake: std::sync::Arc<Fake>) -> (Backend, crate::engine::Definitions) {
        struct Shared(std::sync::Arc<Fake>);
        impl Runner for Shared {
            fn run(&self, program: &str, args: &[String]) -> Result<Output, String> {
                self.0.run(program, args)
            }
        }
        let backend = Backend::with_runner(Box::new(Shared(fake)), "lark-cli", None);
        (backend, load_defs_from(DEFS))
    }

    fn load_defs_from(text: &str) -> crate::engine::Definitions {
        crate::engine::parse(text).unwrap()
    }

    #[test]
    fn 推一轮_动作先做_检查成立才推() {
        let fake = std::sync::Arc::new(Fake {
            seen: std::sync::Mutex::new(Vec::new()),
            members: vec!["herui@example.com"],
            mails: vec![
                ("chenmo@example.com", "2026-10-02"),
                ("herui@example.com", "2026-10-03"),
            ],
            fail_send: false,
        });
        let (backend, defs) = engine(fake);
        let engine = Engine::new(&backend, defs);
        let flow = parse(FLOW).unwrap();
        let context = context_of(&flow, Some(Map::new()));
        let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();
        assert_eq!(state.len(), 2);
        assert_eq!(state["herui@example.com"].state, "已进群");
        assert_eq!(state["chenmo@example.com"].state, "投递");

        let report = advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();
        assert_eq!(report.advanced.len(), 1, "只有投递那位推得动");
        assert_eq!(report.advanced[0].0, "chenmo@example.com");
        assert!(
            report.advanced[0].3.contains("记发送时间"),
            "副作用带上返回值"
        );
        assert_eq!(state["chenmo@example.com"].state, "二维码已发");
        assert!(
            report.waiting.is_empty(),
            "没有检查不成立的：{:?}",
            report.waiting
        );
        assert_eq!(report.todos.len(), 1, "人执行的迁移只记待办");
        assert_eq!(report.todos[0].0, "herui@example.com");
        assert_eq!(report.todos[0].2, "建清单");
    }

    #[test]
    fn 动作失败就不挪格() {
        let fake = std::sync::Arc::new(Fake {
            seen: std::sync::Mutex::new(Vec::new()),
            members: vec![],
            mails: vec![("chenmo@example.com", "2026-10-02")],
            fail_send: true,
        });
        let (backend, defs) = engine(fake);
        let engine = Engine::new(&backend, defs);
        let flow = parse(FLOW).unwrap();
        let context = context_of(&flow, Some(Map::new()));
        let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();
        let err = advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap_err();
        assert!(err.contains("发信失败"), "{err}");
        assert_eq!(state["chenmo@example.com"].state, "投递", "失败不挪格");
    }

    #[test]
    fn 迟到的扫码照样追认() {
        let fake = std::sync::Arc::new(Fake {
            seen: std::sync::Mutex::new(Vec::new()),
            members: vec!["chenmo@example.com"],
            mails: vec![("chenmo@example.com", "2026-10-02")],
            fail_send: false,
        });
        let (backend, defs) = engine(fake);
        let engine = Engine::new(&backend, defs);
        let flow = parse(FLOW).unwrap();
        let context = context_of(&flow, Some(Map::new()));
        let state = derive(&engine, &flow, &context, "2026-10-13").unwrap();
        assert_eq!(
            state["chenmo@example.com"].state, "已进群",
            "重建时就认直接进群的"
        );
    }

    #[test]
    fn 收集节缺进人就明确报错() {
        let fake = std::sync::Arc::new(Fake {
            seen: std::sync::Mutex::new(Vec::new()),
            members: vec![],
            mails: vec![],
            fail_send: false,
        });
        let (backend, defs) = engine(fake);
        let engine = Engine::new(&backend, defs);
        let flow = parse("流程: 招聘\n收集:\n  范围: [收件箱]\n状态: [投递]\n").unwrap();
        let err = derive(&engine, &flow, &Map::new(), "2026-10-13").unwrap_err();
        assert!(err.contains("进人"), "{err}");
    }
}
