//! 量潮招聘流程调度器——实验室 Python 实现的同逻辑重写（Rust 版）。
//!
//! 与 `examples/quanttide-feishu-lab` 的 `flow/state/runner` 一一对应：
//! 流程是数据（YAML 内嵌本文件，改流程改这段字串），执行器读它推格——
//! 检查成立才推、动作先做失败不挪格、人 / 半自动只记待办。
//! 能力层（契约 + mock 夹具）就近放在本文件，与实验室 capabilities/ 对应，示例自包含。
//! 发群二维码这条动作用「先取」在发信之前取一次群邀请链接，填进邮件正文模板。
//!
//! 跑法（在 `src/cli` 下）：
//!     cargo run --example qtrecurit            # 一轮推到底，再打印每人在哪格
//!     cargo run --example qtrecurit -- --step  # 只推一步

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::mail_adapter::MailAdapter;
use crate::mock::Mock;

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
#      动作可以用「先取」在自己之前多取几样东西（如群邀请链接）：每样按「能力」调一次，
#      取到的值按 as 起名，参数模板里用 {{名字}} 引用。
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
      先取:                     # 发信之前先把群邀请链接取来，填进下面的正文模板
        - 能力: chat_link
          as: 邀请链接
      参数:
        to: "{{候选人.邮箱}}"
        subject: 量潮实训基地 · 群二维码
        body: |-
          你好，扫码直接进群（群二维码见附件）。
          也可以点这个链接进群：{{邀请链接}}
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
    /// 执行本动作之前先取的值：各调一次能力，按 `as` 起名，供参数模板引用。
    #[serde(rename = "先取", default)]
    fetch: Vec<Fetch>,
    #[serde(rename = "参数")]
    params: BTreeMap<String, Value>,
}

