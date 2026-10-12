//! 量潮招聘流程调度器——实验室 Python 实现的同逻辑重写（Rust 版）。
//!
//! 与 `examples/quanttide-feishu-lab` 的 `flow/state/runner` 一一对应：
//! 流程是数据（YAML 内嵌本文件，改流程改这段字串），执行器读它推格——
//! 检查成立才推、动作先做失败不挪格、人 / 半自动只记待办。
//!
//! 跑法（在 `src/cli` 下）：
//!     cargo run --example qtrecurit            # 一轮推到底，再打印每人在哪格
//!     cargo run --example qtrecurit -- --step  # 只推一步

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate};
use qtcloud_feishu_cli::mock::Mock;
use serde::{Deserialize, Serialize};

/// 流程定义：与实验室 `tests/fixtures/flows/recruitment.yaml` 同步拷贝。
const FLOW_YAML: &str = r#"# 招聘流程定义
#
# 流程是数据，不是代码：改流程 = 改这份文件，执行器读它跑。
# 每个迁移标两件事：触发（什么事件让它发生）与执行者（人 / 系统 / 半自动）。
# 「人」是当前实情，不是应有形态——流程升级的方向就是把执行者从人换成系统。
#
# 检查：只有「执行者：系统」的迁移需要写检查——那是一段机器可判的条件，
# 成立才推。执行者是人或半自动的，执行器不代劳，只把它记成待办。
# 动作：迁移要真做的事（调一个能力），推过去之前先执行，失败就不挪格。
#      不写动作的迁移照旧只挪格（副作用那句话仍留给人读）。
#
# 依据：qtrecurit-private/docs/handbook/workflow.md（2026-08 按实际运行更新）
# 2026-10 简化：投递后直接发群二维码，问卷不再作为进群前置（「问卷已交」格移除）；
#              问卷调研改到入群后由系统发出，超时机制改为盯二维码。

流程: 招聘
来源: 飞书
版本: 0.1.0

# ── 对接范围：拉哪些地方 ────────────────────────────────────────
# 卡点：HR 处理邮件时会把邮件移进「沟通面试中」等文件夹，
#      只拉收件箱会漏投递；有效投递还可能被 Spam 误判（且 Spam 里的附件下载受限）。
收集:
  邮箱: hr@quanttide.com
  范围:
    - 收件箱
    - 沟通面试中
    - Spam          # 需先移回收件箱再取附件
  对象:
    - 邮件（含附件）  # 群二维码就挂在发出去的邮件上
    - 群成员         # 实训基地群
  游标: 按文件夹各存一个，增量拉取

# ── 状态 ───────────────────────────────────────────────────────
状态:
  - 投递            # 候选人出现在收件箱
  - 二维码已发       # 群二维码邮件已发出，等扫码
  - 无响应          # 二维码发出超过 N 天没进群（可被「已进群」追认）
  - 已进群
  - 问卷已发        # 入群后发送问卷调研
  - 已建清单        # candidates.csv / documents.csv 生成
  - 已评估          # 评估报告完成
  - 推荐池
  - 已推荐
  - 推荐成功
  - 不推荐

