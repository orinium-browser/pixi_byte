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
    // unsupported by the `regex` crate. It must degrade to a no-op (never
    // match), so duplicate-flag detection never misfires on valid flags like
    // "g". Each input must come back unchanged.
    let out = eval("JSON.stringify(['g','gg','ggi','gmi'].map(function(s){return s.replace(/([\\s\\S])(?=[\\s\\S]*\\1)/g,'')}))");
    let ok = out == "[\"g\",\"gg\",\"ggi\",\"gmi\"]";
    println!("dup1 => {out} (ok={ok})");
    assert!(ok, "got {out}");
}

#[test]
fn repl_dup_flag2() {
    let out = eval("JSON.stringify(String.prototype.replace.call('g',/([\\s\\S])(?=[\\s\\S]*\\1)/g,''))");
    println!("dup2 => {out}");
    assert_eq!(out, "\"g\"");
}