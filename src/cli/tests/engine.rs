//! 串联器：选择器、模板、定义读法。

mod support;

use qtcloud_feishu_cli::engine::{self, pick, resolve};
use serde_json::{Map, json};

#[test]
fn 选择器_逐层取与逐个取() {
    let ctx = json!({
        "邮箱": "hr@example.com",
        "候选人": {"邮箱": "chenmo@example.com"},
        "摘要": {"messages": [{"message_id": "m-1"}, {"message_id": "m-2"}]},
    });
    assert_eq!(pick(&ctx, "邮箱").unwrap(), json!("hr@example.com"));
    assert_eq!(
        pick(&ctx, "候选人.邮箱").unwrap(),
        json!("chenmo@example.com")
    );
    assert_eq!(
        pick(&ctx, "摘要.messages[].message_id").unwrap(),
        json!(["m-1", "m-2"])
    );
    assert!(pick(&ctx, "摘要.nothing").is_none());
}

#[test]
fn 选择器_两种列名写法兜底() {
    let ctx = json!({"行": {"fields": {"邮箱": "a@example.com"}}});
    assert_eq!(
        pick(&ctx, "行.fields.email|行.fields.邮箱").unwrap(),
        json!("a@example.com")
    );
}

#[test]
fn 模板_整串保持原类型_嵌在文字里就拼字() {
    let mut ctx = Map::new();
    ctx.insert("文件夹".to_string(), json!("收件箱"));
    ctx.insert("候选人".to_string(), json!({"邮箱": "a@example.com"}));
    assert_eq!(resolve(&json!("{{文件夹}}"), &ctx), json!("收件箱"));
    assert_eq!(
        resolve(&json!("{{候选人.邮箱}}的投递"), &ctx),
        json!("a@example.com的投递")
    );
    assert_eq!(
        resolve(&json!(["{{文件夹}}", "x"]), &ctx),
        json!(["收件箱", "x"])
    );
    assert_eq!(
        resolve(&json!({"a": "{{文件夹}}"}), &ctx),
        json!({"a": "收件箱"})
    );
}

#[test]
fn 定义读得通_步骤与出表都在() {
    let defs = support::definitions();
    let read = defs.get("读投递邮件").unwrap();
    assert_eq!(read.steps.len(), 2, "两步：列摘要 → 取正文");
    assert_eq!(read.steps[0].action, "mail_list");
    assert_eq!(read.steps[1].as_name, "正文");
    assert_eq!(read.rows_from, "正文.messages");
    assert_eq!(
        read.fields[0],
        ("message_id".to_string(), "message_id".to_string())
    );
    assert_eq!(read.increment_by, "日期");
    assert!(engine::load(&std::path::PathBuf::from("没有这个文件.yaml")).is_err());
}
