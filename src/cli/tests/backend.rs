//! 后端：报错口径与位置参数兜底。

mod support;

use serde_json::{Map, json};

#[test]
fn 不认识的参数明确报错() {
    let fake = support::Fake::new("backend_unknown");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("folderx".to_string(), json!("收件箱"));
    let err = backend.call("mail_list", params).unwrap_err();
    assert!(err.contains("不认识这些参数"), "{err}");
}

#[test]
fn 缺必填明确报错() {
    let fake = support::Fake::new("backend_needs");
    let backend = fake.backend();
    let err = backend.call("im_chat_messages", Map::new()).unwrap_err();
    assert!(err.contains("需要 chat_id 或 user_id"), "{err}");
}

#[test]
fn 位置参数没给就用环境缺省() {
    let fake = support::Fake::new("backend_location");
    let backend = fake.backend();
    // im_chat_members 要 chat-id：调用方没给，后端用 LARK_CHAT_ID 的缺省值兜住
    let page = backend.call("im_chat_members", Map::new()).unwrap();
    assert_eq!(page.value()["users"].as_array().unwrap().len(), 1);
    assert!(
        fake.argv_log()[0].contains("--chat-id "),
        "{}",
        fake.argv_log()[0]
    );
}

#[test]
fn 命令段的取值不进代码_全从调用方来() {
    let fake = support::Fake::new("backend_values");
    let backend = fake.backend();
    let mut params = Map::new();
    params.insert("chat_id".to_string(), json!("oc_真群"));
    backend.call("im_chat_members", params).unwrap();
    assert!(
        fake.argv_log()[0].contains("oc_真群"),
        "{}",
        fake.argv_log()[0]
    );
}
