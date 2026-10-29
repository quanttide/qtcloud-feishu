//! 端到端：真跑进程，飞书那一侧用假 lark-cli——一条两步定义与一条流程各走一遍。
//!
//! 跨的是后端 → 串联器 → 执行器整条链，所以不并进任何单个模块的测试文件。

mod fake_lark_cli;

use fake_lark_cli::Fake;
use qtcloud_feishu_cli::engine::Engine;
use qtcloud_feishu_cli::flow;
use serde_json::{Map, json};

fn engine_of(fake: &Fake) -> Engine<'static> {
    let backend: &'static qtcloud_feishu_cli::backend::Backend =
        Box::leak(Box::new(fake.backend()));
    Engine::new(backend, fake_lark_cli::definitions())
}

/// 定义与流程的上下文：键就是我们的话（邮箱、文件夹、游标…）。
fn context(pairs: &[(&str, &str)]) -> Map<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), json!(value)))
        .collect()
}

#[test]
fn 两步定义串起来() {
    let fake = Fake::new("e2e_define");
    let engine = engine_of(&fake);

    let outcome = engine
        .call(
            "读投递邮件",
            &context(&[("邮箱", "hr@example.com"), ("文件夹", "收件箱")]),
        )
        .unwrap();
    assert_eq!(outcome.rows.len(), 2, "两步：列摘要拿 id → 按 id 取正文");
    assert!(outcome.cursor.is_some(), "增量给了游标");
}

#[test]
fn 字段起成我们的话() {
    let fake = Fake::new("e2e_fields");
    let engine = engine_of(&fake);

    let outcome = engine
        .call(
            "读投递邮件",
            &context(&[("邮箱", "hr@example.com"), ("文件夹", "收件箱")]),
        )
        .unwrap();
    assert_eq!(outcome.rows[0]["发件人"], "chenmo@example.com");
    assert_eq!(outcome.rows[0]["日期"], "2026-10-02");
    assert_eq!(outcome.rows[0]["主题"], "应聘数据工程师");
    assert_eq!(
        outcome.cursor.as_deref(),
        Some("2026-10-03"),
        "游标取最新的那个日期"
    );

    let files = fake_lark_cli::landed(&fake.store());
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
fn 带游标再拉只剩新的() {
    let fake = Fake::new("e2e_cursor");
    let engine = engine_of(&fake);
    let outcome = engine
        .call(
            "读投递邮件",
            &context(&[
                ("邮箱", "hr@example.com"),
                ("文件夹", "收件箱"),
                ("游标", "2026-10-03"),
            ]),
        )
        .unwrap();
    assert!(
        outcome.rows.is_empty(),
        "没有比游标更新的：{:?}",
        outcome.rows
    );
}

#[test]
fn 表列名两种写法都认() {
    let fake = Fake::new("e2e_survey");
    let engine = engine_of(&fake);
    let outcome = engine
        .call(
            "查调研记录",
            &context(&[("邮箱列表", "chenmo@example.com")]),
        )
        .unwrap();
    assert_eq!(outcome.rows[0]["邮箱"], "chenmo@example.com");
    assert_eq!(outcome.rows[0]["提交时间"], "2026-10-04");
}

#[test]
fn 两步定义_先取链接再填进正文() {
    let fake = Fake::new("e2e_chat_link");
    let engine = engine_of(&fake);
    // 候选人是个嵌套的上下文（`{{候选人.邮箱}}` 逐层取），单独插进去
    let mut ctx = context(&[("邮箱", "hr@example.com")]);
    ctx.insert("候选人".to_string(), json!({"邮箱": "chenmo@example.com"}));
    let outcome = engine.call("发邀请信", &ctx).unwrap();
    assert!(
        outcome.single.contains_key("邮件id"),
        "{:?}",
        outcome.single
    );

    let log = fake.argv_log();
    let link = log
        .iter()
        .find(|line| line.contains("chats link"))
        .expect("该先取一次群链接");
    assert!(link.contains("--chat-id"), "取链接要带群：{link}");
    let send = log
        .iter()
        .find(|line| line.contains("mail +send"))
        .expect("该发一封");
    assert!(
        send.contains("link_token=mock-link-token-0001"),
        "正文里该填进链接：{send}"
    );
    assert!(send.contains("扫码直接进群"), "模板原话要留着：{send}");
    assert!(send.contains("--to chenmo@example.com"), "{send}");
}

#[test]
fn 起草定义两步取链接填正文但不发() {
    let fake = Fake::new("e2e_draft_create");
    let engine = engine_of(&fake);
    let mut ctx = context(&[("邮箱", "hr@example.com")]);
    ctx.insert("候选人".to_string(), json!({"邮箱": "chenmo@example.com"}));
    let outcome = engine.call("起草邀请信", &ctx).unwrap();
    assert_eq!(
        outcome.single["草稿id"], "mock-draft-001",
        "起草回草稿 id：{:?}",
        outcome.single
    );

    let log = fake.argv_log();
    let draft = log
        .iter()
        .find(|line| line.contains("mail +send"))
        .expect("该起草一封");
    assert!(
        !draft.contains("--confirm-send"),
        "起草不带确认，信不会真发出去：{draft}"
    );
    assert!(draft.contains("--to chenmo@example.com"), "{draft}");
    assert!(
        fake_lark_cli::landed(&fake.store())
            .contains(&"raw/mail/draft-create/0001.json".to_string()),
        "起草也落盘，Studio 才有草稿可看"
    );
}

#[test]
fn 人确认后按草稿id把草稿发出去() {
    let fake = Fake::new("e2e_draft_send");
    let engine = engine_of(&fake);
    let outcome = engine
        .call(
            "发草稿",
            &context(&[("邮箱", "hr@example.com"), ("草稿id", "mock-draft-001")]),
        )
        .unwrap();
    assert_eq!(outcome.single["邮件id"], "mock-from-draft");

    let log = fake.argv_log();
    let send = log
        .iter()
        .find(|line| line.contains("+draft-send"))
        .expect("该发一封草稿");
    assert!(send.contains("mail +draft-send --as user"), "{send}");
    assert!(send.contains("--draft-id mock-draft-001"), "{send}");
}

#[test]
fn 从原始数据重建后推一轮() {
    let fake = Fake::new("e2e_flow");
    let engine = engine_of(&fake);
    let flow = flow::parse(fake_lark_cli::FLOW).unwrap();
    let context = flow::context_of(&flow, None);

    let mut state = flow::derive(&engine, &flow, &context, "2026-10-13").unwrap();
    assert_eq!(state.len(), 2, "群里那位 + 收件箱里的新投递");

    let report = flow::advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();
    assert_eq!(report.advanced.len(), 1);
    assert_eq!(state["chenmo@example.com"].state, "邀请已发");
    assert!(
        fake_lark_cli::landed(&fake.store())
            .iter()
            .any(|p| p.starts_with("raw/mail/send")),
        "发信也落盘"
    );
}

#[test]
fn 状态落盘能读回() {
    let fake = Fake::new("e2e_state");
    let engine = engine_of(&fake);
    let flow = flow::parse(fake_lark_cli::FLOW).unwrap();
    let context = flow::context_of(&flow, None);
    let mut state = flow::derive(&engine, &flow, &context, "2026-10-13").unwrap();
    flow::advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();

    let path = fake.dir.join("state.json");
    flow::save_state(&path, &state).unwrap();
    let reloaded = flow::load_state(&path);
    assert_eq!(reloaded["chenmo@example.com"].state, "邀请已发");
    assert_eq!(reloaded["chenmo@example.com"].since, "2026-10-13");
}
