//! Mail 适配器：流程与邮件能力之间的适配契约。
//!
//! 适配器的分工——契约说流程词汇（文件夹、候选人邮箱），平台词汇（folder id、
//! 命令行参数）只活在实现里；YAML 里只出现能力名，不出现任何命令。
//! 换实现（mock / 将来的 lark-cli 子进程）不动执行器。方法与实验室 Python
//! 能力层的同名能力一一对应，跨语言对照靠它。

/// 一封邮件（流程用到的字段）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mail {
    pub sender: String,
    pub date: String,
}

/// 流程要用到的三种能力。参数与返回值都用普通数据，不给调用方任何平台概念。
pub trait MailAdapter {
    /// 读某个文件夹的邮件。读哪个文件夹由调用方决定，能力层不做取舍；
    /// 数据里没构造过的文件夹读出来是空——是「没数据」，不是「不该读」。
    fn read_mail(&self, folder: &str) -> Vec<Mail>;

    /// 读本实例所指群的成员，返回邮箱。
    fn read_group_members(&self) -> Vec<String>;

    /// 发一封邮件，返回 `message_id`；失败返回错误消息，执行器据此不挪格。
    fn send_mail(
        &self,
        to: &str,
        subject: &str,
        body: &str,
        attachments: &[String],
    ) -> Result<String, String>;
}
