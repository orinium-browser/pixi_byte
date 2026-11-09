use pixi_byte::Lexer;
use pixi_byte::parser::{
    BindingPattern, Expression, MetaPropertyKind, Parser, PatternKey, Statement, VarKind,
};
use pixi_byte::{JSEngine, JSError};

fn parse_program(source: &str) -> pixi_byte::parser::Program {
    Parser::new(Lexer::new(source))
        .expect("parser")
        .parse()
        .expect("program")
}

fn compile_error(source: &str) -> JSError {
    let mut engine = JSEngine::new();
    engine.eval(source).expect_err("expected compile error")
}

#[test]
fn with_statement_block_form_parses() {
    let program = parse_program("with (obj) { let x = 1; }");
    match &program.body[0] {
        Statement::With { object, body } => {
            assert!(matches!(object, Expression::Identifier(name) if name == "obj"));
            assert!(matches!(body.as_ref(), Statement::Block(_)));
        }
        other => panic!("expected With, got {other:?}"),
    }
}

#[test]
fn with_statement_single_statement_form_parses() {
    let program = parse_program("with (obj) x = 1;");
    match &program.body[0] {
        Statement::With { body, .. } => {
            assert!(matches!(body.as_ref(), Statement::Expression(_)));
        }
        other => panic!("expected With, got {other:?}"),
    }
}

#[test]
fn debugger_statement_parses() {
    let program = parse_program("debugger;");
    assert!(matches!(program.body[0], Statement::Debugger));
}

#[test]
fn new_target_meta_property_parses() {
    let program = parse_program("function C() { return new.target; }");
    match &program.body[0] {
        Statement::FunctionDeclaration { body, .. } => match &body[0] {
            Statement::Return(Some(Expression::MetaProperty { meta, property })) => {
                assert_eq!(*meta, MetaPropertyKind::NewTarget);
                assert_eq!(property, "target");
            }
            other => panic!("expected MetaProperty return, got {other:?}"),
        },
        other => panic!("expected function, got {other:?}"),
    }
}

#[test]
fn import_meta_property_parses() {
    let program = parse_program("import.meta;");
    match &program.body[0] {
        Statement::Expression(Expression::MetaProperty { meta, property }) => {
            assert_eq!(*meta, MetaPropertyKind::ImportMeta);
            assert_eq!(property, "meta");
        }
        other => panic!("expected MetaProperty, got {other:?}"),
    }
}

#[test]
fn ordinary_member_access_is_not_meta_property() {
    let object_target = parse_program("object.target;");
    assert!(matches!(
        object_target.body[0],
        Statement::Expression(Expression::MemberAccess { .. })
    ));

    let object_meta = parse_program("object.meta;");
    assert!(matches!(
        object_meta.body[0],
        Statement::Expression(Expression::MemberAccess { .. })
    ));
}

#[test]
fn for_await_of_preserves_is_await() {
    let program = parse_program("async function run(xs){ for await (const x of xs) {} }");
    match &program.body[0] {
        Statement::FunctionDeclaration { body, .. } => match &body[0] {
            Statement::ForOf {
                binding,
                kind,
                right,
                is_await,
                ..
            } => {
                assert!(*is_await);
                assert!(matches!(kind, Some(VarKind::Const)));
                assert!(matches!(
                    binding,
                    BindingPattern::Identifier(name) if name == "x"
                ));
                assert!(matches!(right, Expression::Identifier(name) if name == "xs"));
            }
            other => panic!("expected ForOf, got {other:?}"),
        },
        other => panic!("expected function, got {other:?}"),
    }
}

#[test]
fn for_of_without_await_sets_is_await_false() {
    let program = parse_program("for (const x of xs) {}");
    match &program.body[0] {
        Statement::ForOf { is_await, .. } => assert!(!*is_await),
        other => panic!("expected ForOf, got {other:?}"),
    }
}

fn assert_internal_error_contains(err: JSError, needle: &str) {
    match err {
        JSError::InternalError(message) => {
            assert!(message.contains(needle), "message was: {message}")
        }
        other => panic!("expected InternalError, got {other:?}"),
    }
}

#[test]
fn compiler_rejects_new_constructs_with_internal_errors() {
    assert_internal_error_contains(compile_error("with (obj) { 1; }"), "unsupported with");
    assert_internal_error_contains(compile_error("debugger;"), "unsupported debugger");
    assert_internal_error_contains(
        compile_error("function C(){ return new.target; }"),
        "unsupported meta property",
    );
    assert_internal_error_contains(compile_error("import.meta;"), "unsupported meta property");
    assert_internal_error_contains(
        compile_error("async function run(xs){ for await (const x of xs) {} }"),
        "unsupported for await",
    );
}

#[test]
fn unsupported_compilation_does_not_mutate_existing_global_state() {
    let mut engine = JSEngine::new();
    engine.eval("var x = 1;").expect("seed global");
    let err = engine.eval("x = 2; debugger;").expect_err("expected error");
    assert_internal_error_contains(err, "unsupported debugger");
    let value = engine.eval("x;").expect("read global");
    assert_eq!(value, pixi_byte::JSValue::from_number(1.0));
}

#[test]
fn existing_statement_and_expression_behavior_regressions() {
    let mut engine = JSEngine::new();
    let if_value = engine.eval("let a=1; if (a) { a = a + 1; } a;").unwrap();
    assert_eq!(if_value, pixi_byte::JSValue::from_number(2.0));

    let loop_value = engine
        .eval("let s=0; for (let i=0;i<3;i++){ s+=i; } s;")
        .unwrap();
    assert_eq!(loop_value, pixi_byte::JSValue::from_number(3.0));

    let member_value = engine.eval("const o={a:1}; o.a + o['a'];").unwrap();
    assert_eq!(member_value, pixi_byte::JSValue::from_number(2.0));
}

#[test]
fn computed_binding_pattern_still_parses() {
    let program = parse_program("const {[k]: value} = source;");
    match &program.body[0] {
        Statement::PatternDeclaration { binding, .. } => match binding {
            BindingPattern::Object(props) => {
                assert!(matches!(props[0].0, PatternKey::Computed(_)));
            }
            other => panic!("expected object binding, got {other:?}"),
        },
        other => panic!("expected pattern declaration, got {other:?}"),
    }
}
