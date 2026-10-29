//! 后端：报错口径与位置参数的来路。

mod fake_lark_cli;

use std::time::Duration;

use serde_json::{Map, json};

#[test]
fn 不认识的参数明确报错() {
    let fake = fake_lark_cli::Fake::new("backend_unknown");
    let mut params = Map::new();
    params.insert("folderx".to_string(), json!("收件箱"));
    let err = fake.backend().call("mail_list", params).unwrap_err();
    assert!(err.contains("不认识这些参数"), "{err}");
}

#[test]
fn 缺必填明确报错() {
    let fake = fake_lark_cli::Fake::new("backend_needs");
    let err = fake
        .backend()
        .call("im_chat_messages", Map::new())
        .unwrap_err();
    assert!(err.contains("需要 chat_id 或 user_id"), "{err}");
}

#[test]
fn 位置参数没给就用环境缺省() {
    let fake = fake_lark_cli::Fake::new("backend_location");
    // im_chat_members 要 chat-id：调用方没给，后端拿环境缺省兜住（假 CLI 缺这个 flag 会报错）
    let page = fake.backend().call("im_chat_members", Map::new()).unwrap();
    assert_eq!(page.value()["users"].as_array().unwrap().len(), 1);
    assert!(
        fake.argv_log()[0].contains("--chat-id "),
        "{}",
        fake.argv_log()[0]
    );
}

#[test]
fn 写动作超时按配置生效() {
    let fake = fake_lark_cli::Fake::new("backend_timeout");
    fake.slow_send();
    let backend = fake.backend().with_timeout(Duration::from_millis(300));
    let mut params = Map::new();
    params.insert("to".to_string(), json!("someone@example.com"));
    params.insert("subject".to_string(), json!("主题"));
    params.insert("body".to_string(), json!("正文"));
    let err = backend.call("mail_send", params).unwrap_err();
    assert!(err.contains("超时"), "{err}");
}

#[test]
fn dry_run只拼命令行不跑也不落盘() {
    let fake = fake_lark_cli::Fake::new("backend_plan");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("to".to_string(), json!("someone@example.com"));
    params.insert("subject".to_string(), json!("主题"));
    params.insert("body".to_string(), json!("正文"));
    let argv = backend.plan("mail_send", params).unwrap();
    assert!(
        argv.join(" ").contains("mail +send --confirm-send"),
        "{argv:?}"
    );
    assert!(fake.argv_log().is_empty(), "dry-run 不该跑子进程");
    assert!(
        fake_lark_cli::landed(&fake.store()).is_empty(),
        "dry-run 不该落盘"
    );
}

#[test]
fn dry_run的参数校验与真实调用同一份() {
    let fake = fake_lark_cli::Fake::new("backend_plan_check");
    let mut params = Map::new();
    params.insert("folderx".to_string(), json!("收件箱"));
    let err = fake.backend().plan("mail_list", params).unwrap_err();
    assert!(err.contains("不认识这些参数"), "{err}");
}

#[test]
fn 写动作成功后操作日志有一条且不含正文() {
    let fake = fake_lark_cli::Fake::new("backend_ops_ok");
    let ops = fake.dir.join("ops.jsonl");
    let backend = fake.backend().with_ops_log(ops.clone());
    let mut params = Map::new();
    params.insert("to".to_string(), json!("someone@example.com"));
    params.insert("subject".to_string(), json!("面试邀请"));
    params.insert("body".to_string(), json!("不可告人的正文"));
    backend.call("mail_send", params).unwrap();

    let line = std::fs::read_to_string(&ops).expect("该有一条日志");
    assert!(line.contains("\"action\":\"mail_send\""), "{line}");
    assert!(line.contains("\"resource\":\"mail\""), "{line}");
    assert!(line.contains("面试邀请"), "主题保留：{line}");
    assert!(line.contains("mock-chenmo"), "返回里的 id 要记：{line}");
    assert!(!line.contains("不可告人的正文"), "正文不进日志：{line}");
}

#[test]
fn 读动作不写操作日志() {
    let fake = fake_lark_cli::Fake::new("backend_ops_read");
    let ops = fake.dir.join("ops.jsonl");
    let backend = fake.backend().with_ops_log(ops.clone());
    let mut params = Map::new();
    params.insert("folder".to_string(), json!("收件箱"));
    backend.call("mail_list", params).unwrap();
    assert!(!ops.exists(), "读动作不该写日志");
}

#[test]
fn 写日志失败要报错并给出待补记的那条() {
    let fake = fake_lark_cli::Fake::new("backend_ops_fail");
    // 把日志路径指到一个目录上——往目录里写必然失败
    let ops_dir = fake.dir.join("ops-as-dir");
    std::fs::create_dir_all(&ops_dir).unwrap();
    let backend = fake.backend().with_ops_log(ops_dir);
    let mut params = Map::new();
    params.insert("to".to_string(), json!("someone@example.com"));
    params.insert("subject".to_string(), json!("主题"));
    params.insert("body".to_string(), json!("正文"));
    let err = backend.call("mail_send", params).unwrap_err();
    assert!(err.contains("已执行并落盘"), "要把已发生说清：{err}");
    assert!(err.contains("待补记"), "要给出待补记条目：{err}");
}

#[test]
fn 失败动作不写日志且错误带出平台消息() {
    let fake = fake_lark_cli::Fake::new("backend_ops_failed_action");
    fake.fail_send();
    let ops = fake.dir.join("ops.jsonl");
    let backend = fake.backend().with_ops_log(ops.clone());
    let mut params = Map::new();
    params.insert("to".to_string(), json!("someone@example.com"));
    params.insert("subject".to_string(), json!("主题"));
    params.insert("body".to_string(), json!("正文"));
    let err = backend.call("mail_send", params).unwrap_err();
    assert!(err.contains("发信失败"), "{err}");
    assert!(!ops.exists(), "失败不写日志");
}

#[test]
fn 取值全从调用方来() {
    let fake = fake_lark_cli::Fake::new("backend_values");
    let mut params = Map::new();
    params.insert("chat_id".to_string(), json!("oc_调用方给的"));
    fake.backend().call("im_chat_members", params).unwrap();
    assert!(
        fake.argv_log()[0].contains("oc_调用方给的"),
        "{}",
        fake.argv_log()[0]
    );
}
