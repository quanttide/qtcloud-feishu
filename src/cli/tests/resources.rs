//! 基础单元：十三个动作的登记，以及参数怎么变成命令行。

mod fake_lark_cli;

use qtcloud_feishu_cli::resources;
use serde_json::{Map, json};

#[test]
fn 动作名不重复() {
    let actions: Vec<&resources::Action> = resources::all().collect();
    assert_eq!(actions.len(), 13, "基础单元是十三个动作");
    let mut names: Vec<&str> = actions.iter().map(|a| a.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 13);
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
