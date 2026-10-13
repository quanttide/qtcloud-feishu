//! im 域：群/单聊的消息、成员与群链接，四个动作。
//!
//! ```text
//! im_chat_messages      im +chat-messages-list    群按 chat_id、单聊按 user_id（写一个就够）
//! im_chat_members       im +chat-members-list     群成员
//! im_messages_search    im +messages-search       按发言人 / 被 @ 人搜（用户身份拉）
//! im_chat_link          im chats link             取群分享链接（填进邀请信）
//! ```
//!
//! `with_reactions` 是反向 flag：只有明确写 false 才带 `--no-reactions`；不给就随平台默认。
//!
//! `im_chat_link` 的边界（实测）：调用者得是群成员；**外部群机器人身份取不到**（API 232033），
//! 要么用用户身份，要么别拿机器人试。`validity_period` 写进请求体（`--data`）：
//! `week` / `year` / `permanently`，不给就用群自己的设置。

use serde_json::{Value, json};

use super::action::{Action, Arg, Kind};

pub const RESOURCE: &str = "im";

/// 这条命令的请求体：`--data '{"validity_period": "…"}'`。
fn validity_period(value: &Value) -> Vec<String> {
    vec![
        "--data".to_string(),
        json!({ "validity_period": super::action::text(value) }).to_string(),
    ]
}

pub const ACTIONS: &[Action] = &[
    Action {
        name: "im_chat_messages",
        resource: RESOURCE,
        command: &["im", "+chat-messages-list"],
        params: &[
            ("chat_id", Arg::Flag("--chat-id")),
            ("page_size", Arg::Flag("--page-size")),
            ("page_token", Arg::Flag("--page-token")),
            ("order", Arg::Flag("--order")),
            ("with_reactions", Arg::FalseOnly("--no-reactions")),
            ("user_id", Arg::UserIdAsUser),
        ],
        needs_any: &["chat_id", "user_id"],
        landed: "chat-messages",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
    },
    Action {
        name: "im_chat_members",
        resource: RESOURCE,
        command: &["im", "+chat-members-list", "--page-all"],
        params: &[
            ("chat_id", Arg::Flag("--chat-id")),
            ("page_token", Arg::Flag("--page-token")),
        ],
        needs_any: &[],
        landed: "chat-members",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
    },
    Action {
        name: "im_chat_link",
        resource: RESOURCE,
        command: &["im", "chats", "link"],
        params: &[
            ("chat_id", Arg::Flag("--chat-id")),
            ("validity_period", Arg::Raw(validity_period)),
        ],
        needs_any: &[],
        landed: "chat-link",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
    },
    Action {
        name: "im_messages_search",
        resource: RESOURCE,
        command: &["im", "+messages-search", "--as", "user"],
        params: &[
            ("sender", Arg::Flag("--sender")),
            ("at_chatter", Arg::Flag("--at-chatter-ids")),
            ("chat_id", Arg::Flag("--chat-id")),
            ("page_size", Arg::Flag("--page-size")),
            ("page_token", Arg::Flag("--page-token")),
            ("with_reactions", Arg::FalseOnly("--no-reactions")),
        ],
        needs_any: &["sender", "at_chatter"],
        landed: "messages-search",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        // 契约默认：搜索不去表情
        defaults: &[("with_reactions", Value::Bool(false))],
    },
];
