//! task 域：一条动作三种取法——我负责 / 与我相关 / 清单。
//!
//! ```text
//! task_list(scope="mine")        task +get-my-tasks --page-all
//! task_list(scope="related")     task +get-related-tasks --page-all
//! task_list(scope="tasklists")   task tasklists list --as user --page-all
//! ```
//!
//! 命令段与落盘目录都按 scope 分（`raw/task/mine/…`）；`complete` 是文字 true/false 而非 flag。

use super::action::{Action, Arg, Kind};

pub const RESOURCE: &str = "task";

pub const ACTIONS: &[Action] = &[Action {
    name: "task_list",
    resource: RESOURCE,
    command: &["task"],
    params: &[
        ("complete", Arg::BoolText("--complete")),
        ("page_token", Arg::Flag("--page-token")),
    ],
    needs_any: &[],
    landed: "mine",
    kind: Kind::Page,
    variants: &[
        ("mine", &["task", "+get-my-tasks", "--page-all"], "mine"),
        (
            "related",
            &["task", "+get-related-tasks", "--page-all"],
            "related",
        ),
        (
            "tasklists",
            &["task", "tasklists", "list", "--as", "user", "--page-all"],
            "tasklists",
        ),
    ],
    variants_key: "scope",
    defaults: &[],
}];
