//! 执行器：三条规矩——动作先做失败不挪格、检查成立才推、人 / 半自动只记待办。

mod fake_lark_cli;

use fake_lark_cli::Fake;
use qtcloud_feishu_cli::engine::Engine;
use qtcloud_feishu_cli::flow::{self, advance, derive};
use serde_json::Map;

fn scene(name: &str) -> (Fake, flow::Flow) {
    let fake = Fake::new(name);
    let flow = flow::parse(fake_lark_cli::FLOW).unwrap();
    (fake, flow)
}

#[test]
fn 动作先做检查成立才推() {
    let (fake, flow) = scene("flow_advance");
    let backend = fake.backend();
    let engine = Engine::new(&backend, fake_lark_cli::definitions());
    let context = flow::context_of(&flow, None);

    let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();
    assert_eq!(state.len(), 2);
    assert_eq!(state["herui@example.com"].state, "已进群");
    assert_eq!(state["chenmo@example.com"].state, "投递");

    let report = advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();
    assert_eq!(report.advanced.len(), 1, "只有投递那位推得动");
    assert_eq!(report.advanced[0].0, "chenmo@example.com");
    assert_eq!(state["chenmo@example.com"].state, "邀请已发");
    assert_eq!(state["chenmo@example.com"].since, "2026-10-13");
    assert!(
        report.waiting.is_empty(),
        "没有检查不成立的：{:?}",
        report.waiting
    );
    assert!(
        report.advanced[0].3.contains("mock-chenmo"),
        "发出去的邮件号落进推进记录"
    );
}

#[test]
fn 人执行的迁移只记待办() {
    let (fake, flow) = scene("flow_todo");
    let backend = fake.backend();
    let engine = Engine::new(&backend, fake_lark_cli::definitions());
    let context = flow::context_of(&flow, None);
    let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();

    let report = advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();
    assert_eq!(report.todos.len(), 1);
    assert_eq!(report.todos[0].0, "herui@example.com");
    assert_eq!(report.todos[0].4, "人", "待办里带着执行者");
}

#[test]
fn 动作失败就不挪格且整轮报错() {
    let (fake, flow) = scene("flow_fail");
    fake.fail_send();
    let backend = fake.backend();
    let engine = Engine::new(&backend, fake_lark_cli::definitions());
    let context = flow::context_of(&flow, None);
    let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();

    let err = advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap_err();
    assert!(err.contains("发信失败"), "{err}");
    assert_eq!(state["chenmo@example.com"].state, "投递", "失败不挪格");
}

#[test]
fn 迟到的扫码照样追认() {
    let fake = Fake::new("flow_late");
    fake.set_mails(&[("chenmo@example.com", "2026-10-02")]);
    fake.set_members(&["chenmo@example.com"]);
    let flow = flow::parse(fake_lark_cli::FLOW).unwrap();
    let backend = fake.backend();
    let engine = Engine::new(&backend, fake_lark_cli::definitions());
    let context = flow::context_of(&flow, None);

    let state = derive(&engine, &flow, &context, "2026-10-13").unwrap();
    assert_eq!(
        state["chenmo@example.com"].state, "已进群",
        "群里出现了就认，不管多久"
    );
}

#[test]
fn 起草停格等人确认() {
    let fake = Fake::new("flow_draft_hold");
    let flow = flow::parse(fake_lark_cli::DRAFT_FLOW).unwrap();
    let backend = fake.backend();
    let engine = Engine::new(&backend, fake_lark_cli::definitions());
    let context = flow::context_of(&flow, None);
    let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();

    advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();
    assert_eq!(state["chenmo@example.com"].state, "待确认", "起草后停格");
    assert_eq!(
        state["chenmo@example.com"].artifacts["草稿id"], "mock-draft-001",
        "草稿 id 存进状态产物"
    );
    assert!(
        fake.argv_log()
            .iter()
            .all(|line| !line.contains("+draft-send")),
        "没确认就不发草稿"
    );

    // 第二轮：迁移被评估，但已确认检查不成立——在等，不推
    let report = advance(&engine, &flow, &mut state, &context, "2026-10-14", false).unwrap();
    assert!(
        report
            .waiting
            .iter()
            .any(|(_, _, why)| why.contains("已确认")),
        "未确认的迁移在等：{:?}",
        report.waiting
    );
    assert_eq!(state["chenmo@example.com"].state, "待确认", "仍在等确认");
}

#[test]
fn 确认后下一轮才把草稿发出去() {
    let fake = Fake::new("flow_draft_confirm");
    let flow = flow::parse(fake_lark_cli::DRAFT_FLOW).unwrap();
    let backend = fake.backend();
    let engine = Engine::new(&backend, fake_lark_cli::definitions());
    let context = flow::context_of(&flow, None);
    let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();
    advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();

    flow::confirm(&mut state, "chenmo@example.com", "2026-10-14").unwrap();
    assert_eq!(state["chenmo@example.com"].confirmed, "2026-10-14");
    assert!(
        state["chenmo@example.com"]
            .trail
            .contains(&"已确认".to_string()),
        "确认进轨迹，state 文件就是留痕"
    );

    advance(&engine, &flow, &mut state, &context, "2026-10-14", false).unwrap();
    assert_eq!(state["chenmo@example.com"].state, "已发");
    let log = fake.argv_log();
    let send = log
        .iter()
        .find(|line| line.contains("+draft-send"))
        .expect("确认后该发草稿");
    assert!(send.contains("--draft-id mock-draft-001"), "{send}");
}

#[test]
fn 重复确认明确报错() {
    let mut state = flow::State::new();
    state.insert("chenmo@example.com".to_string(), flow::Entry::default());
    flow::confirm(&mut state, "chenmo@example.com", "2026-10-14").unwrap();
    let err = flow::confirm(&mut state, "chenmo@example.com", "2026-10-15").unwrap_err();
    assert!(err.contains("确认过"), "{err}");

    let nobody = flow::confirm(&mut state, "nobody@example.com", "2026-10-15").unwrap_err();
    assert!(nobody.contains("没有这个人"), "{nobody}");
}

#[test]
fn 旧格式的_状态文件照样读回() {
    let dir = fake_lark_cli::tmp_dir("flow_state_legacy");
    let path = dir.join("state.json");
    std::fs::write(
        &path,
        r#"{"chenmo@example.com":{"since":"2026-10-13","state":"投递","trail":["投递"]}}"#,
    )
    .unwrap();
    let state = flow::load_state(&path);
    assert_eq!(state["chenmo@example.com"].state, "投递");
    assert!(state["chenmo@example.com"].artifacts.is_empty());
    assert!(state["chenmo@example.com"].confirmed.is_empty());

    // 产物与确认写回后存盘读回都在
    let mut state = state;
    flow::confirm(&mut state, "chenmo@example.com", "2026-10-14").unwrap();
    flow::save_state(&path, &state).unwrap();
    let reloaded = flow::load_state(&path);
    assert_eq!(reloaded["chenmo@example.com"].confirmed, "2026-10-14");
}

#[test]
fn 收集节缺进人就明确报错() {
    let fake = Fake::new("flow_config");
    let backend = fake.backend();
    let engine = Engine::new(&backend, fake_lark_cli::definitions());
    let flow = flow::parse("流程: 招聘\n收集:\n  范围: [收件箱]\n状态: [投递]\n").unwrap();
    let err = derive(&engine, &flow, &Map::new(), "2026-10-13").unwrap_err();
    assert!(err.contains("进人"), "{err}");
}
