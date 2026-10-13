//! 串联器：选择器、模板、定义读法。

mod fake_lark_cli;

use qtcloud_feishu_cli::engine::{pick, resolve};
use serde_json::{Map, json};

#[test]
fn 选择器逐层取() {
    let ctx = json!({"候选人": {"邮箱": "chenmo@example.com"}});
    assert_eq!(
        pick(&ctx, "候选人.邮箱").unwrap(),
        json!("chenmo@example.com")
    );
    assert!(pick(&ctx, "候选人.没有这个").is_none());
}

#[test]
fn 选择器逐个取() {
    let ctx = json!({"摘要": {"messages": [{"message_id": "m-1"}, {"message_id": "m-2"}]}});
    assert_eq!(
        pick(&ctx, "摘要.messages[].message_id").unwrap(),
        json!(["m-1", "m-2"])
    );
}

#[test]
fn 选择器在两种列名下都取得到() {
    let ctx = json!({"行": {"fields": {"邮箱": "a@example.com"}}});
    assert_eq!(
        pick(&ctx, "行.fields.email|行.fields.邮箱").unwrap(),
        json!("a@example.com")
    );
}

#[test]
fn 整串模板保持原类型() {
    let mut ctx = Map::new();
    ctx.insert("文件夹".to_string(), json!("收件箱"));
    ctx.insert("邮箱列表".to_string(), json!(["a@example.com"]));
    assert_eq!(resolve(&json!("{{文件夹}}"), &ctx), json!("收件箱"));
    assert_eq!(
        resolve(&json!("{{邮箱列表}}"), &ctx),
        json!(["a@example.com"])
    );
}

#[test]
fn 嵌在文字里的模板拼成字() {
    let mut ctx = Map::new();
    ctx.insert("候选人".to_string(), json!({"邮箱": "a@example.com"}));
    ctx.insert("文件夹".to_string(), json!("收件箱"));
    assert_eq!(
        resolve(&json!("{{候选人.邮箱}}的投递"), &ctx),
        json!("a@example.com的投递")
    );
    assert_eq!(
        resolve(&json!({"folder": "{{文件夹}}"}), &ctx),
        json!({"folder": "收件箱"})
    );
}

#[test]
fn 定义读得通() {
    let defs = fake_lark_cli::definitions();
    assert!(defs.contains_key("读投递邮件"));
    assert_eq!(defs.len(), 4, "这套测试用四条定义");
}

#[test]
fn 步骤与出表都读进来() {
    let defs = fake_lark_cli::definitions();
    let read = &defs["读投递邮件"];
    assert_eq!(read.steps.len(), 2, "两步：列摘要 → 取正文");
    assert_eq!(read.steps[0].action, "mail_list");
    assert_eq!(read.steps[1].as_name, "正文");
    assert_eq!(read.rows_from, "正文.messages");
    assert_eq!(read.fields[0].0, "message_id");
    assert_eq!(read.increment_by, "日期");
}

#[test]
fn 读不到定义文件就报错() {
    let err = qtcloud_feishu_cli::engine::load(&std::path::PathBuf::from("没有这个文件.yaml"))
        .unwrap_err();
    assert!(err.contains("读不到定义文件"), "{err}");
}
