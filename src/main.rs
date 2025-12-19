use pixi_byte::JSEngine;

fn main() {
    println!("PixiByte JavaScript Engine v0.1.0");
    println!("================================\n");

    let mut engine = JSEngine::new();

    let test_code = "1 + 2";

    match engine.eval(test_code) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => eprintln!("Error: {}", e),
    }
}
