//! mail 域：列摘要、取正文、发信、下附件——四个动作，只有发信是写。
//!
//! 落盘：`raw/mail/<动作>/<页序>.json`；附件原件另落 `raw/mail/attachments/<附件id>/<原文件名>`。
//!
//! ```text
//! mail_list                  mail +triage         按文件夹（收件箱 / Spam …）列摘要
//! mail_messages              mail +messages       按 message_id 取正文
//! mail_attachment_download   mail user_mailbox.message.attachments download_url
//! mail_send                  mail +send --confirm-send
//! ```

use serde_json::{Value, json};

use super::action::{Action, Arg, Kind, text};

pub const RESOURCE: &str = "mail";

/// 这条命令的怪癖：文件夹要包成 `{"folder": …}` 的 JSON 给 `--filter`。
fn folder_filter(value: &Value) -> Vec<String> {
    vec![
        "--filter".to_string(),
        json!({ "folder": text(value) }).to_string(),
    ]
}

pub const ACTIONS: &[Action] = &[
    Action {
        name: "mail_list",
        resource: RESOURCE,
        command: &["mail", "+triage"],
        params: &[
            ("mailbox", Arg::Flag("--mailbox")),
            ("query", Arg::Flag("--query")),
            ("page_size", Arg::Flag("--max")),
            ("page_token", Arg::Flag("--page-token")),
            ("folder", Arg::Raw(folder_filter)),
        ],
        needs_any: &[],
        landed: "list",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
    },
    Action {
        name: "mail_messages",
        resource: RESOURCE,
        command: &["mail", "+messages"],
        params: &[
            ("mailbox", Arg::Flag("--mailbox")),
            ("message_ids", Arg::Csv("--message-ids")),
            ("html", Arg::FalseEq("--html=false")),
        ],
        needs_any: &[],
        landed: "messages",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
    },
    Action {
        name: "mail_send",
        resource: RESOURCE,
        command: &["mail", "+send", "--confirm-send"],
        params: &[
            ("to", Arg::Flag("--to")),
            ("subject", Arg::Flag("--subject")),
            ("body", Arg::Flag("--body")),
            ("mailbox", Arg::Flag("--mailbox")),
            ("attachments", Arg::Csv("--attach")),
        ],
        needs_any: &[],
        landed: "send",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
    },
    Action {
        name: "mail_attachment_download",
        resource: RESOURCE,
        command: &["mail", "user_mailbox.message.attachments", "download_url"],
        params: &[
            ("mailbox", Arg::Flag("--user-mailbox-id")),
            ("message_id", Arg::Flag("--message-id")),
            ("attachment_ids", Arg::Csv("--attachment-ids")),
        ],
        needs_any: &[],
        landed: "attachments",
        kind: Kind::Files,
        variants: &[],
        variants_key: "",
        defaults: &[],
    },
];
