//! 端到端：真跑进程，飞书那一侧用假 lark-cli（按命令段吐预制负载）。
//!
//! 假 CLI 与夹具都是这个测试自己写出来的，落在临时目录里——不往仓库里塞第二份拷贝；
//! 命令面还没定，所以这里直接调机制（后端 → 串联器 → 执行器），不经过命令行。

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::backend::Backend;
use crate::engine::{Engine, parse as parse_definitions};
use crate::flow::{self, advance, derive};

fn work_dir(name: &str) -> PathBuf {
    crate::test_support::tmp_dir(name)
}

/// 假 lark-cli：按命令段把夹具吐出来；不认识的命令按真 CLI 的规矩 exit 2。
fn fake_lark_cli(dir: &Path, fixtures: &Path) -> PathBuf {
    let script = dir.join("fake-lark-cli");
    let body = format!(
        r#"#!/bin/sh
case "$*" in
  *"+triage"*) cat "{f}/triage.json" ;;
  *"+messages"*) cat "{f}/messages.json" ;;
  *"chat-members-list"*) cat "{f}/members.json" ;;
  *"+send"*) cat "{f}/send.json" ;;
  *"+record-list"*) cat "{f}/records.json" ;;
  *) echo '{{"ok":false,"error":{{"message":"fake 没有这条命令"}}}}' >&2; exit 2 ;;
esac
"#,
        f = fixtures.display()
    );
    fs::write(&script, body).unwrap();
    let mut perms = fs::metadata(&script).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&script, perms).unwrap();
    script
}

fn fixtures(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("triage.json"),
        r#"{"ok":true,"data":{"messages":[
            {"message_id":"m-001","mailbox_id":"hr@example.com","date":"2026-10-02","from":"chenmo@example.com","subject":"应聘数据工程师"},
            {"message_id":"m-002","mailbox_id":"hr@example.com","date":"2026-10-03","from":"herui@example.com","subject":"应聘前端工程师"}
        ],"count":2,"has_more":false,"page_token":""}}"#,
    )
    .unwrap();
    fs::write(
        dir.join("messages.json"),
        r#"{"ok":true,"data":{"messages":[
            {"message_id":"m-001","subject":"应聘数据工程师","head_from":{"mail_address":"chenmo@example.com"},"to":[{"mail_address":"hr@example.com"}],"date_formatted":"2026-10-02","body_plain_text":"简历见附件","attachments":[{"id":"m-001-att-0","filename":"陈默-简历.txt"}]},
            {"message_id":"m-002","subject":"应聘前端工程师","head_from":{"mail_address":"herui@example.com"},"to":[{"mail_address":"hr@example.com"}],"date_formatted":"2026-10-03","body_plain_text":"作品集见附件","attachments":[]}
        ],"total":2,"unavailable_message_ids":[]}}"#,
    )
    .unwrap();
    fs::write(
        dir.join("members.json"),
        r#"{"ok":true,"data":{"users":[{"member_id":"herui@example.com","member_id_type":"email","name":"herui"}],"bots":[],"truncations":[]}}"#,
    )
    .unwrap();
    fs::write(
        dir.join("send.json"),
        r#"{"ok":true,"data":{"message_id":"mock-chenmo","thread_id":"t-1"}}"#,
    )
    .unwrap();
    fs::write(
        dir.join("records.json"),
        r#"{"ok":true,"data":{"items":[{"record_id":"rec-0001","fields":{"email":"chenmo@example.com","submitted_at":"2026-10-04"}}],"total":1,"offset":0,"limit":100}}"#,
    )
    .unwrap();
}

const DEFINITIONS: &str = r#"定义:
  - 名称: 读投递邮件
    说明: 一个文件夹的投递：先列摘要拿 id，再按 id 取正文
    步骤:
      - 动作: mail_list
        参数: {mailbox: "{{邮箱}}", folder: "{{文件夹}}"}
        as: 摘要
      - 动作: mail_messages
        参数: {mailbox: "{{邮箱}}", message_ids: "{{摘要.messages[].message_id}}"}
        as: 正文
    出:
      行: 正文.messages
      字段: {message_id: message_id, 主题: subject, 发件人: head_from.mail_address, 日期: date_formatted}
    增量: {比: 日期}
  - 名称: 看谁进群了
    步骤:
      - 动作: im_chat_members
        参数: {chat_id: "{{群}}"}
        as: 成员
    出:
      行: 成员.users
      字段: {邮箱: member_id}
  - 名称: 查调研记录
    步骤:
      - 动作: base_records
        参数:
          filter_json: {logic: and, conditions: [[邮箱, intersects, "{{邮箱列表}}"]]}
        as: 记录
    出:
      行: 记录.items
      字段: {邮箱: "fields.email|fields.邮箱", 提交时间: "fields.submitted_at|fields.提交时间"}
  - 名称: 发群二维码
    步骤:
      - 动作: mail_send
        参数: {mailbox: "{{邮箱}}", to: "{{候选人.邮箱}}", subject: 量潮实训基地 · 群二维码, body: 扫码进群（见附件）}
    出: {字段: {邮件id: message_id}}
"#;

const FLOW: &str = r#"流程: 招聘
收集:
  邮箱: hr@example.com
  范围: [收件箱]
  进人: 读投递邮件
  群成员: 看谁进群了
