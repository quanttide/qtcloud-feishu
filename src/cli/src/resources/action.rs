//! 基础单元的形状与规矩：一个动作登记什么、参数怎么变成命令行、取回来的东西长什么样。
//!
//! **登记就是契约**——十三个动作没有第二处签名：每个动作长什么样写在它所属的资源文件里
//! （`mail.rs` / `im.rs` / `vc.rs` / `task.rs` / `doc.rs` / `base.rs`），这里只提供登记用的形状
//! 与共用的转换。后端不逐一写方法：动作名 → 登记，一次派发。
//!
//! 一个动作登记六件事：
//!
//! - `command`：固定命令段，如 `["mail", "+triage"]`
//! - `params`：参数名 → 怎么变成 argv 片段（普通 flag 直接给，要改写值的给函数）
//! - `needs_any`：必填参数（写一个就够的那些，如 chat_id / user_id）
//! - `landed`：落盘目录名：`raw/<资源>/<这个名字>/<页序>.json`
//! - `kind`：`Page`＝一页负载；`Files`＝一批原件（只有取附件是这一类）
//! - `variants` / `variants_key`：同一个动作换命令段（如 task 的三种取法）

use serde_json::{Map, Value};
use std::path::PathBuf;

/// 一页响应：负载 + 翻页信息（原样信封已经落盘，见 [`crate::store`]）。
#[derive(Debug, Clone)]
pub struct Page {
    pub payload: Value,
    pub has_more: bool,
    pub page_token: String,
}

impl Page {
    pub fn new(raw: &Value) -> Self {
        let data = raw.get("data").cloned().unwrap_or(Value::Null);
        let has_more = data
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let page_token = data
            .get("page_token")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        Page {
            payload: data,
            has_more,
            page_token,
        }
    }
}

/// 落盘的附件原件：飞书给的 id + 原文件名 + 本机路径。
#[derive(Debug, Clone)]
pub struct Attachment {
    pub attachment_id: String,
    pub filename: String,
    pub path: PathBuf,
}

/// 一次基础单元调用的产出：一页负载，或一批附件原件。
#[derive(Debug, Clone)]
pub enum Payload {
    Page(Page),
    Files(Vec<Attachment>),
}

impl Payload {
    /// 负载本体（附件那类没有负载，回一页空的）。
    pub fn value(&self) -> &Value {
        match self {
            Payload::Page(page) => &page.payload,
            Payload::Files(_) => &Value::Null,
        }
    }
}

/// 参数怎么变成 argv 片段。
///
/// 前七种是**通用形状**（逗号串、布尔文字、只有 false 才带…），名字直说形状；
/// 通用形状凑不出来的，用 `Raw` 把这条命令自己的怪癖写成函数——那个函数住在它所属的
/// 资源文件里，不往共用处加名字（这是实验室学到的一课）。
#[derive(Clone, Copy)]
pub enum Arg {
    /// `--flag <值>`
    Flag(&'static str),
    /// `--flag a,b,c`（表值拼逗号串）
    Csv(&'static str),
    /// `--flag true|false`（布尔渲染成文字）
    BoolText(&'static str),
    /// 值明确为 false 才带 `--flag`
    FalseOnly(&'static str),
    /// 值明确为 false 才带 `<flag>=false`
    FalseEq(&'static str),
    /// `--user-id <值> --as user`
    UserIdAsUser,
    /// 给了字典就序列化成 JSON，给了字符串就原样传
    JsonOrText(&'static str),
    /// 这条命令自己的怪癖：函数直接给完整片段（含 flag）
    Raw(fn(&Value) -> Vec<String>),
}

/// 动作的产出形状。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 一页 JSON 负载。
    Page,
    /// 一批原件（取附件）：先取地址，再把字节落到 attachments/。
    Files,
}

/// 一个飞书动作的登记。
pub struct Action {
    pub name: &'static str,
    pub resource: &'static str,
    pub command: &'static [&'static str],
    pub params: &'static [(&'static str, Arg)],
    pub needs_any: &'static [&'static str],
    pub landed: &'static str,
    pub kind: Kind,
    /// 同一个动作的几种取法：值 → (命令段, 落盘目录名)。
    pub variants: &'static [(&'static str, &'static [&'static str], &'static str)],
    /// 挑变体的那个参数（不变成 flag）。
    pub variants_key: &'static str,
    /// 契约默认值：调用方没给时补上。
    pub defaults: &'static [(&'static str, Value)],
}

impl Action {
    /// 参数 → （argv, 落盘目录名）。参数名与必填由 `Backend::call` 先校验。
    pub fn argv(&self, values: &Map<String, Value>) -> (Vec<String>, String) {
        let mut command: Vec<String> = self.command.iter().map(|s| s.to_string()).collect();
        let mut landed = self.landed.to_string();
        if !self.variants.is_empty() {
            let key = values
                .get(self.variants_key)
                .and_then(Value::as_str)
                .filter(|k| self.variants.iter().any(|(v, _, _)| v == k))
                .unwrap_or(self.variants[0].0);
            if let Some((_, command_seg, landing)) =
                self.variants.iter().find(|(v, _, _)| *v == key)
            {
                command = command_seg.iter().map(|s| s.to_string()).collect();
                landed = landing.to_string();
            }
        }
        for (name, arg) in self.params {
            let Some(value) = values.get(*name) else {
                continue;
            };
            if is_absent(value) {
                continue; // 没给就不带这个 flag——缺省值由平台说了算
            }
            match arg {
                Arg::Flag(flag) => {
                    command.push(flag.to_string());
                    command.push(text(value));
                }
                Arg::Csv(flag) => {
                    command.push(flag.to_string());
                    command.push(csv_join(value));
                }
                Arg::BoolText(flag) => {
                    command.push(flag.to_string());
                    command.push(
                        if value == &Value::Bool(false) {
                            "false"
                        } else {
                            "true"
                        }
                        .to_string(),
                    );
                }
                Arg::FalseOnly(flag) => {
                    if value == &Value::Bool(false) {
                        command.push(flag.to_string());
                    }
                }
                Arg::FalseEq(flag) => {
                    if value == &Value::Bool(false) {
                        command.push(flag.to_string());
                    }
                }
                Arg::UserIdAsUser => {
                    command.push("--user-id".to_string());
                    command.push(text(value));
                    command.push("--as".to_string());
                    command.push("user".to_string());
                }
                Arg::JsonOrText(flag) => {
                    command.push(flag.to_string());
                    command.push(match value {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    });
                }
                Arg::Raw(f) => command.extend(f(value)),
            }
        }
        command.push("--format".to_string());
        command.push("json".to_string());
        (command, landed)
    }

    pub fn param_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self.params.iter().map(|(n, _)| *n).collect();
        if !self.variants_key.is_empty() {
            names.push(self.variants_key);
        }
        names
    }
}

/// 表值 → 逗号串（标量当单元素）。
pub fn csv_join(value: &Value) -> String {
    match value {
        Value::Array(items) => items.iter().map(text).collect::<Vec<_>>().join(","),
        other => text(other),
    }
}

/// 值算不算「没给」：null / 空串 / 空表。
pub fn is_absent(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

/// 值 → 命令行上的文字（字符串原样，其余按 JSON 写）。
pub fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}
