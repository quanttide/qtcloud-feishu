//! im 域：群/单聊的消息与成员，三个动作。
//!
//! ```text
//! im_chat_messages      im +chat-messages-list    群按 chat_id、单聊按 user_id（写一个就够）
//! im_chat_members       im +chat-members-list     群成员
//! im_messages_search    im +messages-search       按发言人 / 被 @ 人搜（用户身份拉）
//! ```
//!
//! `with_reactions` 是反向 flag：只有明确写 false 才带 `--no-reactions`；不给就随平台默认。

use serde_json::Value;

use super::action::{Action, Arg, Kind};

pub const RESOURCE: &str = "im";

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
