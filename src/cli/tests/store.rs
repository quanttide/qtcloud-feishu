//! 落盘契约：页序、附件、游标。

mod fake_lark_cli;

use qtcloud_feishu_cli::store::RawStore;

#[test]
fn 页序接着排不覆盖历史页() {
    let store = RawStore::new(fake_lark_cli::tmp_dir("store_seq"));
    let raw = serde_json::json!({"ok": true, "data": {}});
    let first = store.save_response("mail", "list", &raw).unwrap();
    let second = store.save_response("mail", "list", &raw).unwrap();
    assert!(first.ends_with("0001.json"));
    assert!(second.ends_with("0002.json"));
}

#[test]
fn 附件各自成目录并保持原名() {
    let store = RawStore::new(fake_lark_cli::tmp_dir("store_att"));
    let path = store
        .save_attachment("m-1-att-0", "简历.txt", b"hi")
        .unwrap();
    assert_eq!(path.parent().unwrap().file_name().unwrap(), "m-1-att-0");
    assert_eq!(path.file_name().unwrap(), "简历.txt");
    assert_eq!(std::fs::read(&path).unwrap(), b"hi");
}

#[test]
fn 游标另存并带时间戳() {
    let store = RawStore::new(fake_lark_cli::tmp_dir("store_cursor"));
    store.set_cursor("mail", "2026-10-02").unwrap();
    let text = std::fs::read_to_string(store.cursor_path("mail")).unwrap();
    assert!(text.contains("2026-10-02"), "{text}");
    assert!(text.contains("updated_at"), "{text}");
}
