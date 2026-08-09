use crate::parser::{Expression, Literal, Program, Statement};

impl Program {
    pub fn dump(&self) {
        println!("Program");

        for (i, stmt) in self.body.iter().enumerate() {
            stmt.dump_impl(String::new(), i == self.body.len() - 1);
        }
    }
}

impl Statement {
    fn dump_impl(&self, prefix: String, last: bool) {
        let branch = if last { "└─ " } else { "├─ " };

        let next_prefix = if last {
            format!("{prefix}   ")
        } else {
            format!("{prefix}│  ")
        };

        match self {
            Statement::FunctionDeclaration { name, params, body } => {
                println!("{prefix}{branch}Function {}({})", name, params.join(", "));

                for (i, stmt) in body.iter().enumerate() {
                    stmt.dump_impl(next_prefix.clone(), i == body.len() - 1);
                }
            }

            Statement::Return(Some(expr)) => {
                println!("{prefix}{branch}Return");

                expr.dump_impl(next_prefix, true);
            }

            Statement::Return(None) => {
                println!("{prefix}{branch}Return");
            }

            Statement::Expression(expr) => {
                expr.dump_impl(prefix, last);
            }

            Statement::VariableDeclaration { name, init, .. } => {
                println!("{prefix}{branch}Var {name}");

                if let Some(expr) = init {
                    expr.dump_impl(next_prefix, true);
                }
            }
        }
    }
}

impl Expression {
    fn dump_impl(&self, prefix: String, last: bool) {
        let branch = if last { "└─ " } else { "├─ " };

        let next_prefix = if last {
            format!("{prefix}   ")
        } else {
            format!("{prefix}│  ")
        };

        match self {
            Expression::Binary { op, left, right } => {
                println!("{prefix}{branch}Binary({op:?})");

                left.dump_impl(next_prefix.clone(), false);

                right.dump_impl(next_prefix, true);
            }

            Expression::Identifier(name) => {
                println!("{prefix}{branch}Identifier({name})");
            }

            Expression::Literal(lit) => match lit {
                Literal::Number(n) => {
                    println!("{prefix}{branch}Number({n})");
                }

                Literal::String(s) => {
                    println!("{prefix}{branch}String(\"{s}\")");
                }

                Literal::Boolean(b) => {
                    println!("{prefix}{branch}Boolean({b})");
                }

                Literal::Null => {
                    println!("{prefix}{branch}Null");
                }

                Literal::Undefined => {
                    println!("{prefix}{branch}Undefined");
                }
            },

            Expression::Call { callee, args } => {
                println!("{prefix}{branch}Call");

                let total = args.len() + 1;

                callee.dump_impl(next_prefix.clone(), total == 1);

                for (i, arg) in args.iter().enumerate() {
                    arg.dump_impl(next_prefix.clone(), i == args.len() - 1);
                }
            }

            Expression::New { callee, args } => {
                println!("{prefix}{branch}New");

                let total = args.len() + 1;
                callee.dump_impl(next_prefix.clone(), total == 1);

                for (i, arg) in args.iter().enumerate() {
                    arg.dump_impl(next_prefix.clone(), i == args.len() - 1);
                }
            }

            Expression::Unary { op, arg } => {
                println!("{prefix}{branch}Unary({op:?})");

                arg.dump_impl(next_prefix, true);
            }

            Expression::Assignment { left, right } => {
                println!("{prefix}{branch}Assignment");

                left.dump_impl(next_prefix.clone(), false);

                right.dump_impl(next_prefix, true);
            }

            Expression::This => {
                println!("{prefix}{branch}This");
            }

            Expression::ArrayLiteral(items) => {
                println!("{prefix}{branch}Array");

                for (i, item) in items.iter().enumerate() {
                    item.dump_impl(next_prefix.clone(), i == items.len() - 1);
                }
            }

            Expression::ObjectLiteral(props) => {
                println!("{prefix}{branch}Object");

                for (i, (name, value)) in props.iter().enumerate() {
                    let is_last = i == props.len() - 1;

                    let prop_branch = if is_last { "└─ " } else { "├─ " };

                    println!("{next_prefix}{prop_branch}{name}");

                    value.dump_impl(format!("{next_prefix}│  "), true);
                }
            }

            Expression::MemberAccess {
                object,
                property,
                computed: _,
            } => {
                println!("{prefix}{branch}MemberAccess");

                object.dump_impl(next_prefix.clone(), false);

                property.dump_impl(next_prefix, true);
            }

            Expression::Function { params, body, .. } => {
                println!("{prefix}{branch}FunctionExpr({})", params.join(", "));

                for (i, stmt) in body.iter().enumerate() {
                    stmt.dump_impl(next_prefix.clone(), i == body.len() - 1);
                }
            }

            Expression::ArrowFunction { params, body } => {
                println!("{prefix}{branch}ArrowFunction({})", params.join(", "));

                for (i, statement) in body.iter().enumerate() {
                    statement.dump_impl(next_prefix.clone(), i == body.len() - 1);
                }
            }
        }
    }
}
