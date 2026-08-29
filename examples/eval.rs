use pixi_byte::{EvalOptions, JSEngine};
use std::{env, fs};

fn main() {
    let (eval_options, args) = parse_args();

    let path = args
        .get(0)
        .expect("Usage: cargo run --example eval <file.js>");

    let source = fs::read_to_string(&path).expect("Failed to read file");

    let mut engine = JSEngine::new();

    install(engine.global_mut());

    match engine.eval_with_options(&source, &eval_options) {
        Ok(v) => println!("=> {}", v),
        Err(e) => eprintln!("Error: {e}"),
    }
}

pub fn parse_args() -> (EvalOptions, Vec<String>) {
    let mut options = EvalOptions {
        dump_ast: false,
        dump_bytecode: false,
    };

    let mut args = Vec::new();

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--dump-ast" => {
                options.dump_ast = true;
            }
            "--dump-bytecode" => {
                options.dump_bytecode = true;
            }
            _ if arg.starts_with("-") => {
                eprintln!("Unknown option: {}", arg);
            }
            _ => {
                args.push(arg);
            }
        }
    }

    (options, args)
}

use std::cell::RefCell;
use std::rc::Rc;

use pixi_byte::{JSResult, JSValue, value::JSObject};

pub fn install(global: &Rc<RefCell<JSObject>>) {
    global.borrow_mut().set(
        "println".to_string(),
        JSValue::from_native_function(println_native),
    );
}

fn println_native(_vm: &mut pixi_byte::vm::VM, args: Vec<JSValue>) -> JSResult<JSValue> {
    for (i, arg) in args.iter().enumerate() {
        if i != 0 {
            print!(" ");
        }

        print!("{}", arg);
        // Or: print!("{}", arg.to_string());
    }

    println!();

    Ok(JSValue::undefined())
}
