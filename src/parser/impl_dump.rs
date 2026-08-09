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

            Statement::VariableDeclaration { declarations, .. } => {
                println!("{prefix}{branch}Var");
                for (index, (name, init)) in declarations.iter().enumerate() {
                    let declaration_last = index + 1 == declarations.len();
                    println!("{next_prefix}{}{name}", if declaration_last { "└─ " } else { "├─ " });
                    if let Some(expr) = init {
                        expr.dump_impl(
                            format!("{next_prefix}{}", if declaration_last { "   " } else { "│  " }),
                            true,
                        );
                    }
                }
            }
            Statement::If {
                test,
                consequent,
                alternate,
            } => {
                println!("{prefix}{branch}If");
                test.dump_impl(next_prefix.clone(), consequent.is_empty() && alternate.is_none());
                for (index, stmt) in consequent.iter().enumerate() {
                    stmt.dump_impl(
                        next_prefix.clone(),
                        alternate.is_none() && index + 1 == consequent.len(),
                    );
                }
                if let Some(alternate) = alternate {
                    println!("{next_prefix}└─ Else");
                    for (index, stmt) in alternate.iter().enumerate() {
                        stmt.dump_impl(
                            format!("{next_prefix}   "),
                            index + 1 == alternate.len(),
                        );
                    }
                }
            }
            Statement::While { test, body } => {
                println!("{prefix}{branch}While");
                test.dump_impl(next_prefix.clone(), body.is_empty());
                for (index, stmt) in body.iter().enumerate() {
                    stmt.dump_impl(next_prefix.clone(), index + 1 == body.len());
                }
            }
            Statement::DoWhile { body, test } => {
                println!("{prefix}{branch}DoWhile");
                for statement in body {
                    statement.dump_impl(next_prefix.clone(), false);
                }
                test.dump_impl(next_prefix, true);
            }
            Statement::For {
                init,
                test,
                update,
                body,
            } => {
                println!("{prefix}{branch}For");
                if let Some(init) = init {
                    init.dump_impl(next_prefix.clone(), false);
                }
                if let Some(test) = test {
                    test.dump_impl(next_prefix.clone(), false);
                }
                for update in update {
                    update.dump_impl(next_prefix.clone(), body.is_empty());
                }
                for (index, stmt) in body.iter().enumerate() {
                    stmt.dump_impl(next_prefix.clone(), index + 1 == body.len());
                }
            }
            Statement::ForIn {
                binding,
                right,
                body,
            } => {
                println!("{prefix}{branch}ForIn({binding})");
                right.dump_impl(next_prefix.clone(), body.is_empty());
                for (index, statement) in body.iter().enumerate() {
                    statement.dump_impl(next_prefix.clone(), index + 1 == body.len());
                }
            }
            Statement::Throw(expression) => {
                println!("{prefix}{branch}Throw");
                expression.dump_impl(next_prefix, true);
            }
            Statement::Try {
                block,
                handler,
                finalizer,
            } => {
                println!("{prefix}{branch}Try");
                for statement in block {
                    statement.dump_impl(next_prefix.clone(), false);
                }
                if let Some((binding, body)) = handler {
                    println!(
                        "{next_prefix}├─ Catch{}",
                        binding
                            .as_ref()
                            .map(|binding| format!("({binding})"))
                            .unwrap_or_default()
                    );
                    for statement in body {
                        statement.dump_impl(next_prefix.clone(), false);
                    }
                }
                if let Some(body) = finalizer {
                    println!("{next_prefix}└─ Finally");
                    for (index, statement) in body.iter().enumerate() {
                        statement.dump_impl(next_prefix.clone(), index + 1 == body.len());
                    }
                }
            }
            Statement::Switch {
                discriminant,
                cases,
            } => {
                println!("{prefix}{branch}Switch");
                discriminant.dump_impl(next_prefix.clone(), cases.is_empty());
                for (case_index, (test, body)) in cases.iter().enumerate() {
                    let case_last = case_index + 1 == cases.len();
                    if let Some(test) = test {
                        println!("{next_prefix}{}Case", if case_last { "└─ " } else { "├─ " });
                        test.dump_impl(next_prefix.clone(), body.is_empty());
                    } else {
                        println!(
                            "{next_prefix}{}Default",
                            if case_last { "└─ " } else { "├─ " }
                        );
                    }
                    for (index, statement) in body.iter().enumerate() {
                        statement.dump_impl(next_prefix.clone(), index + 1 == body.len());
                    }
                }
            }
            Statement::Break => println!("{prefix}{branch}Break"),
            Statement::Continue => println!("{prefix}{branch}Continue"),
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
            Expression::Update {
                arg,
                increment,
                prefix,
            } => {
                let operator = if *increment { "++" } else { "--" };
                let position = if *prefix { "Prefix" } else { "Postfix" };
                println!("{prefix}{branch}{position}Update({operator})");
                arg.dump_impl(next_prefix, true);
            }
            Expression::Conditional {
                test,
                consequent,
                alternate,
            } => {
                println!("{prefix}{branch}Conditional");
                test.dump_impl(next_prefix.clone(), false);
                consequent.dump_impl(next_prefix.clone(), false);
                alternate.dump_impl(next_prefix, true);
            }
            Expression::Sequence(expressions) => {
                println!("{prefix}{branch}Sequence");
                for (index, expression) in expressions.iter().enumerate() {
                    expression.dump_impl(next_prefix.clone(), index + 1 == expressions.len());
                }
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
            Expression::RegExpLiteral { pattern, flags } => {
                println!("{prefix}{branch}RegExp(/{pattern}/{flags})");
            }

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
