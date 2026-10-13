//! 本机落盘位置：按 XDG 基目录规范取，四类各归各的目录。
//!
//! ```text
//! 存什么                          目录                                    环境变量         缺省
//! 飞书原样留档（镜像）            <data>/qtcloud-feishu/                  XDG_DATA_HOME    ~/.local/share
//!   其中的 raw/ 与 cursor/ 就在这个目录里
//! 流程状态（谁在哪一格）          <state>/qtcloud-feishu/<名字>.json       XDG_STATE_HOME   ~/.local/state
//! 定义与流程（我们的话）          <config>/qtcloud-feishu/<名字>.yaml      XDG_CONFIG_HOME  ~/.config
//! 纯可重建的中间产物              <cache>/qtcloud-feishu/                  XDG_CACHE_HOME   ~/.cache
//! ```
//!
//! 为什么这么分：镜像里的 `raw/` 是**一页一份、从覆盖**的留档（页序只加不改），飞书那边给不回
//! 当时那一页，所以它算数据而不是缓存；流程状态能重建，但重启后要还在，落在 state；
//! 定义与流程是配置；将来渲染出来的 CSV / HTML 这类随时能重做的，落 cache。
//!
//! 规矩（照规范）：变量**没设或为空**用缺省；变量给了**相对路径**当没设（规范要求绝对路径）；
//! `HOME` 也没有就报错，不猜。

use std::path::PathBuf;

/// 应用名：XDG 目录下再分一层，免得跟别的东西混在一起。
pub const APP: &str = "qtcloud-feishu";

/// 取一个 XDG 目录：`var` 给了绝对路径就用它，否则 `HOME/<fallback>`。
///
/// `get` 是环境变量读取（真实现是 [`std::env::var`]，测试给一张表）。
pub fn locate(
    var: &str,
    fallback: &[&str],
    get: &dyn Fn(&str) -> Option<String>,
) -> Result<PathBuf, String> {
    if let Some(value) = get(var).filter(|value| !value.is_empty()) {
        let path = PathBuf::from(&value);
        if path.is_absolute() {
            return Ok(path.join(APP));
        }
        // 相对路径按规范当没设，落到下面的缺省
    }
    let home = get("HOME")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{var} 与 HOME 都没给：定不出落盘目录"))?;
    Ok(fallback
        .iter()
        .fold(PathBuf::from(home), |base, part| base.join(part))
        .join(APP))
}

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// 镜像根：`raw/<资源>/<动作>/<页序>.json`、`raw/mail/attachments/…`、`cursor/` 都在它下面。
pub fn mirror_dir() -> Result<PathBuf, String> {
    locate("XDG_DATA_HOME", &[".local", "share"], &env)
}

/// 流程状态放哪：`<state>/qtcloud-feishu/<名字>.json`。
pub fn state_path(name: &str) -> Result<PathBuf, String> {
    Ok(locate("XDG_STATE_HOME", &[".local", "state"], &env)?.join(format!("{name}.json")))
}

/// 定义与流程的缺省位置：`<config>/qtcloud-feishu/<名字>.yaml`。
pub fn config_path(name: &str) -> Result<PathBuf, String> {
    Ok(locate("XDG_CONFIG_HOME", &[".config"], &env)?.join(format!("{name}.yaml")))
}

/// 纯可重建的中间产物放哪。
pub fn cache_dir() -> Result<PathBuf, String> {
    locate("XDG_CACHE_HOME", &[".cache"], &env)
}
