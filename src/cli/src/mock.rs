//! 能力层的 mock 实现：数据构造时注入，零外部 IO。
//!
//! 与实验室 `capabilities/mock.py` 对应，两件事分开放：
//! 契约在 [`crate::capabilities`]，这里只管「拿一份数据当能力使」；
//! `Mock::default()` 是实验室那套夹具（跨语言对照的 `message_id` 算法也对齐），
//! 换场景用 [`Mock::with`] 注入另一份数据，不改本文件。

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use crate::capabilities::{Capabilities, Mail};

/// 一封已发出的邮件：mock 把它记下来，调用方与测试能断言「发了什么」。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentMail {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<String>,
    pub message_id: String,
}

/// 可注入数据的能力 mock。
///
/// 夹具不再是编译期常量：文件夹、群成员、读哪个群都由构造给，
/// 同一进程里可以有多套互不相干的数据，测试之间互不污染。
pub struct Mock {
    /// 文件夹名 → 邮件（发件人, 日期）
    pub folders: BTreeMap<String, Vec<(String, String)>>,
    /// 群 id → 成员邮箱
    pub groups: BTreeMap<String, Vec<String>>,
    /// 本实例读哪个群（与真实现把 chat_id 放构造参数的做法对齐，方法签名不带群概念）
    pub chat_id: String,
    /// 已发出的邮件（对应 Python 夹具层写进 `outbox/` 的那叠文件）
    pub sent: RefCell<Vec<SentMail>>,
    /// 置真时 `send_mail` 一律失败：用来测「动作先做，失败不挪格」
    pub fail_send: Cell<bool>,
}

impl Mock {
    /// 任意数据构造：换场景换一份数据，不改这个文件。
    pub fn with(
        folders: BTreeMap<String, Vec<(String, String)>>,
        groups: BTreeMap<String, Vec<String>>,
        chat_id: &str,
    ) -> Self {
        Self {
            folders,
            groups,
            chat_id: chat_id.to_string(),
            sent: RefCell::new(Vec::new()),
            fail_send: Cell::new(false),
        }
    }
}

impl Default for Mock {
    /// 实验室夹具：收件箱四封投递 + 实训基地群三个成员。
    fn default() -> Self {
        let folders = BTreeMap::from([(
            "收件箱".to_string(),
            vec![
                ("chenmo@example.com".to_string(), "2026-10-02".to_string()),
                (
                    "linyizhou@example.com".to_string(),
                    "2026-10-03".to_string(),
                ),
                ("suxiao@example.com".to_string(), "2026-10-03".to_string()),
                ("herui@example.com".to_string(), "2026-10-04".to_string()),
            ],
        )]);
        let groups = BTreeMap::from([(
            "oc_mock".to_string(),
            vec![
                "herui@example.com".to_string(),
                "yuanyu@example.com".to_string(),
                "hyacinth@example.com".to_string(),
            ],
        )]);
        Self::with(folders, groups, "oc_mock")
    }
}

impl Capabilities for Mock {
    fn read_mail(&self, folder: &str) -> Vec<Mail> {
        self.folders
            .get(folder)
            .map(|mails| {
                mails
                    .iter()
                    .map(|(sender, date)| Mail {
                        sender: sender.clone(),
                        date: date.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn read_group_members(&self) -> Vec<String> {
        self.groups.get(&self.chat_id).cloned().unwrap_or_default()
    }

    fn send_mail(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        attachments: &[String],
    ) -> Result<String, String> {
        if self.fail_send.get() {
            return Err(format!("mock 注入的失败：发给 {to} 的「{subject}」"));
        }
        let message_id = message_id(to, subject);
        self.sent.borrow_mut().push(SentMail {
            to: to.to_string(),
            subject: subject.to_string(),
            body: body.to_string(),
            attachments: attachments.to_vec(),
            message_id: message_id.clone(),
        });
        Ok(message_id)
    }
}

/// `message_id` 生成：算法与 Python mock 同步，跨语言对照靠它，动它等于断对照。
fn message_id(to: &str, subject: &str) -> String {
    let mut digest: u64 = 0;
    for ch in to.chars() {
        digest = (digest * 131 + u64::from(u32::from(ch))) % 0xFFFFFF;
    }
    let local = to.split('@').next().unwrap_or(to);
    let head: String = subject.chars().take(12).filter(|c| *c != ' ').collect();
    format!("mock-{local}_{digest:06x}-{head}")
}
