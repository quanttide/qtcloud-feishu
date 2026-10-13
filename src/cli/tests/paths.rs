//! 落盘位置：照 XDG 基目录规范的几条规矩。

use std::collections::BTreeMap;
use std::path::PathBuf;

use qtcloud_feishu_cli::paths::{APP, locate};

/// 造一个环境变量表（不真改进程环境，测试之间不互相踩）。
fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let table: BTreeMap<String, String> = pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    move |key: &str| table.get(key).cloned()
}

#[test]
fn 变量给了绝对路径就用它() {
    let got = locate(
        "XDG_DATA_HOME",
        &[".local", "share"],
        &env(&[
            ("XDG_DATA_HOME", "/srv/feishu-data"),
            ("HOME", "/home/someone"),
        ]),
    )
    .unwrap();
    assert_eq!(got, PathBuf::from("/srv/feishu-data").join(APP));
}

#[test]
fn 变量没设就用家目录缺省() {
    let got = locate(
        "XDG_DATA_HOME",
        &[".local", "share"],
        &env(&[("HOME", "/home/someone")]),
    )
    .unwrap();
    assert_eq!(got, PathBuf::from("/home/someone/.local/share").join(APP));
}

#[test]
fn 变量为空当没设() {
    let got = locate(
        "XDG_CACHE_HOME",
        &[".cache"],
        &env(&[("XDG_CACHE_HOME", ""), ("HOME", "/home/someone")]),
    )
    .unwrap();
    assert_eq!(got, PathBuf::from("/home/someone/.cache").join(APP));
}

#[test]
fn 变量是相对路径当没设() {
    let got = locate(
        "XDG_STATE_HOME",
        &[".local", "state"],
        &env(&[("XDG_STATE_HOME", "相对/路径"), ("HOME", "/home/someone")]),
    )
    .unwrap();
    assert_eq!(got, PathBuf::from("/home/someone/.local/state").join(APP));
}

#[test]
fn 家目录也没给就报错不猜() {
    let err = locate("XDG_CONFIG_HOME", &[".config"], &env(&[])).unwrap_err();
    assert!(err.contains("HOME"), "{err}");
}

#[test]
fn 四类目录各有各的位置() {
    let e = env(&[("HOME", "/home/someone")]);
    assert_eq!(
        locate("XDG_DATA_HOME", &[".local", "share"], &e).unwrap(),
        PathBuf::from("/home/someone/.local/share").join(APP)
    );
    assert_eq!(
        locate("XDG_STATE_HOME", &[".local", "state"], &e).unwrap(),
        PathBuf::from("/home/someone/.local/state").join(APP)
    );
    assert_eq!(
        locate("XDG_CONFIG_HOME", &[".config"], &e).unwrap(),
        PathBuf::from("/home/someone/.config").join(APP)
    );
    assert_eq!(
        locate("XDG_CACHE_HOME", &[".cache"], &e).unwrap(),
        PathBuf::from("/home/someone/.cache").join(APP)
    );
}

#[test]
fn 流程状态与配置文件各带自己的名字() {
    // 这两个函数读真环境；只断言形状（家目录下的相对层级与文件名），不写盘
    let state = qtcloud_feishu_cli::paths::state_path("招聘").unwrap();
    assert!(
        state.ends_with("qtcloud-feishu/招聘.json"),
        "{}",
        state.display()
    );
    let config = qtcloud_feishu_cli::paths::config_path("招聘流程").unwrap();
    assert!(
        config.ends_with("qtcloud-feishu/招聘流程.yaml"),
        "{}",
        config.display()
    );
    let mirror = qtcloud_feishu_cli::paths::mirror_dir().unwrap();
    assert!(mirror.ends_with("qtcloud-feishu"), "{}", mirror.display());
    assert!(mirror.is_absolute());
}
