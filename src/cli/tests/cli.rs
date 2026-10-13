//! 端到端：跑真的二进制，飞书那一侧用假 lark-cli（按命令段吐预制负载）。
//!
//! 夹具与假 CLI 都是这个测试自己写出来的，落在临时目录里——不往仓库里塞第二份拷贝。

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_qtcloud-feishu");

fn work_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qtcloud-feishu-it-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
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

/// 夹具：两封投递（收件箱）+ 一位已进群。
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

struct Env {
    dir: PathBuf,
    lark: PathBuf,
    definitions: PathBuf,
    flow: PathBuf,
    state: PathBuf,
}

fn setup(name: &str) -> Env {
    let dir = work_dir(name);
    let fixtures_dir = dir.join("fixtures");
    fixtures(&fixtures_dir);
    let lark = fake_lark_cli(&dir, &fixtures_dir);
    let definitions = dir.join("定义.yaml");
    let flow = dir.join("流程.yaml");
    fs::write(&definitions, DEFINITIONS).unwrap();
    fs::write(&flow, FLOW).unwrap();
    Env {
        state: dir.join("state.json"),
        definitions,
        flow,
        lark,
        dir,
    }
}

impl Env {
    fn run(&self, args: &[&str]) -> (String, String, bool) {
        let output = Command::new(BIN)
            .args(["--lark-cli"])
            .arg(&self.lark)
            .args(args)
            .output()
            .expect("跑得起来");
        (
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
            output.status.success(),
        )
    }
}

#[test]
fn 列基础单元的动作() {
    let env = setup("units");
    let (stdout, _, ok) = env.run(&["units"]);
    assert!(ok);
    assert!(stdout.contains("mail_list"), "{stdout}");
    for action in [
        "mail_list",
        "mail_messages",
        "mail_send",
        "im_chat_members",
        "base_records",
    ] {
        assert!(stdout.contains(action), "{action} 该在清单里");
    }
    assert!(
        stdout.contains("--format json") || stdout.contains("参数"),
        "{stdout}"
    );
}

#[test]
fn 跑一个基础单元动作_取值落盘() {
    let env = setup("unit");
    let store = env.dir.join("store");
    let (stdout, stderr, ok) = env.run(&[
        "--store",
        store.to_str().unwrap(),
        "unit",
        "mail_list",
        "-p",
        "folder=收件箱",
        "-p",
        "mailbox=hr@example.com",
    ]);
    assert!(ok, "{stdout}{stderr}");
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(payload["count"], 2);
    let landed = store.join("raw/mail/list/0001.json");
    let raw: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&landed).unwrap()).unwrap();
    assert_eq!(raw["ok"], true, "落盘的是未解信封的原样响应");
    assert!(raw["data"]["messages"].is_array());
}

#[test]
fn 拉到底并推游标() {
    let env = setup("page_all");
    let store = env.dir.join("store");
    let (stdout, stderr, ok) = env.run(&[
        "--store",
        store.to_str().unwrap(),
        "unit",
        "mail_list",
        "-p",
        "folder=收件箱",
        "--page-all",
        "--cursor",
        "mail",
    ]);
    assert!(ok, "{stdout}{stderr}");
    let pages: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(pages.as_array().unwrap().len(), 1, "这一页 has_more=false");
    assert!(store.join("cursor/mail.json").exists(), "拉到底才写游标");
}

#[test]
fn 跑一条定义_两步串起来_字段起成我们的话() {
    let env = setup("define");
    let store = env.dir.join("store");
    let (stdout, stderr, ok) = env.run(&[
        "--store",
        store.to_str().unwrap(),
        "define",
        "读投递邮件",
        "--definitions",
        env.definitions.to_str().unwrap(),
        "-p",
        "邮箱=hr@example.com",
        "-p",
        "文件夹=收件箱",
    ]);
    assert!(ok, "{stdout}{stderr}");
    let out: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = out["行"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["发件人"], "chenmo@example.com");
    assert_eq!(rows[0]["日期"], "2026-10-02");
    assert_eq!(rows[0]["主题"], "应聘数据工程师");
    assert_eq!(out["游标"], "2026-10-03", "增量游标取最新的那个日期");
    assert!(store.join("raw/mail/list/0001.json").exists());
    assert!(
        store.join("raw/mail/messages/0001.json").exists(),
        "两步各落一份"
    );
}

