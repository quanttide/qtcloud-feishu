//! 落盘契约：页序、附件、游标。

mod support;

use qtcloud_feishu_cli::store::RawStore;

#[test]
fn 页序接着排_重跑不覆盖() {
    let store = RawStore::new(support::tmp_dir("store_seq"));
    let raw = serde_json::json!({"ok": true, "data": {}});
    let first = store.save_response("mail", "list", &raw).unwrap();
    let second = store.save_response("mail", "list", &raw).unwrap();
    assert!(first.ends_with("0001.json"));
    assert!(second.ends_with("0002.json"));
}

#[test]
fn 附件落在_附件id_原名下_游标另存() {
    let store = RawStore::new(support::tmp_dir("store_att"));
    let path = store
        .save_attachment("m-1-att-0", "简历.txt", b"hi")
        .unwrap();
    assert_eq!(path.parent().unwrap().file_name().unwrap(), "m-1-att-0");
    assert_eq!(std::fs::read(&path).unwrap(), b"hi");

    store.set_cursor("mail", "2026-10-02").unwrap();
    let text = std::fs::read_to_string(store.cursor_path("mail")).unwrap();
    assert!(text.contains("2026-10-02"), "{text}");
    assert!(text.contains("updated_at"), "游标带时间戳：{text}");
}
