//! base 域：多维表格的记录，一个动作。
//!
//! ```text
//! base_records    base +record-list    按 base_token + table_id 列表；filter_json 给字典或 JSON 串
//! ```

use super::action::{Action, Arg, Kind};

pub const RESOURCE: &str = "base";

pub const ACTIONS: &[Action] = &[Action {
    name: "base_records",
    resource: RESOURCE,
    command: &["base", "+record-list"],
    params: &[
        ("base_token", Arg::Flag("--base-token")),
        ("table_id", Arg::Flag("--table-id")),
        ("offset", Arg::Flag("--offset")),
        ("limit", Arg::Flag("--limit")),
        // 查询条件在配置里就是一段 YAML（字典），序列化是这条动作自己的事
        ("filter_json", Arg::JsonOrText("--filter-json")),
    ],
    needs_any: &[],
    landed: "record-list",
    kind: Kind::Page,
    variants: &[],
    variants_key: "",
    defaults: &[],
}];
