//! debug_assert 発火の再現・修正確認用（debug ビルドで実行する使い捨てテスト）。
use pixi_byte::JSEngine;

#[test]
fn inline_string_values_drop_cleanly_in_debug_builds() {
    let mut engine = JSEngine::new();
    // インライン文字列（<=5 バイト）がスタックに push/pop され、
    // 通常の drop 経路を通るワークロード。
    let result = engine
        .eval(
            r#"
            let s = "foo.bar.baz.";
            let parts = s.split(".");
            let total = "";
            for (const p of parts) { total += p; }
            let c = s[0];
            let t = c + "x";
            t.length + parts.length
        "#,
        )
        .unwrap();
    // t = "f" + "x" → length 2、split 結果は 4 要素（末尾の空文字列含む）。
    assert_eq!(result.to_string(), "6");
}
