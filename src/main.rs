use pixi_byte::JSEngine;

fn main() {
    println!("PixiByte JavaScript Engine v0.1.0");
    println!("================================\n");

    let mut engine = JSEngine::new();

    let test_code = r"
        function add(a, b) {
            return this.x + a + b;
        }

        var obj = { x: 100 };

        add.call(obj, 1, 2)
        add.apply(obj, [1, 2])
    ";

    match engine.eval(test_code) {
        Ok(result) => println!("Result: {:?}", result),
        Err(e) => eprintln!("Error: {}", e),
    }
}
