use pixi_byte::JSEngine;

fn main() {
    println!("PixiByte JavaScript Engine v0.1.0");
    println!("================================\n");

    let mut engine = JSEngine::new();

    let test_code = r"
        const a = {
            x: 1,
            get: function () {
                return this.x;
            }
        };

        const b = {
            x: 2,
            get: a.get
        };

        b.get();
    ";

    match engine.eval(test_code) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => eprintln!("Error: {}", e),
    }
}
