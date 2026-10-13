//! doc 域：找文档与取正文，两个动作。
//!
//! ```text
//! drive_search    drive +search     按编辑时间或创建时间找（写一个就够）
//! docs_fetch      docs +fetch       按文档 token 导出正文（detail / doc_format 决定格式）
//! ```

use super::action::{Action, Arg, Kind};

pub const RESOURCE: &str = "doc";

pub const ACTIONS: &[Action] = &[
    Action {
        name: "drive_search",
        resource: RESOURCE,
        command: &["drive", "+search"],
        params: &[
            ("edited_since", Arg::Flag("--edited-since")),
            ("created_since", Arg::Flag("--created-since")),
            ("page_size", Arg::Flag("--page-size")),
            ("page_token", Arg::Flag("--page-token")),
        ],
        needs_any: &["edited_since", "created_since"],
        landed: "drive-search",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
    },
    Action {
        name: "docs_fetch",
        resource: RESOURCE,
        command: &["docs", "+fetch"],
        params: &[
            ("doc", Arg::Flag("--doc")),
            ("detail", Arg::Flag("--detail")),
            ("doc_format", Arg::Flag("--doc-format")),
        ],
        needs_any: &[],
        landed: "docs-fetch",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
    },
];