# ── 迁移 ───────────────────────────────────────────────────────
迁移:
  - 从: 投递
    到: 二维码已发
    触发: 收到投递，直接发群二维码
    执行者: 系统
    动作:
      能力: send_mail
      参数:
        to: "{{候选人.邮箱}}"
        subject: 量潮实训基地 · 群二维码
        body: 你好，扫码直接进群（群二维码见附件）。
        attachments:
          - 群二维码.png
    副作用: 记发送时间

  - 从: 二维码已发
    到: 已进群
    触发: 群成员名单里出现该邮箱（扫码进群）
    执行者: 系统
    检查:
      能力: read_group_members
      成立: 包含该候选人邮箱
    副作用: 记进群时间

  - 从: 二维码已发
    到: 无响应
    触发: 二维码发出超过 N 天仍未进群
    执行者: 系统
    检查:
      能力: _since
      成立: 超过天数
      天数: 7            # N：说改就改，代码不动
    副作用: 记入待跟进清单

  - 从: 无响应
    到: 已进群
    触发: 迟到的扫码照样算
    执行者: 系统
    检查:
      能力: read_group_members
      成立: 包含该候选人邮箱
    副作用: 追认进群时间

  - 从: 已进群
    到: 问卷已发
    触发: 入群后发送问卷调研
    执行者: 系统
    动作:
      能力: send_mail
      参数:
        to: "{{候选人.邮箱}}"
        subject: 量潮招聘 · 准入问卷
        body: 请填写问卷：…
    副作用: 记发送时间

  - 从: 问卷已发
    到: 已建清单
    触发: 当月清单生成
    执行者: 半自动       # 系统生成，姓名/岗位/附件类型需人工核对
    副作用: 写 candidates.csv / documents.csv

  - 从: 已建清单
    到: 已评估
    触发: 评估报告完成
    执行者: 人           # 准则：了解度与留存优先
    副作用: 写 data/report/YYYY-MM-{candidates-ranking,survey}.md

  - 从: 已评估
    到: 推荐池
    触发: 命中准入底线（责任心 ≥ 中 + 材料真实）
    执行者: 人
    副作用: 标记推荐列；淘汰组不进池

  - 从: 推荐池
    到: 已推荐
    触发: 匹配到企业需求单 + 候选人知情同意
    执行者: 半自动
    副作用: 生成唯一凭证（编号 + 候选人邮箱 + 企业邮箱 + 时间戳）并发出

  - 从: 已推荐
    到: 推荐成功
    触发: 企业反馈入职
    执行者: 人
    副作用: 入 referrals.csv 台账；结果回灌评估模型

# ── 终止与复活 ─────────────────────────────────────────────────
终止:
  - 不推荐         # 不推荐 ≠ 淘汰，两条线分开走
  - 推荐成功

复活:
  - 无响应 → 已进群   # 迟到的扫码照样算（这条真在迁移表里，执行器推得动）

# ── 待定（卡点分析的产物，先留空，不编）────────────────────────
待定:
  - 无响应的天数 N
  - 二维码发出后的催促节奏
  - 文件夹漂移的自动补拉范围（现在靠人记得）
  - Spam 里有效投递的自动识别"#;

// ── 流程定义：YAML → 结构（对应 flow.py）────────────────────────

#[derive(Deserialize)]
struct Flow {
    #[serde(rename = "收集")]
    collect: Collect,
    #[serde(rename = "状态")]
    states: Vec<String>,
    #[serde(rename = "迁移")]
    migrations: Vec<Migration>,
}

#[derive(Deserialize)]
struct Collect {
    #[serde(rename = "范围")]
    scope: Vec<String>,
}

#[derive(Deserialize)]
struct Migration {
    #[serde(rename = "从")]
    from: String,
    #[serde(rename = "到")]
    to: String,
    #[serde(rename = "触发")]
    trigger: String,
    #[serde(rename = "执行者")]
    executor: String,
    #[serde(rename = "检查")]
    check: Option<Check>,
    #[serde(rename = "动作")]
    action: Option<Action>,
    #[serde(rename = "副作用", default)]
    effect: String,
}

impl Migration {
    /// 只有「系统」执行的迁移，执行器才代劳；人做的是待办。
    fn automatic(&self) -> bool {
        self.executor == "系统"
    }
}

/// 一段机器可判的条件。当前流程的检查都不带参数（按候选人过滤是执行器的活）。
#[derive(Deserialize)]
struct Check {
    #[serde(rename = "能力")]
    capability: String,
    #[serde(rename = "成立")]
    holds: Option<String>,
    #[serde(rename = "天数")]
    days: Option<i64>,
}

