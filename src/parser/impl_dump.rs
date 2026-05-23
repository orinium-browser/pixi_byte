use crate::parser::{Expression, Literal, Program, Statement};

impl Program {
    pub fn dump(&self) {
        self.dump_impl(0);
    }

    fn dump_impl(&self, depth: usize) {
        let indent = "│  ".repeat(depth);

        println!("{indent}Program");

        for stmt in &self.body {
            stmt.dump_impl(depth + 1);
        }
    }
}

impl Statement {
    fn dump_impl(&self, depth: usize) {
        let indent = "│  ".repeat(depth);

        match self {
            Statement::FunctionDeclaration { name, params, body } => {
                println!("{indent}├─ Function {}({})", name, params.join(", "));

                for stmt in body {
                    stmt.dump_impl(depth + 1);
                }
            }

            Statement::Return(Some(expr)) => {
                println!("{indent}├─ Return");
                expr.dump_impl(depth + 1);
            }

            Statement::Expression(expr) => {
                expr.dump_impl(depth);
            }

            _ => {}
        }
    }
}

impl Expression {
    fn dump_impl(&self, depth: usize) {
        let indent = "│  ".repeat(depth);

        match self {
            Expression::Binary { op, left, right } => {
                println!("{indent}├─ Binary({op:?})");

                left.dump_impl(depth + 1);
                right.dump_impl(depth + 1);
            }

            Expression::Identifier(name) => {
                println!("{indent}├─ Identifier({name})");
            }

            Expression::Literal(Literal::Number(n)) => {
                println!("{indent}├─ Number({n})");
            }

            Expression::Call { callee, args } => {
                println!("{indent}├─ Call");

                callee.dump_impl(depth + 1);

                for arg in args {
                    arg.dump_impl(depth + 1);
                }
            }

            Expression::Unary { op, arg } => {
                println!("{indent}├─ Unary({op:?})");
                arg.dump_impl(depth + 1);
            }

            Expression::Assignment { left, right } => {
                println!("{indent}├─ Assignment");
                left.dump_impl(depth + 1);
                right.dump_impl(depth + 1);
            }

            Expression::This => {
                println!("{indent}├─ This");
            }

            Expression::ArrayLiteral(items) => {
                println!("{indent}├─ Array");

                for item in items {
                    item.dump_impl(depth + 1);
                }
            }

            Expression::ObjectLiteral(props) => {
                println!("{indent}├─ Object");

                for (name, value) in props {
                    println!("{indent}│  ├─ {name}");
                    value.dump_impl(depth + 2);
                }
            }

            _ => {
                println!("{indent}├─ <unimplemented>");
            }
        }
    }
}
