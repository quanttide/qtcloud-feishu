//! 基础单元后端：子进程调 lark-cli。
//!
//! 同一份定义两种活法，差别只在 PATH：
//!
//! ```text
//! PATH 前面放假 lark-cli  → 离线夹具
//! PATH 里是真 lark-cli    → 打真飞书
//! ```
//!
//! 命令怎么写只由各资源的登记决定（`resources/`）；拼 argv、跑子进程、解信封、落盘、
//! 取附件字节由 [`Backend::call`] 一次做完。加一个动作不在这里改代码。
//!
//! 值全部从参数与环境来，代码里不写任何邮箱、群 id、人名、关键词：
//! `LARK_CLI` / `LARK_MAILBOX` / `LARK_BASE_TOKEN` / `LARK_TABLE_ID` / `LARK_CHAT_ID`。
//!
//! 落盘根不给就不落盘（纯读）；要缺省位置用 [`Backend::with_mirror`]，目录规则见 [`crate::paths`]。

use std::collections::BTreeMap;
use std::process::Command;

use serde_json::{Map, Value};

use crate::resources::{self, Attachment, Payload};
use crate::store::RawStore;

/// 资源位置参数：调用方没给就用实例的（构造参数或环境变量）。
pub const LOCATION: &[&str] = &["mailbox", "chat_id", "base_token", "table_id"];

/// 跑一条命令得到的输出。
#[derive(Debug, Clone)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// 谁把命令跑起来。真实现 spawn 子进程；测试给一个假的（不 fork，直接回预制输出）。
pub trait Runner {
    fn run(&self, program: &str, args: &[String]) -> Result<Output, String>;
}

pub struct CliRunner;

impl Runner for CliRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<Output, String> {
        let output = Command::new(program).args(args).output().map_err(|err| {
            format!("找不到 {program}：{err}（PATH 里没有；离线自测把假 lark-cli 放到 PATH 前面）")
        })?;
        Ok(Output {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        })
    }
}

pub struct Backend {
    runner: Box<dyn Runner>,
    program: String,
    store: Option<RawStore>,
    location: BTreeMap<String, String>,
}

impl Backend {
    /// 真后端：`program` 缺省 `lark-cli`（或环境变量 `LARK_CLI`）。
    pub fn new(program: Option<String>, store_root: Option<std::path::PathBuf>) -> Self {
        let program = program
            .or_else(|| std::env::var("LARK_CLI").ok())
            .unwrap_or_else(|| "lark-cli".to_string());
        let mut location = BTreeMap::new();
        for (key, env, fallback) in [
            ("mailbox", "LARK_MAILBOX", "me"),
            ("chat_id", "LARK_CHAT_ID", "oc_mock_chat"),
            ("base_token", "LARK_BASE_TOKEN", "mock-base-token"),
            ("table_id", "LARK_TABLE_ID", "mock-table-id"),
        ] {
            location.insert(
                key.to_string(),
                std::env::var(env).unwrap_or_else(|_| fallback.to_string()),
            );
        }
        Backend {
            runner: Box::new(CliRunner),
            program,
            store: store_root.map(RawStore::new),
            location,
        }
    }

    /// 用 XDG 缺省的镜像根建后端（`~/.local/share/qtcloud-feishu` 一类）。
    pub fn with_mirror(program: Option<String>) -> Result<Self, String> {
        Ok(Backend::new(program, Some(crate::paths::mirror_dir()?)))
    }

    pub fn store(&self) -> Option<&RawStore> {
        self.store.as_ref()
    }

