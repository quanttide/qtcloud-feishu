//! 基础单元：十六个动作的登记，以及参数怎么变成命令行。

mod fake_lark_cli;

use qtcloud_feishu_cli::resources;
use serde_json::{Map, json};

#[test]
fn 动作名不重复() {
    let actions: Vec<&resources::Action> = resources::all().collect();
    assert_eq!(actions.len(), 16, "基础单元是十六个动作");
    let mut names: Vec<&str> = actions.iter().map(|a| a.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 16);
}

#[test]
fn 每个动作都说清资源与落盘名() {
    for action in resources::all() {
        assert!(!action.resource.is_empty(), "{} 缺资源名", action.name);
        assert!(
            !action.landed.is_empty() || !action.variants.is_empty(),
            "{} 缺落盘目录名",
            action.name
        );
    }
}

#[test]
fn 起草直发发草稿三个动作各拼各的命令行() {
    let fake = fake_lark_cli::Fake::new("res_mail_variants");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("to".to_string(), json!("someone@example.com"));
    params.insert("subject".to_string(), json!("面试邀请"));
    params.insert("body".to_string(), json!("正文"));

    backend.call("mail_draft_create", params.clone()).unwrap();
    backend.call("mail_send", params).unwrap();
    backend
        .call(
            "mail_draft_send",
            json!({"draft_ids": ["mock-draft-001"]})
                .as_object()
                .unwrap()
                .clone(),
        )
        .unwrap();

    let log = fake.argv_log();
    let draft = log
        .iter()
        .find(|l| l.contains("+send") && !l.contains("confirm-send"))
        .expect("起草拼 +send");
    assert!(!draft.contains("--confirm-send"), "起草不带确认：{draft}");
    let send = log
        .iter()
        .find(|l| l.contains("--confirm-send"))
        .expect("直发带确认");
    assert!(send.contains("mail +send --confirm-send"), "{send}");
    let draft_send = log
        .iter()
        .find(|l| l.contains("+draft-send"))
        .expect("发草稿拼 +draft-send");
    assert!(
        draft_send.contains("--draft-id mock-draft-001"),
        "{draft_send}"
    );
    assert!(
        draft_send.contains("--as user"),
        "发信必须用用户身份：{draft_send}"
    );

    // 落盘目录分开，起草不混进直发的目录
    let files = fake_lark_cli::landed(&fake.store());
    assert!(
        files.contains(&"raw/mail/draft-create/0001.json".to_string()),
        "{files:?}"
    );
    assert!(
        files.contains(&"raw/mail/send/0001.json".to_string()),
        "{files:?}"
    );
    assert!(
        files.contains(&"raw/mail/draft-send/0001.json".to_string()),
        "{files:?}"
    );
}

#[test]
fn 正文文件拼成_body_file_标志() {
    let fake = fake_lark_cli::Fake::new("res_body_file");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("to".to_string(), json!("someone@example.com"));
    params.insert("body_file".to_string(), json!("letter.txt"));
    backend.call("mail_draft_create", params).unwrap();
    assert!(
        fake.argv_log()[0].contains("--body-file letter.txt"),
        "{}",
        fake.argv_log()[0]
    );
}

#[test]
fn 命令行里不出现固定邮箱_缺省取环境() {
    let fake = fake_lark_cli::Fake::new("res_mailbox");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("to".to_string(), json!("someone@example.com"));
    params.insert("subject".to_string(), json!("主题"));
    params.insert("body".to_string(), json!("正文"));
    backend.call("mail_send", params).unwrap();
    let argv = &fake.argv_log()[0];
    assert!(
        argv.contains("--mailbox "),
        "mailbox 缺省该由环境兜底：{argv}"
    );
    assert!(!argv.contains("hr@example.com"), "不该有写死的邮箱：{argv}");
}

#[test]
fn 按登记拼出命令行() {
    let fake = fake_lark_cli::Fake::new("res_argv");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("folder".to_string(), json!("收件箱"));
    params.insert("mailbox".to_string(), json!("hr@example.com"));
    params.insert("page_size".to_string(), json!(2));
    backend.call("mail_list", params).unwrap();

    let argv = &fake.argv_log()[0];
    for expected in [
        "mail +triage",
        "--mailbox hr@example.com",
        "--max 2",
        "--filter {\"folder\":\"收件箱\"}",
        "--format json",
    ] {
        assert!(argv.contains(expected), "命令行里该有 {expected}：{argv}");
    }
}

#[test]
fn 按登记把原样响应落盘() {
    let fake = fake_lark_cli::Fake::new("res_landed");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("folder".to_string(), json!("收件箱"));
    let payload = backend.call("mail_list", params).unwrap();
    assert_eq!(payload.value()["count"], 2);

    assert_eq!(
        fake_lark_cli::landed(&fake.store()),
        vec!["raw/mail/list/0001.json"]
    );
    let raw: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fake.store().join("raw/mail/list/0001.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(raw["ok"], true, "落盘的是未解信封的原样响应");
}

#[test]
fn 取群链接按登记拼出命令行() {
    let fake = fake_lark_cli::Fake::new("res_chat_link");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("chat_id".to_string(), json!("oc_调用方给的"));
    let payload = backend.call("im_chat_link", params).unwrap();
    assert!(
        payload.value()["share_link"]
            .as_str()
            .unwrap()
            .contains("link_token=")
    );

    let argv = &fake.argv_log()[0];
    assert!(argv.contains("im chats link"), "{argv}");
    assert!(argv.contains("--chat-id oc_调用方给的"), "{argv}");
}

#[test]
fn 取群链接可以要求长期有效() {
    let fake = fake_lark_cli::Fake::new("res_chat_link_permanent");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("chat_id".to_string(), json!("oc_x"));
    params.insert("validity_period".to_string(), json!("permanently"));
    let payload = backend.call("im_chat_link", params).unwrap();
    assert_eq!(
        payload.value()["is_permanent"],
        true,
        "夹具会跟着请求体回话"
    );

    let argv = &fake.argv_log()[0];
    assert!(
        argv.contains("--data {\"validity_period\":\"permanently\"}"),
        "{argv}"
    );
}

#[test]
fn 任务动作的三种取法各换命令段与落盘目录() {
    let fake = fake_lark_cli::Fake::new("res_task");
    let backend = fake.backend();
    for scope in ["mine", "related", "tasklists"] {
        let mut params = Map::new();
        params.insert("scope".to_string(), json!(scope));
        backend.call("task_list", params).unwrap();
        let expected = format!("raw/task/{scope}/0001.json");
        assert!(
            fake_lark_cli::landed(&fake.store()).contains(&expected),
            "scope={scope} 该落到 {expected}"
        );
    }
    let log = fake.argv_log();
    assert!(log[0].contains("task +get-my-tasks"), "{}", log[0]);
    assert!(log[1].contains("task +get-related-tasks"), "{}", log[1]);
    assert!(
        log[2].contains("task tasklists list --as user"),
        "{}",
        log[2]
    );
}
