//! mail 域：列摘要、取正文、起草、直发、发草稿、下附件——六个动作，三个写（起草/直发/发草稿）。
//!
//! 落盘：`raw/mail/<动作>/<页序>.json`；附件原件另落 `raw/mail/attachments/<附件id>/<原文件名>`。
//!
//! ```text
//! mail_list                  mail +triage         按文件夹（收件箱 / Spam …）列摘要
//! mail_messages              mail +messages       按 message_id 取正文
//! mail_attachment_download   mail user_mailbox.message.attachments download_url
//! mail_draft_create          mail +send           起草（不带 --confirm-send，只生成草稿）
//! mail_send                  mail +send --confirm-send   确认后直发
//! mail_draft_send            mail +draft-send --as user  发已存在的草稿
//! ```
//!
//! 草稿确认制由执行器表达：起草后停格，人确认后发草稿。`body` 与 `body_file` 互斥、
//! 附件必须是相对当前工作目录的路径——登记层不表达这些，命令面做校验。
use super::action::{Action, Arg, Kind, text};
use serde_json::{Value, json};
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
        logged: false,
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
        logged: false,
    },
    Action {
        name: "mail_draft_create",
        resource: RESOURCE,
        command: &["mail", "+send"],
        params: &[
            ("to", Arg::Flag("--to")),
            ("subject", Arg::Flag("--subject")),
            ("body", Arg::Flag("--body")),
            ("body_file", Arg::Flag("--body-file")),
            ("mailbox", Arg::Flag("--mailbox")),
            ("attachments", Arg::Csv("--attach")),
        ],
        needs_any: &[],
        landed: "draft-create",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
        logged: true,
    },
    Action {
        name: "mail_send",
        resource: RESOURCE,
        command: &["mail", "+send", "--confirm-send"],
        params: &[
            ("to", Arg::Flag("--to")),
            ("subject", Arg::Flag("--subject")),
            ("body", Arg::Flag("--body")),
            ("body_file", Arg::Flag("--body-file")),
            ("mailbox", Arg::Flag("--mailbox")),
            ("attachments", Arg::Csv("--attach")),
        ],
        needs_any: &[],
        landed: "send",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
        logged: true,
    },
    Action {
        name: "mail_draft_send",
        resource: RESOURCE,
        command: &["mail", "+draft-send", "--as", "user"],
        params: &[("draft_ids", Arg::Csv("--draft-id"))],
        needs_any: &["draft_ids"],
        landed: "draft-send",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
        logged: true,
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
        logged: false,
    },
];
