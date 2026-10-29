//! vc 域：会议搜索与单场会议详情，两个动作。
//!
//! ```text
//! vc_meeting_search    vc +search --as user        按参与人 + 时间范围找会议
//! vc_meeting_get       vc meeting get              按会议 id 取详情（带参会人）
//! ```

use super::action::{Action, Arg, Kind};

pub const RESOURCE: &str = "vc";

pub const ACTIONS: &[Action] = &[
    Action {
        name: "vc_meeting_search",
        resource: RESOURCE,
        command: &["vc", "+search", "--as", "user"],
        params: &[
            ("participant_id", Arg::Flag("--participant-ids")),
            ("start", Arg::Flag("--start")),
            ("end", Arg::Flag("--end")),
            ("page_size", Arg::Flag("--page-size")),
            ("page_token", Arg::Flag("--page-token")),
        ],
        needs_any: &["participant_id"],
        landed: "meeting-search",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
        logged: false,
    },
    Action {
        name: "vc_meeting_get",
        resource: RESOURCE,
        command: &["vc", "meeting", "get", "--with-participants", "true"],
        params: &[("meeting_id", Arg::Flag("--meeting-id"))],
        needs_any: &[],
        landed: "meeting-get",
        kind: Kind::Page,
        variants: &[],
        variants_key: "",
        defaults: &[],
        logged: false,
    },
];
