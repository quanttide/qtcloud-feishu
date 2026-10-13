#![allow(dead_code)] // 每个测试文件只用到其中一部分，别为没用到的那些报错

//! 测试设施：临时目录 + 假 lark-cli + 夹具。
//!
//! 假 CLI 与夹具都是测试自己写到临时目录的，仓库里不留第二份拷贝；argv 由假 CLI 追加到日志，
//! 断言"拼出来的命令行"就靠它——不必为了测试在实现里开洞。

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use qtcloud_feishu_cli::backend::Backend;
use qtcloud_feishu_cli::engine::{self, Definitions};

/// 每个用例一块自己的临时目录，跑完不清理也不互相踩。
pub fn tmp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("qtcloud-feishu-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("建临时目录");
    dir
}

/// 假 lark-cli：把 argv 记进日志，再按命令段吐夹具；不认识的命令按真 CLI 的规矩 exit 2。
pub struct Fake {
    pub dir: PathBuf,
    pub fixtures: PathBuf,
    log: PathBuf,
    script: PathBuf,
}

impl Fake {
    pub fn new(name: &str) -> Self {
        let dir = tmp_dir(name);
        let fixtures = dir.join("fixtures");
        let log = dir.join("argv.log");
        fs::create_dir_all(&fixtures).unwrap();
        write_fixtures(&fixtures);
        let script = dir.join("fake-lark-cli");
        let body = format!(
            r#"#!/bin/sh
echo "$@" >> "{log}"
case "$*" in
  *"fail-send"*) : ;;
  *"+triage"*) cat "{f}/triage.json" ;;
  *"+messages"*) cat "{f}/messages.json" ;;
  *"chat-members-list"*) cat "{f}/members.json" ;;
  *"+send"*) if [ -f "{f}/fail-send" ]; then echo '{{"ok":false,"error":{{"message":"发信失败"}}}}' >&2; exit 2; fi; cat "{f}/send.json" ;;
  *"+record-list"*) cat "{f}/records.json" ;;
  *"task"*) cat "{f}/tasks.json" ;;
  *) echo '{{"ok":false,"error":{{"message":"fake 没有这条命令"}}}}' >&2; exit 2 ;;
esac
"#,
            log = log.display(),
            f = fixtures.display()
        );
        fs::write(&script, body).unwrap();
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();
        Fake {
            dir,
            fixtures,
            log,
            script,
        }
    }

    /// 让发信这条命令失败（验"动作失败就不挪格"）。
    pub fn fail_send(&self) {
        fs::write(self.fixtures.join("fail-send"), b"1").unwrap();
    }

    pub fn store(&self) -> PathBuf {
        self.dir.join("store")
    }

    pub fn backend(&self) -> Backend {
        Backend::new(Some(self.script.display().to_string()), Some(self.store()))
    }

    /// 假 CLI 收到过的命令行（一行一条，参数以空格相连）。
    pub fn argv_log(&self) -> Vec<String> {
        fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// 改夹具：投递邮件换成给定的人与日期。
    pub fn set_mails(&self, mails: &[(&str, &str)]) {
        let triage: Vec<String> = mails
            .iter()
            .enumerate()
            .map(|(i, (who, date))| {
                format!(
                    r#"{{"message_id":"m-{i:03}","mailbox_id":"hr@example.com","date":"{date}","from":"{who}","subject":"应聘"}}"#
                )
            })
            .collect();
        let messages: Vec<String> = mails
            .iter()
            .enumerate()
            .map(|(i, (who, date))| {
                format!(
                    r#"{{"message_id":"m-{i:03}","subject":"应聘","head_from":{{"mail_address":"{who}"}},"to":[{{"mail_address":"hr@example.com"}}],"date_formatted":"{date}","body_plain_text":"简历"}}"#
                )
            })
            .collect();
        fs::write(
            self.fixtures.join("triage.json"),
            format!(
                r#"{{"ok":true,"data":{{"messages":[{}],"count":{},"has_more":false,"page_token":""}}}}"#,
                triage.join(","),
                triage.len()
            ),
        )
        .unwrap();
        fs::write(
            self.fixtures.join("messages.json"),
            format!(
                r#"{{"ok":true,"data":{{"messages":[{}],"total":{},"unavailable_message_ids":[]}}}}"#,
                messages.join(","),
                messages.len()
            ),
        )
        .unwrap();
    }

    /// 改夹具：群成员换成本组人。
    pub fn set_members(&self, members: &[&str]) {
        let users: Vec<String> = members
            .iter()
            .map(|who| format!(r#"{{"member_id":"{who}","member_id_type":"email","name":"x"}}"#))
            .collect();
        fs::write(
            self.fixtures.join("members.json"),
            format!(
                r#"{{"ok":true,"data":{{"users":[{}],"bots":[],"truncations":[]}}}}"#,
                users.join(",")
            ),
        )
        .unwrap();
    }
}

fn write_fixtures(dir: &std::path::Path) {
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
        dir.join("tasks.json"),
        r#"{"ok":true,"data":{"items":[{"guid":"tk-001","summary":"核对简历","completed":false}],"has_more":false,"page_token":""}}"#,
    )
    .unwrap();
    fs::write(
        dir.join("records.json"),
        r#"{"ok":true,"data":{"items":[{"record_id":"rec-0001","fields":{"email":"chenmo@example.com","submitted_at":"2026-10-04"}}],"total":1,"offset":0,"limit":100}}"#,
    )
    .unwrap();
}

/// 定义：我们的话 → 一串飞书活动（测试用的一套）。
pub const DEFINITIONS: &str = r#"定义:
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

/// 流程：状态、迁移、检查、动作（写的是定义的名字）。
pub const FLOW: &str = r#"流程: 招聘
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
  - 从: 已进群
    到: 建清单
    触发: 人工建清单
    执行者: 人
"#;

pub fn definitions() -> Definitions {
    engine::parse(DEFINITIONS).expect("定义读得通")
}

/// 落盘目录下所有文件的相对路径（排序）。
pub fn landed(store: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
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
                out.push(rel.display().to_string());
            }
        }
    }
    out.sort();
    out
}
