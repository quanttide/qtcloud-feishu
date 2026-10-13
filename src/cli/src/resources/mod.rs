//! 基础单元（门面）：六资源十四个动作——动作登记就是契约，按飞书资源分文件。
//!
//! ```text
//! mail.rs / im.rs / vc.rs / task.rs / doc.rs / base.rs   各资源的动作登记
//! action.rs         登记的形状：Action + Page / Attachment + 参数 → argv
//! ```
//!
//! 十四个动作的名字与参数只写在这些登记里——没有第二处签名，后端也不逐一写方法
//! （动作名 → 登记，一次派发）。加一个动作 = 改它所属的那个资源文件。

use serde_json::{Map, Value};

pub mod action;
pub mod base;
pub mod doc;
pub mod im;
pub mod mail;
pub mod task;
pub mod vc;

pub use action::{Action, Attachment, Kind, Page, Payload};

const MODULES: &[&[Action]] = &[
    mail::ACTIONS,
    im::ACTIONS,
    vc::ACTIONS,
    task::ACTIONS,
    doc::ACTIONS,
    base::ACTIONS,
];

/// 十四个动作的登记，供后端与命令行按名字查。
pub fn all() -> impl Iterator<Item = &'static Action> {
    MODULES.iter().flat_map(|actions| actions.iter())
}

/// 按名字找一个动作。
pub fn get(name: &str) -> Option<&'static Action> {
    all().find(|action| action.name == name)
}

/// 把一个动作拉到底：一页一页取（每页已由后端落盘），最后推游标。
///
/// `cursor_key` 给了才动游标（列表类动作用资源名当键）；整段拉完才前进，中途出错不推——
/// 下次还能从断点续。这是 `data/intention` 里说的「受控拉取」那一层。
pub fn pull(
    backend: &crate::backend::Backend,
    name: &str,
    params: &Map<String, Value>,
    cursor_key: Option<&str>,
    max_pages: usize,
) -> Result<Vec<Page>, String> {
    let mut pages = Vec::new();
    let mut token: Option<String> = None;
    loop {
        let mut page_params: Map<String, Value> = params.clone();
        if let Some(token) = &token {
            page_params.insert("page_token".to_string(), Value::String(token.clone()));
        }
        let page = match backend.call(name, page_params)? {
            Payload::Page(page) => page,
            Payload::Files(_) => return Err(format!("{name} 不是分页动作")),
        };
        let (has_more, next) = (page.has_more, page.page_token.clone());
        pages.push(page);
        if !has_more || next.is_empty() || pages.len() >= max_pages {
            break;
        }
        token = Some(next);
    }
    if let (Some(key), Some(store), Some(last)) = (cursor_key, backend.store(), pages.last()) {
        store
            .set_cursor(key, &last.page_token)
            .map_err(|err| format!("游标落盘失败：{err}"))?;
    }
    Ok(pages)
}
