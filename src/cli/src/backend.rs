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
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Map, Value, json};

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

pub struct CliRunner {
    timeout: Duration,
}

impl Runner for CliRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<Output, String> {
        // ETXTBSY：刚写好的脚本紧接着执行，内核可能短暂拒绝（文件刚被写、句柄还在）。
        // 等一下重试，而不是把瞬时状态当失败。
        let mut child = None;
        for attempt in 0..5 {
            match Command::new(program)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(spawned) => {
                    child = Some(spawned);
                    break;
                }
                // ETXTBSY 在 Linux 上是 26
                Err(err) if err.raw_os_error() == Some(26) && attempt < 4 => {
                    std::thread::sleep(Duration::from_millis(50 * (attempt + 1) as u64));
                }
                Err(err) => {
                    return Err(format!(
                        "找不到 {program}：{err}（PATH 里没有；离线自测把假 lark-cli 放到 PATH 前面）"
                    ));
                }
            }
        }
        let mut child = child.expect("重试后总能得到子进程或已报错返回");
        // stdout/stderr 在子线程里读走，免得管道写满把子进程据死；主循环按超时轮询收尾
        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();
        let out = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(pipe) = stdout_pipe.as_mut() {
                use std::io::Read;
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        });
        let err = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(pipe) = stderr_pipe.as_mut() {
                use std::io::Read;
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        });
        let deadline = Instant::now() + self.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(format!(
                        "{program} 超时（{} 秒）：写动作没有超时会无限挂起，已终止子进程",
                        self.timeout.as_secs()
                    ));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(err) => break Err(format!("等 {program} 结束失败：{err}")),
            }
        };
        let stdout = String::from_utf8_lossy(&out.join().unwrap_or_default()).to_string();
        let stderr = String::from_utf8_lossy(&err.join().unwrap_or_default()).to_string();
        let status = status?;
        Ok(Output {
            stdout,
            stderr,
            success: status.success(),
        })
    }
}

pub struct Backend {
    runner: Box<dyn Runner>,
    program: String,
    store: Option<RawStore>,
    location: BTreeMap<String, String>,
    timeout: Duration,
    ops_log: Option<std::path::PathBuf>,
}

/// 操作日志里不落的参数键：正文类内容不进日志（隐私口径照旧）。
const OPS_PRIVATE_PARAMS: &[&str] = &["body", "markdown", "text", "content"];

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
        let timeout = std::env::var("LARK_CLI_TIMEOUT")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(30));
        Backend {
            runner: Box::new(CliRunner { timeout }),
            program,
            store: store_root.map(RawStore::new),
            location,
            timeout,
            ops_log: None,
        }
    }

    /// 用 XDG 缺省的镜像根建后端（`~/.local/share/qtcloud-feishu` 一类）。
    pub fn with_mirror(program: Option<String>) -> Result<Self, String> {
        let mut backend = Backend::new(program, Some(crate::paths::mirror_dir()?));
        backend.ops_log = Some(crate::paths::ops_log_path()?);
        Ok(backend)
    }

    /// 换超时（测试要验超时分支，不想等真的三十秒）。
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self.runner = Box::new(CliRunner { timeout });
        self
    }

    /// 指定操作日志位置（不给就不写日志）。
    pub fn with_ops_log(mut self, path: std::path::PathBuf) -> Self {
        self.ops_log = Some(path);
        self
    }

    pub fn store(&self) -> Option<&RawStore> {
        self.store.as_ref()
    }

    /// 跑一个动作：校验参数 → 拼 argv → 跑 → 落盘 → 写操作日志 → （附件那类）取字节。
    pub fn call(&self, name: &str, params: Map<String, Value>) -> Result<Payload, String> {
        let log_params = if resources::get(name).is_some_and(|action| action.logged) {
            Some(params.clone())
        } else {
            None
        };
        let (action, argv, landed) = self.prepare(name, params)?;
        let raw = self.run_json(&argv)?;
        if let Some(store) = &self.store {
            store
                .save_response(action.resource, &landed, &raw)
                .map_err(|err| format!("落盘失败：{err}"))?;
        }
        let page = crate::resources::Page::new(&raw);
        if action.logged
            && let Some(params) = &log_params
        {
            self.write_ops_log(name, action, params, &page)?;
        }
        match action.kind {
            resources::Kind::Page => Ok(Payload::Page(page)),
            resources::Kind::Files => Ok(Payload::Files(self.download(&page)?)),
        }
    }

    /// dry-run：只拼命令行，不跑子进程、不落盘、不写日志。参数校验与 `call` 共用一份。
    pub fn plan(&self, name: &str, params: Map<String, Value>) -> Result<Vec<String>, String> {
        let (_action, argv, _landed) = self.prepare(name, params)?;
        Ok(argv)
    }

    /// 校验参数、补位置参数与默认值、拼 argv——`call` 与 `plan` 共用的前半段。
    fn prepare(
        &self,
        name: &str,
        mut params: Map<String, Value>,
    ) -> Result<(&'static resources::Action, Vec<String>, String), String> {
        let action = resources::get(name).ok_or_else(|| format!("没有这个动作：{name}"))?;
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
        Ok((action, argv, landed))
    }

    /// 写动作成功后追加一条操作日志：时间、动作、资源、参数（正文类排除）、返回里的 id 与状态。
    /// 写入失败要把待补记的那条报出来——动作已经发生，不静默也不假装没做过。
    fn write_ops_log(
        &self,
        name: &str,
        action: &resources::Action,
        params: &Map<String, Value>,
        page: &crate::resources::Page,
    ) -> Result<(), String> {
        let Some(path) = &self.ops_log else {
            return Ok(()); // 没配操作日志就不写（镜像后端由 with_mirror 配上）
        };
        let filtered: Map<String, Value> = params
            .iter()
            .filter(|(key, _)| !OPS_PRIVATE_PARAMS.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        let id = ["draft_id", "message_id", "id"]
            .iter()
            .find_map(|key| page.payload.get(*key).and_then(Value::as_str));
        let entry = json!({
            "time": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            "action": name,
            "resource": action.resource,
            "params": filtered,
            "id": id,
            "status": "ok",
        });
        let line = format!("{entry}\n");
        use std::io::Write;
        let append = || -> std::io::Result<()> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            file.write_all(line.as_bytes())
        };
        append().map_err(|err| {
            format!(
                "{name} 已执行并落盘，但操作日志写入失败：{err}；待补记：{}",
                line.trim_end()
            )
        })
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
