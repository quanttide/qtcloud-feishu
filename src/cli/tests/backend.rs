//! 后端：报错口径与位置参数的来路。

mod fake_lark_cli;

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
