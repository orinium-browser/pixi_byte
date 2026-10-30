use pixi_byte::JSEngine;

fn eval(code: &str) -> String {
    let mut engine = JSEngine::new();
    match engine.eval(code) {
        Ok(v) => v.to_string(),
        Err(e) => format!("ERR: {e:?}"),
    }
}

#[test]
fn repl_dup_flag() {
    // The dedupe helper `([\s\S])(?=[\s\S]*\1)` uses a lookahead + backreference,
    // which the linear-time `regex` crate cannot compile. The `fancy-regex`
    // fallback supports it, so duplicate flag characters are actually removed
    // (each character that recurs later in the string is dropped).
    let out = eval(
        "JSON.stringify(['g','gg','ggi','gmi'].map(function(s){return s.replace(/([\\s\\S])(?=[\\s\\S]*\\1)/g,'')}))",
    );
    let expected = "[\"g\",\"g\",\"gi\",\"gmi\"]";
    let ok = out == expected;
    println!("dup1 => {out} (ok={ok})");
    assert!(ok, "got {out}");
}

#[test]
fn repl_dup_flag2() {
    let out =
        eval("JSON.stringify(String.prototype.replace.call('g',/([\\s\\S])(?=[\\s\\S]*\\1)/g,''))");
    println!("dup2 => {out}");
    assert_eq!(out, "\"g\"");
}
