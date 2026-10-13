//! 基础单元：十三个动作的登记，以及"参数怎么变成命令行"。

mod support;

use qtcloud_feishu_cli::resources;
use serde_json::{Map, json};

#[test]
fn 十三个动作_名字唯一() {
    let actions: Vec<&resources::Action> = resources::all().collect();
    assert_eq!(actions.len(), 13, "基础单元是十三个动作");
    let mut names: Vec<&str> = actions.iter().map(|a| a.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 13, "动作名不许重");
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
fn 参数按登记拼成命令行_并落一份原样响应() {
    let fake = support::Fake::new("res_argv");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("folder".to_string(), json!("收件箱"));
    params.insert("mailbox".to_string(), json!("hr@example.com"));
    params.insert("page_size".to_string(), json!(2));

    let payload = backend.call("mail_list", params).unwrap();
    assert_eq!(payload.value()["count"], 2);

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

    let files = support::landed(&fake.store());
    assert_eq!(files, vec!["raw/mail/list/0001.json"]);
    let raw: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fake.store().join("raw/mail/list/0001.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(raw["ok"], true, "落盘的是未解信封的原样响应");
}

#[test]
fn task_三种取法各换命令段与落盘目录() {
    let fake = support::Fake::new("res_task");
    let backend = fake.backend();
    for (scope, landed) in [
        ("mine", "mine"),
        ("related", "related"),
        ("tasklists", "tasklists"),
    ] {
        let mut params = Map::new();
        params.insert("scope".to_string(), json!(scope));
        backend.call("task_list", params).unwrap();
        assert!(
            support::landed(&fake.store()).contains(&format!("raw/task/{landed}/0001.json")),
            "scope={scope} 该落到 raw/task/{landed}/"
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