/// 动作之前先取的一样东西（如群邀请链接）。
#[derive(Deserialize)]
struct Fetch {
    #[serde(rename = "能力")]
    capability: String,
    #[serde(rename = "参数", default)]
    params: BTreeMap<String, Value>,
    #[serde(rename = "as")]
    as_name: String,
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
fn derive(flow: &Flow, cap: &impl MailAdapter, today: &str) -> State {
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

// ── 能力层：契约与夹具实现（对应实验室 capabilities/）────────────
// 换实现不换执行器，签名收 `&impl MailAdapter`；契约说流程词汇，
// 平台词汇（folder id、命令行参数）只活在实现里。

mod mail_adapter {
    //! Mail 适配器：流程与邮件能力之间的适配契约。
    //!
    //! 适配器的分工——契约说流程词汇（文件夹、候选人邮箱），平台词汇（folder id、
    //! 命令行参数）只活在实现里；YAML 里只出现能力名，不出现任何命令。
    //! 换实现（mock / 将来的 lark-cli 子进程）不动执行器。方法与实验室 Python
    //! 能力层的同名能力一一对应，跨语言对照靠它。

    /// 一封邮件（流程用到的字段）。
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Mail {
        pub sender: String,
        pub date: String,
    }

    /// 流程要用到的四种能力。参数与返回值都用普通数据，不给调用方任何平台概念。
    pub trait MailAdapter {
        /// 读某个文件夹的邮件。读哪个文件夹由调用方决定，能力层不做取舍；
        /// 数据里没构造过的文件夹读出来是空——是「没数据」，不是「不该读」。
        fn read_mail(&self, folder: &str) -> Vec<Mail>;

        /// 读本实例所指群的成员，返回邮箱。
        fn read_group_members(&self) -> Vec<String>;

        /// 取本实例所指群的邀请链接（群分享链接）。
        ///
        /// 真实现是 `lark-cli im chats link --chat-id …`（API `POST /open-apis/im/v1/chats/:chat_id/link`，
        /// 返回 `share_link` / `expire_time` / `is_permanent`，可指定 `validity_period`）。
        /// 两条边界已知：调用者得是群成员，且**外部群机器人身份取不到**（232033：
        /// 机器人无权管理外部群）——实训基地群是外部群，真取要用 HR 的用户身份。
        fn chat_link(&self) -> Result<String, String>;

        /// 发一封邮件，返回 `message_id`；失败返回错误消息，执行器据此不挪格。
        fn send_mail(
            &self,
            to: &str,
            subject: &str,
            body: &str,
            attachments: &[String],
        ) -> Result<String, String>;
    }
}

mod mock {
    //! 能力层的 mock 实现：数据构造时注入，零外部 IO。
    //!
    //! 与实验室 `capabilities/mock.py` 对应，两件事分开放：
    //! 契约在 [`crate::mail_adapter`]，这里只管「拿一份数据当能力使」；
    //! `Mock::default()` 是实验室那套夹具（跨语言对照的 `message_id` 算法也对齐），
    //! 换场景用 [`Mock::with`] 注入另一份数据，不改本文件。

    use std::cell::{Cell, RefCell};
    use std::collections::BTreeMap;

    use crate::mail_adapter::{Mail, MailAdapter};

    /// 一封已发出的邮件：mock 把它记下来，调用方与测试能断言「发了什么」。
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SentMail {
        pub to: String,
        pub subject: String,
        pub body: String,
        pub attachments: Vec<String>,
        pub message_id: String,
    }

    /// 可注入数据的能力 mock。
    ///
    /// 夹具不再是编译期常量：文件夹、群成员、读哪个群都由构造给，
    /// 同一进程里可以有多套互不相干的数据，测试之间互不污染。
    pub struct Mock {
        /// 文件夹名 → 邮件（发件人, 日期）
        pub folders: BTreeMap<String, Vec<(String, String)>>,
        /// 群 id → 成员邮箱
        pub groups: BTreeMap<String, Vec<String>>,
        /// 本实例读哪个群（与真实现把 chat_id 放构造参数的做法对齐，方法签名不带群概念）
        pub chat_id: String,
        /// 已发出的邮件（对应 Python 夹具层写进 `outbox/` 的那叠文件）
        pub sent: RefCell<Vec<SentMail>>,
        /// 置真时 `send_mail` 一律失败：用来测「动作先做，失败不挪格」
        pub fail_send: Cell<bool>,
    }

    impl Mock {
        /// 任意数据构造：换场景换一份数据，不改这个文件。
        pub fn with(
            folders: BTreeMap<String, Vec<(String, String)>>,
            groups: BTreeMap<String, Vec<String>>,
            chat_id: &str,
        ) -> Self {
            Self {
                folders,
                groups,
                chat_id: chat_id.to_string(),
                sent: RefCell::new(Vec::new()),
                fail_send: Cell::new(false),
            }
        }
    }

    impl Default for Mock {
        /// 实验室夹具：收件箱四封投递 + 实训基地群三个成员。
        fn default() -> Self {
            let folders = BTreeMap::from([(
                "收件箱".to_string(),
                vec![
                    ("chenmo@example.com".to_string(), "2026-10-02".to_string()),
                    (
                        "linyizhou@example.com".to_string(),
                        "2026-10-03".to_string(),
                    ),
                    ("suxiao@example.com".to_string(), "2026-10-03".to_string()),
                    ("herui@example.com".to_string(), "2026-10-04".to_string()),
                ],
            )]);
            let groups = BTreeMap::from([(
                "oc_mock".to_string(),
                vec![
                    "herui@example.com".to_string(),
                    "yuanyu@example.com".to_string(),
                    "hyacinth@example.com".to_string(),
                ],
            )]);
            Self::with(folders, groups, "oc_mock")
        }
    }

    impl MailAdapter for Mock {
        fn read_mail(&self, folder: &str) -> Vec<Mail> {
            self.folders
                .get(folder)
                .map(|mails| {
                    mails
                        .iter()
                        .map(|(sender, date)| Mail {
                            sender: sender.clone(),
                            date: date.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        }

        fn read_group_members(&self) -> Vec<String> {
            self.groups.get(&self.chat_id).cloned().unwrap_or_default()
        }

        fn chat_link(&self) -> Result<String, String> {
            // 真链接形如 https://applink.feishu.cn/client/chat/chatter/add_by_link?link_token=<uuid>
            Ok(format!(
                "https://applink.feishu.cn/client/chat/chatter/add_by_link?link_token=mock-{}",
                self.chat_id
            ))
        }

        fn send_mail(
            &self,
            to: &str,
            subject: &str,
            body: &str,
            attachments: &[String],
        ) -> Result<String, String> {
            if self.fail_send.get() {
                return Err(format!("mock 注入的失败：发给 {to} 的「{subject}」"));
            }
            let message_id = message_id(to, subject);
            self.sent.borrow_mut().push(SentMail {
                to: to.to_string(),
                subject: subject.to_string(),
                body: body.to_string(),
                attachments: attachments.to_vec(),
                message_id: message_id.clone(),
            });
            Ok(message_id)
        }
    }

    /// `message_id` 生成：算法与 Python mock 同步，跨语言对照靠它，动它等于断对照。
    fn message_id(to: &str, subject: &str) -> String {
        let mut digest: u64 = 0;
        for ch in to.chars() {
            digest = (digest * 131 + u64::from(u32::from(ch))) % 0xFFFFFF;
        }
        let local = to.split('@').next().unwrap_or(to);
        let head: String = subject.chars().take(12).filter(|c| *c != ' ').collect();
        format!("mock-{local}_{digest:06x}-{head}")
    }
}

// ── 执行器：一轮推一步（对应 runner.py）────────────────────────

#[derive(Default, Debug)]
struct Report {
    advanced: Vec<String>,
    todos: Vec<String>,
    waiting: Vec<String>,
}

/// 检查成立吗。没写检查 = 无条件：人这一格就该推。
fn holds(
    check: Option<&Check>,
    cap: &impl MailAdapter,
    who: &str,
    entry: &Entry,
    today: &str,
) -> bool {
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
fn perform(mig: &Migration, cap: &impl MailAdapter, who: &str) -> Result<Option<String>, String> {
    let Some(action) = &mig.action else {
        return Ok(None);
    };
    // 先取：动作之前多跑一次能力，取到的值按 as 起名，填进参数模板（如群邀请链接）
    let mut vars: BTreeMap<String, String> = BTreeMap::new();
    for fetch in &action.fetch {
        if !fetch.params.is_empty() {
            return Err(format!(
                "迁移 {}→{} 的先取「{}」还不接参数（参数由本实例给）",
                mig.from, mig.to, fetch.capability
            ));
        }
        let value = match fetch.capability.as_str() {
            "chat_link" => cap.chat_link()?,
            other => {
                return Err(format!(
                    "迁移 {}→{} 的先取里没有能力「{other}」",
                    mig.from, mig.to
                ));
            }
        };
        vars.insert(fetch.as_name.clone(), value);
    }
    let text = |key: &str| -> Result<String, String> {
        match action.params.get(key) {
            Some(Value::Text(t)) => Ok(fill(t, who, &vars)),
            _ => Err(format!(
                "迁移 {}→{} 的动作 {} 缺字符串参数 {key}",
                mig.from, mig.to, action.capability
            )),
        }
    };
    match action.capability.as_str() {
        "send_mail" => {
            let attachments = match action.params.get("attachments") {
                Some(Value::List(items)) => items.iter().map(|s| fill(s, who, &vars)).collect(),
                _ => Vec::new(),
            };
            let id = cap.send_mail(
                &text("to")?,
                &text("subject")?,
                &text("body")?,
                &attachments,
            )?;
            Ok(Some(id))
        }
        other => Err(format!(
            "迁移 {}→{} 的动作里没有能力「{other}」",
            mig.from, mig.to
        )),
    }
}

/// 参数模板取值：`{{候选人.邮箱}}` 与「先取」起的名字（如 `{{邀请链接}}`）。
fn fill(template: &str, who: &str, vars: &BTreeMap<String, String>) -> String {
    let mut text = template.replace("{{候选人.邮箱}}", who);
    for (name, value) in vars {
        text = text.replace(&format!("{{{{{name}}}}}"), value);
    }
    text
}

fn advance(
    flow: &Flow,
    cap: &impl MailAdapter,
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
    let cap = Mock::default();
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一份场景数据：收件箱 + 群成员都可指定——数据是注入的，不是编译期常量。
    fn scene(inbox: &[(&str, &str)], members: &[&str]) -> Mock {
        let folders = BTreeMap::from([(
            "收件箱".to_string(),
            inbox
                .iter()
                .map(|(e, d)| (e.to_string(), d.to_string()))
                .collect::<Vec<_>>(),
        )]);
        let groups = BTreeMap::from([(
            "oc_mock".to_string(),
            members.iter().map(|m| m.to_string()).collect::<Vec<_>>(),
        )]);
        Mock::with(folders, groups, "oc_mock")
    }

    fn flow() -> Flow {
        serde_yaml::from_str(FLOW_YAML).expect("内嵌流程应能解析")
    }

    /// 流程最关键的约束：动作先做，失败就不挪格——用注入的失败把它钉住。
    #[test]
    fn 动作失败就不挪格() {
        let flow = flow();
        let cap = scene(&[("newbie@example.com", "2026-10-12")], &[]);

        // 正常路径先各发一封：留得下记录、断言得出内容
        let id = cap
            .send_mail(
                "a@example.com",
                "量潮实训基地 · 群二维码",
                "正文",
                &["群二维码.png".to_string()],
            )
            .expect("正常发信应成功");
        {
            let sent = cap.sent.borrow();
            assert_eq!(sent.len(), 1);
            assert_eq!(sent[0].message_id, id);
            assert_eq!(sent[0].attachments, ["群二维码.png"]);
        }

        cap.fail_send.set(true);
        let mut state = derive(&flow, &cap, "2026-10-12");
        let before = state["newbie@example.com"].trail.clone();

        let err = advance(&flow, &cap, &mut state, "2026-10-12", false)
            .expect_err("发信失败应让这一轮直接报错");
        assert!(err.contains("mock 注入的失败"), "错误应带上失败原因：{err}");
        let entry = &state["newbie@example.com"];
        assert_eq!(entry.state, "投递", "动作失败，格子不该动");
        assert_eq!(entry.trail, before, "轨迹不该多出一格");
        assert_eq!(cap.sent.borrow().len(), 1, "失败的那次不该留下已发记录");
    }

    /// 发信之前先取群邀请链接，并按 `{{邀请链接}}` 填进正文模板。
    #[test]
    fn 发信前先取邀请链接并填进正文() {
        let flow = flow();
        let cap = scene(&[("newbie@example.com", "2026-10-12")], &[]);
        let mut state = derive(&flow, &cap, "2026-10-12");

        advance(&flow, &cap, &mut state, "2026-10-12", false).expect("应能推一步");

        let sent = cap.sent.borrow();
        assert_eq!(sent.len(), 1, "投递那位该收到一封");
        let mail = &sent[0];
        assert_eq!(mail.subject, "量潮实训基地 · 群二维码");
        assert!(
            mail.body.contains("link_token=mock-oc_mock"),
            "正文里该填进邀请链接：{}",
            mail.body
        );
        assert!(
            mail.body.contains("扫码直接进群"),
            "模板原话要留着：{}",
            mail.body
        );
        assert_eq!(mail.attachments, ["群二维码.png"], "二维码附件照旧");
    }

    /// 迟到的扫码照样算（复活）：成员是运行期注入的，不在默认夹具里。
    #[test]
    fn 迟到的扫码照样追认() {
        let flow = flow();
        let alone = scene(&[("late@example.com", "2026-10-12")], &[]);
        let mut state = derive(&flow, &alone, "2026-10-12");
        assert_eq!(state["late@example.com"].state, "投递");

        // 二维码发出十天还没进群 → 无响应
        let entry = state.get_mut("late@example.com").unwrap();
        entry.state = "无响应".to_string();
        entry.since = "2026-10-02".to_string();
        entry.trail = vec![
            "投递".to_string(),
            "二维码已发".to_string(),
            "无响应".to_string(),
        ];

        // 后来扫码进群：同一场景重新注入成员（数据是构造进来的，能改）
        let joined = scene(&[("late@example.com", "2026-10-12")], &["late@example.com"]);
        let report =
            advance(&flow, &joined, &mut state, "2026-10-12", false).expect("追认不该失败");
        assert!(
            report
                .advanced
                .iter()
                .any(|l| l.contains("无响应 → 已进群")),
            "应有一条追认：{report:?}"
        );
        assert_eq!(state["late@example.com"].state, "已进群");
        assert_eq!(state["late@example.com"].since, "2026-10-12");
    }
}