状态: [投递, 二维码已发, 已进群]
迁移:
  - 从: 投递
    到: 二维码已发
    触发: 收到投递
    执行者: 系统
    动作:
      定义: 发群二维码
    副作用: 记发送时间
  - 从: 二维码已发
    到: 已进群
    触发: 群里出现该邮箱
    执行者: 系统
    检查:
      定义: 看谁进群了
      成立: 包含该候选人邮箱
"#;

struct Scene {
    lark: PathBuf,
    store: PathBuf,
}

fn scene(name: &str) -> Scene {
    let dir = work_dir(name);
    let fixtures_dir = dir.join("fixtures");
    fixtures(&fixtures_dir);
    let lark = fake_lark_cli(&dir, &fixtures_dir);
    Scene {
        lark,
        store: dir.join("store"),
    }
}

impl Scene {
    fn backend(&self) -> Backend {
        Backend::new(
            Some(self.lark.display().to_string()),
            Some(self.store.clone()),
        )
    }
}

fn landed(store: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack = vec![store.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(rel) = path.strip_prefix(store) {
                out.insert(rel.display().to_string());
            }
        }
    }
    out
}

#[test]
fn 定义_两步串起来_字段起成我们的话并落盘() {
    let scene = scene("e2e_define");
    let backend = scene.backend();
    let engine = Engine::new(&backend, parse_definitions(DEFINITIONS).unwrap());
    let mut context = Map::new();
    context.insert("邮箱".to_string(), json!("hr@example.com"));
    context.insert("文件夹".to_string(), json!("收件箱"));

    let outcome = engine.call("读投递邮件", &context).unwrap();
    assert_eq!(outcome.rows.len(), 2);
    assert_eq!(outcome.rows[0]["发件人"], "chenmo@example.com");
    assert_eq!(outcome.rows[0]["日期"], "2026-10-02");
    assert_eq!(
        outcome.cursor.as_deref(),
        Some("2026-10-03"),
        "增量游标取最新的日期"
    );

    // 增量：带上游标再跑，没有更新的
    let mut with_cursor = context.clone();
    with_cursor.insert("游标".to_string(), json!("2026-10-03"));
    let again = engine.call("读投递邮件", &with_cursor).unwrap();
    assert!(again.rows.is_empty(), "{:?}", again.rows);

    // 两步各落一份原样响应
    let files = landed(&scene.store);
    assert!(files.contains("raw/mail/list/0001.json"), "{files:?}");
    assert!(files.contains("raw/mail/messages/0001.json"), "{files:?}");
    let raw: Value = serde_json::from_str(
        &fs::read_to_string(scene.store.join("raw/mail/list/0001.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(raw["ok"], true, "落盘的是未解信封的原样响应");
}

#[test]
fn 定义_表列名两种写法都认() {
    let scene = scene("e2e_survey");
    let backend = scene.backend();
    let engine = Engine::new(&backend, parse_definitions(DEFINITIONS).unwrap());
    let mut context = Map::new();
    context.insert("邮箱列表".to_string(), json!(["chenmo@example.com"]));
    let outcome = engine.call("查调研记录", &context).unwrap();
    assert_eq!(outcome.rows[0]["邮箱"], "chenmo@example.com");
    assert_eq!(outcome.rows[0]["提交时间"], "2026-10-04");
}

#[test]
fn 流程_从原始数据重建再推一轮() {
    let scene = scene("e2e_flow");
    let backend = scene.backend();
    let engine = Engine::new(&backend, parse_definitions(DEFINITIONS).unwrap());
    let flow = flow::parse(FLOW).unwrap();
    let context = flow::context_of(&flow, None);

    let mut state = derive(&engine, &flow, &context, "2026-10-13").unwrap();
    assert_eq!(state.len(), 2);
    assert_eq!(state["herui@example.com"].state, "已进群");
    assert_eq!(state["chenmo@example.com"].state, "投递");

    let report = advance(&engine, &flow, &mut state, &context, "2026-10-13", false).unwrap();
    assert_eq!(report.advanced.len(), 1);
    assert_eq!(report.advanced[0].0, "chenmo@example.com");
    assert!(
        report.advanced[0].3.contains("记发送时间"),
        "副作用带上返回值"
    );
    assert_eq!(state["chenmo@example.com"].state, "二维码已发");
    assert!(landed(&scene.store).contains("raw/mail/send/0001.json"));
    // 已进群那位没有可推的活，且状态不再变
    assert_eq!(state["herui@example.com"].state, "已进群");
}

#[test]
fn 位置参数由环境变量给_命令行不写死() {
    let scene = scene("e2e_env");
    // 不传 --chat-id：由 LARK_CHAT_ID 兜底（这里用夹具里的默认群名）
    let backend = scene.backend();
    let engine = Engine::new(&backend, parse_definitions(DEFINITIONS).unwrap());
    let outcome = engine.call("看谁进群了", &Map::new()).unwrap();
    assert_eq!(outcome.rows.len(), 1);
    assert_eq!(outcome.rows[0]["邮箱"], "herui@example.com");
}

#[test]
fn 不认识的参数与缺必填都明确报错() {
    let scene = scene("e2e_errors");
    let backend = scene.backend();
    let mut params = Map::new();
    params.insert("folderx".to_string(), json!("收件箱"));
    let err = backend.call("mail_list", params).unwrap_err();
    assert!(err.contains("不认识这些参数"), "{err}");
    let err = backend.call("im_chat_messages", Map::new()).unwrap_err();
    assert!(err.contains("需要 chat_id 或 user_id"), "{err}");
}