/// 一段机器要执行的动作：推这条迁移时先调它，失败就不挪格。
#[derive(Deserialize)]
struct Action {
    #[serde(rename = "能力")]
    capability: String,
    #[serde(rename = "参数")]
    params: BTreeMap<String, Value>,
}

/// 动作参数值：字符串或字符串列表（流程里只用到这两样）。
#[derive(Deserialize)]
#[serde(untagged)]
enum Value {
    Text(String),
    List(Vec<String>),
}

// ── 状态：一个候选人现在在哪一格（对应 state.py）─────────────────

#[derive(Serialize, Deserialize, Default)]
struct Entry {
    since: String,
    state: String,
    #[serde(default)]
    trail: Vec<String>,
}

type State = BTreeMap<String, Entry>; // 有序即 Python 的 sorted(state)

fn load_state(path: &Path) -> State {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_state(path: &Path, state: &State) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut json = serde_json::to_string_pretty(state)?;
    json.push('\n');
    fs::write(path, json)
}

/// 从原始数据重建每人当前在哪一格（对应 state.py::derive）。
fn derive(flow: &Flow, cap: &Mock, today: &str) -> State {
    let members: BTreeSet<String> = cap.read_group_members().into_iter().collect();

    let mut applied: BTreeMap<String, String> = BTreeMap::new();
    for folder in &flow.collect.scope {
        if folder != "收件箱" {
            continue; // 漂移 / Spam 是补拉范围：进人仍以收件箱为准（见流程「待定」）
        }
        for m in cap.read_mail(folder) {
            let who = m.sender.to_lowercase();
            if who.contains('@') {
                applied.entry(who).or_insert(m.date);
            }
        }
    }

    // 群里还有员工，不是候选人：候选人从投递来，群成员只用来升级状态
    let mut state = State::new();
    for (who, date) in applied {
        let member = members.contains(&who);
        let (where_, since) = if member {
            ("已进群".to_string(), today.to_string())
        } else {
            (flow.states[0].clone(), date)
        };
        state.insert(
            who,
            Entry {
                since,
                state: where_.clone(),
                trail: vec![where_],
            },
        );
    }
    state
}

/// 「YYYY-MM-DD」之间隔了几天（对应 state.py::since_days）。
fn days_since(from: &str, to: &str) -> Option<i64> {
    let parse = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok();
    Some((parse(to)? - parse(from)?).num_days())
}

// 能力层（mock）在库里：`qtcloud_feishu_cli::mock`（对应实验室 capabilities/mock.py）。

// ── 执行器：一轮推一步（对应 runner.py）────────────────────────

#[derive(Default)]
struct Report {
    advanced: Vec<String>,
    todos: Vec<String>,
    waiting: Vec<String>,
}

/// 检查成立吗。没写检查 = 无条件：人这一格就该推。
fn holds(check: Option<&Check>, cap: &Mock, who: &str, entry: &Entry, today: &str) -> bool {
    let Some(c) = check else {
        return true;
    };
    if c.capability == "_since" {
        let days = days_since(&entry.since, today).unwrap_or(0);
        return days >= c.days.unwrap_or(0);
    }
    let result: Vec<String> = match c.capability.as_str() {
        "read_group_members" => cap.read_group_members(),
        _ => return false, // 未知能力：检查不成立（与 Python 版 getattr 为空一致）
    };
    match c.holds.as_deref() {
        Some("非空") => !result.is_empty(),
        Some("为空") => result.is_empty(),
        Some(h) if h.starts_with("包含") => result.iter().any(|m| m.eq_ignore_ascii_case(who)),
        _ => false,
    }
}

