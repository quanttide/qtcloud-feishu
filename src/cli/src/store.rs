//! 落盘契约：一份响应一个文件，路径只由资源、动作与页序决定。
//!
//! ```text
//! <root>/
//!   raw/<资源>/<动作>/<页序>.json             飞书原样响应（含 ok/data 信封）
//!   raw/mail/attachments/<附件id>/<原文件名>   附件原件
//!   cursor/<资源>.json                        {"cursor": …, "updated_at": …}
//! ```
//!
//! 三条规矩（这一层存在的全部理由）：
//!
//! 1. **原样**：写进 `raw/` 的是平台回的那份负载，不解信封、不拆数组、不聚合、不按月切。
//!    要什么形状，读的时候再算——写的时候不许加工。
//! 2. **无业务名**：路径里只有资源名、动作名、飞书 id 与页序。不出现邮箱、人名、来源。
//! 3. **游标只在整段拉完后前进**：`set_cursor` 由调用方在拉完时调，中途失败不推游标。

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

/// 页序固定四位：raw/<资源>/<动作>/0001.json
const SEQ_WIDTH: usize = 4;

pub struct RawStore {
    root: PathBuf,
}

impl RawStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        RawStore { root: root.into() }
    }

    pub fn raw_dir(&self, resource: &str, action: &str) -> PathBuf {
        self.root.join("raw").join(resource).join(action)
    }

    pub fn response_path(&self, resource: &str, action: &str, seq: usize) -> PathBuf {
        self.raw_dir(resource, action)
            .join(format!("{seq:0SEQ_WIDTH$}.json"))
    }

    pub fn attachment_path(&self, attachment_id: &str, filename: &str) -> PathBuf {
        self.root
            .join("raw")
            .join("mail")
            .join("attachments")
            .join(attachment_id)
            .join(filename)
    }

    pub fn cursor_path(&self, resource: &str) -> PathBuf {
        self.root.join("cursor").join(format!("{resource}.json"))
    }

    /// 下一个页序：接着已有文件往下排，重跑同一次调用不会覆盖历史页。
    pub fn next_seq(&self, resource: &str, action: &str) -> usize {
        match fs::read_dir(self.raw_dir(resource, action)) {
            Ok(entries) => {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                    .count()
                    + 1
            }
            Err(_) => 1,
        }
    }

    /// 把一份原样响应写成一个文件。
    pub fn save_response(
        &self,
        resource: &str,
        action: &str,
        raw: &Value,
    ) -> std::io::Result<PathBuf> {
        let path = self.response_path(resource, action, self.next_seq(resource, action));
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, serde_json::to_string_pretty(raw)?)?;
        Ok(path)
    }

    pub fn save_attachment(
        &self,
        attachment_id: &str,
        filename: &str,
        data: &[u8],
    ) -> std::io::Result<PathBuf> {
        let path = self.attachment_path(attachment_id, filename);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(&path, data)?;
        Ok(path)
    }

    pub fn set_cursor(&self, resource: &str, cursor: &str) -> std::io::Result<PathBuf> {
        let path = self.cursor_path(resource);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let payload = serde_json::json!({
            "cursor": cursor,
            "updated_at": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        });
        fs::write(&path, serde_json::to_string_pretty(&payload)?)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        crate::test_support::tmp_dir(name)
    }

    #[test]
    fn 页序接着排_重跑不覆盖() {
        let store = RawStore::new(tmp("store_seq"));
        let raw = serde_json::json!({"ok": true, "data": {}});
        let first = store.save_response("mail", "list", &raw).unwrap();
        let second = store.save_response("mail", "list", &raw).unwrap();
        assert!(first.ends_with("0001.json"));
        assert!(second.ends_with("0002.json"));
    }

    #[test]
    fn 附件与游标各落各的() {
        let store = RawStore::new(tmp("store_att"));
        let path = store
            .save_attachment("m-1-att-0", "简历.txt", b"hi")
            .unwrap();
        assert_eq!(path.parent().unwrap().file_name().unwrap(), "m-1-att-0");
        assert_eq!(fs::read(&path).unwrap(), b"hi");
        store.set_cursor("mail", "2026-10-02").unwrap();
        let text = fs::read_to_string(store.cursor_path("mail")).unwrap();
        assert!(text.contains("2026-10-02"), "{text}");
    }
}
