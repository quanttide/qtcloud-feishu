//! 测试用的小工具：给每个用例一块自己的临时目录（跑完不清理也不互相踩）。

use std::path::PathBuf;

pub fn tmp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("qtcloud-feishu-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("建临时目录");
    dir
}