#[test]
fn 增量_带游标再拉就只剩新的() {
    let env = setup("cursor");
    let (stdout, stderr, ok) = env.run(&[
        "define",
        "读投递邮件",
        "--definitions",
        env.definitions.to_str().unwrap(),
        "-p",
        "邮箱=hr@example.com",
        "-p",
        "文件夹=收件箱",
        "-p",
        "游标=2026-10-03",
    ]);
    assert!(ok, "{stdout}{stderr}");
    let out: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(out.get("行").is_none(), "没有比游标更新的：{out}");
}

#[test]
fn 查调研记录_中文列名与英文键都认() {
    let env = setup("survey");
    let (stdout, stderr, ok) = env.run(&[
        "define",
        "查调研记录",
        "--definitions",
        env.definitions.to_str().unwrap(),
        "-p",
        "邮箱列表=[\"chenmo@example.com\"]",
    ]);
    assert!(ok, "{stdout}{stderr}");
    let out: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out["行"][0]["邮箱"], "chenmo@example.com");
    assert_eq!(out["行"][0]["提交时间"], "2026-10-04");
}

#[test]
fn 流程_从原始数据重建再推一轮() {
    let env = setup("flow");
    let args = |more: &[&str]| -> Vec<String> {
        let mut args: Vec<String> = vec![
            "flow".into(),
            "run".into(),
            "--flow".into(),
            env.flow.display().to_string(),
            "--definitions".into(),
            env.definitions.display().to_string(),
            "--state".into(),
            env.state.display().to_string(),
            "--today".into(),
            "2026-10-13".into(),
        ];
        args.extend(more.iter().map(|s| (*s).to_string()));
        args
    };
    let owned = args(&[]);
    let refs: Vec<&str> = owned.iter().map(String::as_str).collect();
    let (stdout, stderr, ok) = env.run(&refs);
    assert!(ok, "{stdout}{stderr}");
    assert!(stdout.contains("流程：招聘"));
    assert!(stdout.contains("从飞书侧原始数据重建（2 人）"), "{stdout}");
    // 投递那位推得动（有动作），已进群那位没有可推的迁移
    assert!(
        stdout.contains("✓ chenmo@example.com") && stdout.contains("投递 → 二维码已发"),
        "{stdout}"
    );
    assert!(stdout.contains("记发送时间：mock-chenmo"), "{stdout}");
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&env.state).unwrap()).unwrap();
    assert_eq!(state["chenmo@example.com"]["state"], "二维码已发");
    assert_eq!(state["herui@example.com"]["state"], "已进群");
    assert_eq!(state["chenmo@example.com"]["since"], "2026-10-13");
}

#[test]
fn 流程_缺了配置就明确报错() {
    let env = setup("flow_error");
    let (stdout, stderr, ok) = env.run(&[
        "flow",
        "status",
        "--flow",
        env.flow.to_str().unwrap(),
        "--definitions",
        env.dir.join("没有这个文件.yaml").to_str().unwrap(),
        "--state",
        env.state.to_str().unwrap(),
    ]);
    assert!(!ok);
    assert!(stderr.contains("读不到定义文件"), "{stdout}{stderr}");
}

#[test]
fn 参数不成对时说清楚怎么写() {
    let env = setup("bad_param");
    let (_, stderr, ok) = env.run(&[
        "define",
        "读投递邮件",
        "--definitions",
        env.definitions.to_str().unwrap(),
        "-p",
        "邮箱",
    ]);
    assert!(!ok);
    assert!(stderr.contains("名=值"), "{stderr}");
}
