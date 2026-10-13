//! 端到端：真跑进程，飞书那一侧用假 lark-cli；一条定义与一条流程各走一遍。

mod support;

use qtcloud_feishu_cli::engine::Engine;
use qtcloud_feishu_cli::flow;
use serde_json::{Map, json};

fn engine_and_flow(name: &str) -> (support::Fake, Engine<'static>) {
    let fake = support::Fake::new(name);
    let backend: &'static qtcloud_feishu_cli::backend::Backend =
        Box::leak(Box::new(fake.backend()));
    let engine = Engine::new(backend, support::definitions());
    (fake, engine)
}

#[test]
fn 定义_两步串起来_字段起成我们的话() {
    let (fake, engine) = engine_and_flow("e2e_define");
    let mut context = Map::new();
    context.insert("邮箱".to_string(), json!("hr@example.com"));
    context.insert("文件夹".to_string(), json!("收件箱"));

    let outcome = engine.call("读投递邮件", &context).unwrap();
    assert_eq!(outcome.rows.len(), 2);
    assert_eq!(outcome.rows[0]["发件人"], "chenmo@example.com");
    assert_eq!(outcome.rows[0]["日期"], "2026-10-02");
    assert_eq!(outcome.rows[0]["主题"], "应聘数据工程师");
    assert_eq!(
        outcome.cursor.as_deref(),
        Some("2026-10-03"),
        "增量游标取最新的日期"
    );

    let files = support::landed(&fake.store());
    assert!(
        files.contains(&"raw/mail/list/0001.json".to_string()),
        "{files:?}"
    );
    assert!(
        files.contains(&"raw/mail/messages/0001.json".to_string()),
        "两步各落一份：{files:?}"
    );
}

#[test]
fn 增量_带游标再拉就只剩新的() {
    let (_fake, engine) = engine_and_flow("e2e_cursor");
    let mut context = Map::new();
    context.insert("邮箱".to_string(), json!("hr@example.com"));
    context.insert("文件夹".to_string(), json!("收件箱"));
    context.insert("游标".to_string(), json!("2026-10-03"));
    let outcome = engine.call("读投递邮件", &context).unwrap();
    assert!(
        outcome.rows.is_empty(),
        "没有比游标更新的：{:?}",
        outcome.rows
    );
}

#[test]
fn 表列名两种写法都认() {
    let (_fake, engine) = engine_and_flow("e2e_survey");
    let mut context = Map::new();
    context.insert("邮箱列表".to_string(), json!(["chenmo@example.com"]));
    let outcome = engine.call("查调研记录", &context).unwrap();
    assert_eq!(outcome.rows[0]["邮箱"], "chenmo@example.com");
    assert_eq!(outcome.rows[0]["提交时间"], "2026-10-04");
}

#[test]
fn 流程_从原始数据重建_再推一轮_状态落盘() {
    let (fake, engine) = engine_and_flow("e2e_flow");
    let flow = flow::parse(support::FLOW).unwrap();
    let context = flow::context_of(&flow, None);

    let mut state = flow::derive(&engine, &flow, &context, "2026-10-13").unwrap();
    assert_eq!(state.len(), 2, "群里那位 + 收件箱里的新投递");

    let report = flow::advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();
    assert_eq!(report.advanced.len(), 1);
    assert_eq!(state["chenmo@example.com"].state, "二维码已发");

    let state_path = fake.dir.join("state.json");
    flow::save_state(&state_path, &state).unwrap();
    let reloaded = flow::load_state(&state_path);
    assert_eq!(reloaded["chenmo@example.com"].state, "二维码已发");
    assert!(
        support::landed(&fake.store())
            .iter()
            .any(|p| p.starts_with("raw/mail/send")),
        "发信落盘"
    );
}