    /// 跑一个动作：校验参数 → 拼 argv → 跑 → 落盘 → （附件那类）取字节。
    pub fn call(&self, name: &str, params: Map<String, Value>) -> Result<Payload, String> {
        let action = resources::get(name).ok_or_else(|| format!("没有这个动作：{name}"))?;
        let mut params = params;
        let allowed = action.param_names();
        let unknown: Vec<String> = params
            .keys()
            .filter(|key| !allowed.contains(&key.as_str()))
            .cloned()
            .collect();
        if !unknown.is_empty() {
            return Err(format!("{name} 不认识这些参数：{unknown:?}"));
        }
        if !action.needs_any.is_empty()
            && !action.needs_any.iter().any(|key| {
                params
                    .get(*key)
                    .is_some_and(|value| !resources::action::is_absent(value))
            })
        {
            return Err(format!("{name} 需要 {}", action.needs_any.join(" 或 ")));
        }
        for key in LOCATION {
            if action.param_names().contains(key)
                && params.get(*key).is_none_or(resources::action::is_absent)
                && let Some(value) = self.location.get(*key)
            {
                params.insert((*key).to_string(), Value::String(value.clone()));
            }
        }
        for (key, value) in action.defaults {
            params
                .entry((*key).to_string())
                .or_insert_with(|| value.clone());
        }

        let (argv, landed) = action.argv(&params);
        let raw = self.run_json(&argv)?;
        if let Some(store) = &self.store {
            store
                .save_response(action.resource, &landed, &raw)
                .map_err(|err| format!("落盘失败：{err}"))?;
        }
        let page = crate::resources::Page::new(&raw);
        match action.kind {
            resources::Kind::Page => Ok(Payload::Page(page)),
            resources::Kind::Files => Ok(Payload::Files(self.download(&page)?)),
        }
    }

    /// 跑一条命令，回（负载, 原样信封）。信封就是你写进 raw/ 的那份东西。
    fn run_json(&self, argv: &[String]) -> Result<Value, String> {
        let output = self.runner.run(&self.program, argv)?;
        let text = if output.success {
            &output.stdout
        } else {
            &output.stderr
        };
        let payload = extract_json(text).ok_or_else(|| {
            format!(
                "{} {} 输出不是 JSON：{}",
                self.program,
                argv.join(" "),
                text.trim()
            )
        })?;
        if !output.success || payload.get("ok").and_then(Value::as_bool) != Some(true) {
            let message = payload
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or(text.trim());
            return Err(format!(
                "{} {} 失败：{message}",
                self.program,
                argv.iter().take(2).cloned().collect::<Vec<_>>().join(" ")
            ));
        }
        Ok(payload)
    }

    /// 取附件原件：先取地址，再把字节落到 attachments/。
    fn download(&self, page: &crate::resources::Page) -> Result<Vec<Attachment>, String> {
        let mut landed = Vec::new();
        let empty = Vec::new();
        let items = page
            .payload
            .get("attachments")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        for item in items {
            let id = item
                .get("attachment_id")
                .or_else(|| item.get("id"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let name = item
                .get("filename")
                .and_then(Value::as_str)
                .unwrap_or(&id)
                .to_string();
            let bytes = match item.get("download_url").and_then(Value::as_str) {
                Some(url) => fetch_bytes(url)?,
                None => Vec::new(),
            };
            let path = match &self.store {
                Some(store) => store
                    .save_attachment(&id, &name, &bytes)
                    .map_err(|err| format!("附件落盘失败：{err}"))?,
                None => std::path::PathBuf::new(),
            };
            landed.push(Attachment {
                attachment_id: id,
                filename: name,
                path,
            });
        }
        Ok(landed)
    }
}

/// download_url → 字节。`file://` 与本地路径直接读（地址里的百分号转义交给 url 库解）；
/// http(s) 走一个请求。
fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    match url::Url::parse(url) {
        Ok(parsed) if parsed.scheme() == "file" => {
            let path = parsed
                .to_file_path()
                .map_err(|_| format!("附件地址看不懂：{url}"))?;
            std::fs::read(&path).map_err(|err| format!("读附件失败：{}：{err}", path.display()))
        }
        Ok(parsed) if parsed.scheme().starts_with("http") => {
            let response = ureq::get(url)
                .call()
                .map_err(|err| format!("取附件失败：{url}：{err}"))?;
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut response.into_reader(), &mut bytes)
                .map_err(|err| format!("读附件字节失败：{url}：{err}"))?;
            Ok(bytes)
        }
        _ => std::fs::read(url).map_err(|err| format!("读附件失败：{url}：{err}")),
    }
}

/// 从输出里抠 JSON。真 lark-cli 会在 JSON 前后打进度行，直接 parse 会失败。
pub fn extract_json(text: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return Some(value);
    }
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if start < end {
        serde_json::from_str::<Value>(&text[start..=end]).ok()
    } else {
        None
    }
}
