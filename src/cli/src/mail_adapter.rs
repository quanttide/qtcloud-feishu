//! 能力层的契约：流程对外的需求清单。与实验室 Python 的 `Capabilities` Protocol 同名同义。
//!
//! YAML 里只出现能力名，不出现任何命令——实现可替换（mock / 将来的 lark-cli 子进程），
//! 执行器代码不动。

/// 一封邮件（流程用到的字段）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mail {
    pub sender: String,
    pub date: String,
}

/// 流程要用到的三种能力。参数与返回值都用普通数据，不给调用方任何平台概念。
pub trait Capabilities {
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