/// 执行迁移的动作：调能力，返回它的返回值（没有动作返回 None）。
fn perform(mig: &Migration, cap: &Mock, who: &str) -> Result<Option<String>, String> {
    let Some(action) = &mig.action else {
        return Ok(None);
    };
    let text = |key: &str| -> Result<String, String> {
        match action.params.get(key) {
            Some(Value::Text(t)) => Ok(t.replace("{{候选人.邮箱}}", who)),
            _ => Err(format!(
                "迁移 {}→{} 的动作 {} 缺字符串参数 {key}",
                mig.from, mig.to, action.capability
            )),
        }
    };
    match action.capability.as_str() {
        "send_mail" => {
            let attachments = match action.params.get("attachments") {
                Some(Value::List(items)) => items
                    .iter()
                    .map(|s| s.replace("{{候选人.邮箱}}", who))
                    .collect(),
                _ => Vec::new(),
            };
            let id = cap.send_mail(
                &text("to")?,
                &text("subject")?,
                &text("body")?,
                &attachments,
            );
            Ok(Some(id))
        }
        other => Err(format!(
            "迁移 {}→{} 的动作里没有能力「{other}」",
            mig.from, mig.to
        )),
    }
}

fn advance(
    flow: &Flow,
    cap: &Mock,
    state: &mut State,
    today: &str,
    step_only: bool,
) -> Result<Report, String> {
    let mut report = Report::default();
    let people: Vec<String> = state.keys().cloned().collect();
    for who in people {
        let entry = state.get_mut(&who).expect("状态里应有此人");
        let from_state = entry.state.clone();
        let migs: Vec<&Migration> = flow
            .migrations
            .iter()
            .filter(|m| m.from == from_state)
            .collect();
        for mig in migs {
            if !mig.automatic() {
                report.todos.push(format!(
                    "  · {who} 停在「{}」：{}（{}）",
                    entry.state, mig.trigger, mig.executor
                ));
                continue;
            }
            if !holds(mig.check.as_ref(), cap, &who, entry, today) {
                let known = mig
                    .check
                    .as_ref()
                    .map(|c| c.capability.as_str())
                    .unwrap_or("无检查");
                report.waiting.push(format!(
                    "  ⋯ {who}：{}→{} 检查不成立（{known}）",
                    mig.from, mig.to
                ));
                continue;
            }
            let result = perform(mig, cap, &who)?; // 动作先做：失败就不挪格
            entry.state = mig.to.clone();
            entry.since = today.to_string();
            entry.trail.push(mig.to.clone());
            let mut effect = mig.effect.clone();
            match result {
                Some(r) if effect.is_empty() => effect = r,
                Some(r) => effect = format!("{effect}：{r}"),
                None => {}
            }
            report
                .advanced
                .push(format!("  ✓ {who}：{} → {}（{effect}）", mig.from, mig.to));
            if step_only {
                return Ok(report);
            }
        }
    }
    Ok(report)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let step_only = std::env::args().any(|a| a == "--step");
    let today = Local::now().format("%Y-%m-%d").to_string();

    // 读流程 → 读状态（没有就从原始数据重建）→ 推 → 落盘
    let flow: Flow = serde_yaml::from_str(FLOW_YAML)?;
    let cap = Mock;
    let state_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("qtrecurit")
        .join("state.json");

    let mut state = load_state(&state_path);
    if state.is_empty() {
        state = derive(&flow, &cap, &today);
        println!("本地无状态：从飞书侧原始数据重建（{} 人）", state.len());
    }

    let report = advance(&flow, &cap, &mut state, &today, step_only)
        .map_err(|e| format!("能力调用失败：{e}"))?;
    save_state(&state_path, &state)?;

    println!("推进 {} 条", report.advanced.len());
    for line in &report.advanced {
        println!("{line}");
    }
    println!(
        "待办 {} 条（执行者是人或半自动，执行器不代劳）",
        report.todos.len()
    );
    for line in &report.todos {
        println!("{line}");
    }
    if !report.waiting.is_empty() {
        println!("等条件 {} 条", report.waiting.len());
        for line in &report.waiting {
            println!("{line}");
        }
    }
    println!("状态已落盘：{}", state_path.display());

    // status：每人现在在哪一格
    println!();
    for (who, entry) in &state {
        println!("  {who:26} {}（{} 起）", entry.state, entry.since);
    }
    Ok(())
}
