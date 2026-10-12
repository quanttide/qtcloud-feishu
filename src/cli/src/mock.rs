//! 能力层的 mock 实现：数据内嵌、不打任何外部接口。
//!
//! 与实验室 `capabilities/mock.py` 对应：同一套能力名（读邮件 / 读群成员 / 发邮件），
//! 换实现不换流程。`message_id` 的生成算法与 Python 版一致，两版产物可对照。

/// 一封邮件（流程只用到投递场景的字段）。
pub struct Mail {
    pub sender: String,
    pub date: String,
}

/// 收件箱里的投递（夹具同步拷贝）：邮箱 → 投递日。
const INBOX: &[(&str, &str)] = &[
    ("chenmo@example.com", "2026-10-02"),
    ("herui@example.com", "2026-10-04"),
    ("linyizhou@example.com", "2026-10-03"),
    ("suxiao@example.com", "2026-10-03"),
];

/// 群成员（夹具同步拷贝）：含员工，候选人只取交集。
const GROUP_MEMBERS: &[&str] = &[
    "herui@example.com",
    "yuanyu@example.com",
    "hyacinth@example.com",
];

/// 夹具实现：读内嵌数据，不打任何外部接口。
pub struct Mock;

impl Mock {
    /// 读某个文件夹的邮件。当前只带收件箱——derive 只认收件箱，与 Python 版一致。
    pub fn read_mail(&self, folder: &str) -> Vec<Mail> {
        if folder != "收件箱" {
            return Vec::new();
        }
        INBOX
            .iter()
            .map(|(who, date)| Mail {
                sender: (*who).to_string(),
                date: (*date).to_string(),
            })
            .collect()
    }

    /// 读群成员，返回邮箱集合。
    pub fn read_group_members(&self) -> Vec<String> {
        GROUP_MEMBERS.iter().map(|m| (*m).to_string()).collect()
    }

    /// 发一封邮件，返回 message_id（算法与 Python mock 同，产物可对照）。
    /// 真实现要带正文与附件（夹具层会写进 `outbox/`），mock 只回 message_id。
    pub fn send_mail(
        &self,
        to: &str,
        subject: &str,
        _body: &str,
        _attachments: &[String],
    ) -> String {
        let mut digest: u64 = 0;
        for ch in to.chars() {
            digest = (digest * 131 + u64::from(u32::from(ch))) % 0xFFFFFF;
        }
        let local = to.split('@').next().unwrap_or(to);
        let head: String = subject.chars().take(12).filter(|c| *c != ' ').collect();
        format!("mock-{local}_{digest:06x}-{head}")
    }
}
