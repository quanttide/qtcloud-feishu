//! 执行器：三条规矩——动作先做失败不挪格、检查成立才推、人 / 半自动只记待办。

mod support;

use qtcloud_feishu_cli::engine::Engine;
use qtcloud_feishu_cli::flow::{self, advance, derive};
use serde_json::Map;

fn scene(name: &str) -> (support::Fake, qtcloud_feishu_cli::flow::Flow) {
    let fake = support::Fake::new(name);
    let flow = flow::parse(support::FLOW).unwrap();
    (fake, flow)
}

#[test]
fn 动作先做_检查成立才推() {
    let (fake, flow) = scene("flow_advance");
    let backend = fake.backend();
    let engine = Engine::new(&backend, support::definitions());
    let context = flow::context_of(&flow, None);

    let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();
    assert_eq!(state.len(), 2);
    assert_eq!(state["herui@example.com"].state, "已进群");
    assert_eq!(state["chenmo@example.com"].state, "投递");

    let report = advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();
    assert_eq!(report.advanced.len(), 1, "只有投递那位推得动");
    assert_eq!(report.advanced[0].0, "chenmo@example.com");
    assert!(
        report.advanced[0].3.contains("记发送时间"),
        "副作用带上返回值：{:?}",
        report.advanced[0]
    );
    assert_eq!(state["chenmo@example.com"].state, "二维码已发");
    assert_eq!(state["chenmo@example.com"].since, "2026-10-13");
    assert!(
        report.waiting.is_empty(),
        "没有检查不成立：{:?}",
        report.waiting
    );
    assert_eq!(report.todos.len(), 1, "人执行的迁移只记待办");
    assert_eq!(report.todos[0].0, "herui@example.com");
    assert_eq!(report.todos[0].2, "建清单");
    assert!(
        report.advanced[0].3.contains("mock-chenmo"),
        "发出去的 message_id 落进推进记录"
    );
}

#[test]
fn 动作失败就不挪格_整轮报错() {
    let (fake, flow) = scene("flow_fail");
    fake.fail_send();
    let backend = fake.backend();
    let engine = Engine::new(&backend, support::definitions());
    let context = flow::context_of(&flow, None);
    let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();

    let err = advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap_err();
    assert!(err.contains("发信失败"), "{err}");
    assert_eq!(state["chenmo@example.com"].state, "投递", "失败不挪格");
    assert!(
        !support::landed(&fake.store())
            .iter()
            .any(|p| p.starts_with("raw/mail/send")),
        "发信没落盘成成功的记录"
    );
}

#[test]
fn 迟到的扫码照样追认() {
    let fake = support::Fake::new("flow_late");
    fake.set_mails(&[("chenmo@example.com", "2026-10-02")]);
    fake.set_members(&["chenmo@example.com"]);
    let flow = flow::parse(support::FLOW).unwrap();
    let backend = fake.backend();
    let engine = Engine::new(&backend, support::definitions());
    let context = flow::context_of(&flow, None);

    let state = derive(&engine, &flow, &context, "2026-10-13").unwrap();
    assert_eq!(
        state["chenmo@example.com"].state, "已进群",
        "群里出现了就认，不管多久"
    );
    assert_eq!(state["chenmo@example.com"].since, "2026-10-13");
}

#[test]
fn 收集节缺进人就明确报错() {
    let fake = support::Fake::new("flow_config");
    let backend = fake.backend();
    let engine = Engine::new(&backend, support::definitions());
    let flow = flow::parse("流程: 招聘\n收集:\n  范围: [收件箱]\n状态: [投递]\n").unwrap();
    let err = derive(&engine, &flow, &Map::new(), "2026-10-13").unwrap_err();
    assert!(err.contains("进人"), "{err}");
}
