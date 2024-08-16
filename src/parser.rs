use std::io::BufReader;

use crate::{
    error::Error,
    scanner::{JoinOp, Scanner, SignOp, Token},
};

// Expr represents an individual tokenized expression consisting
// of left operand, operator and a right operand.
#[derive(Default, Clone)]
pub struct Expr {
    pub left: Token,
    pub op: SignOp,
    pub right: Token,
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{} {} {}}}", self.left, self.op, self.right)
    }
}

impl Expr {
    fn is_zero(&self) -> bool {
        self.op == SignOp::None && self.left == Token::None && self.right == Token::None
    }
}

// ExprGroup represents a wrapped expression and its join type.
//
// The group's Item could be either an `Expr` instance or `ExprGroups` slice (for nested expressions).
pub struct ExprGroup {
    pub join: JoinOp,
    pub item: ExprGroupItem,
}

impl std::fmt::Display for ExprGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{} {}}}", self.join, self.item)
    }
}

pub enum ExprGroupItem {
    Expr(Expr),
    ExprGroups(ExprGroups),
}

impl std::fmt::Display for ExprGroupItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprGroupItem::Expr(expr) => write!(f, "{expr}",),
            ExprGroupItem::ExprGroups(expr_groups) => write!(f, "{expr_groups}",),
        }
    }
}

pub struct ExprGroups {
    expr_groups: Vec<ExprGroup>,
}

impl ExprGroups {
    fn new() -> Self {
        Self {
            expr_groups: Vec::new(),
        }
    }

    pub fn get(&self) -> &Vec<ExprGroup> {
        &self.expr_groups
    }

    fn push(&mut self, value: ExprGroup) {
        self.expr_groups.push(value)
    }

    fn len(&self) -> usize {
        self.expr_groups.len()
    }
}

impl std::fmt::Display for ExprGroups {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for (i, expr_group) in self.expr_groups.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{{{} {}}}", expr_group.join, expr_group.item)?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

// parser's state machine steps
#[derive(PartialEq)]
enum Step {
    BeforeSign,
    Sign,
    AfterSign,
    Join,
}

// Parse parses the provided text and returns its processed AST
// in the form of `ExprGroup` slice(s).
//
// Comments and whitespaces are ignored.
pub fn parse(text: &str) -> Result<ExprGroups, Error> {
    let mut result = ExprGroups::new();
    let mut scanner = Scanner::new(BufReader::new(text.as_bytes()))?;
    let mut step = Step::BeforeSign;
    let mut join = JoinOp::And;

    let mut expr = Expr::default();

    loop {
        let t = scanner.scan()?;

        if matches!(t, Token::Eof(_)) {
            break;
        }

        if matches!(t, Token::Ws(_)) || matches!(t, Token::Comment(_)) {
            continue;
        }

        if matches!(t, Token::Group(_)) {
            let group_result = parse(t.literal())?;

            // append only if non-empty group
            if group_result.len() > 0 {
                result.push(ExprGroup {
                    join,
                    item: ExprGroupItem::ExprGroups(group_result),
                })
            }

            step = Step::Join;
            continue;
        }

        match step {
            Step::BeforeSign => {
                if !matches!(t, Token::Identifier(_))
                    && !matches!(t, Token::Text(_))
                    && !matches!(t, Token::Number(_))
                {
                    return Err(Error::Unexpected(format!(
                        "Expected left operand (identifier, text or number), got {} ({})",
                        t.literal(),
                        t.kind()
                    )));
                }

                expr = Expr {
                    left: t,
                    ..Default::default()
                };

                step = Step::Sign
            }
            Step::Sign => {
                if !matches!(t, Token::Sign(_)) {
                    return Err(Error::Unexpected(format!(
                        "Expected a sign operator, got {} ({})",
                        t.literal(),
                        t.kind()
                    )));
                }

                expr.op = match SignOp::from_str(t.literal()) {
                    Some(op) => op,
                    None => {
                        return Err(Error::Unexpected(format!(
                            "Expected a sign operator, got {} ({})",
                            t.literal(),
                            t.kind()
                        )))
                    }
                };

                step = Step::AfterSign;
            }
            Step::AfterSign => {
                if !matches!(t, Token::Identifier(_))
                    && !matches!(t, Token::Text(_))
                    && !matches!(t, Token::Number(_))
                {
                    return Err(Error::Unexpected(format!(
                        "Expected right operand (identifier, text or number), got {} ({})",
                        t.literal(),
                        t.kind(),
                    )));
                }

                expr.right = t;
                result.push(ExprGroup {
                    join,
                    item: ExprGroupItem::Expr(expr.clone()),
                });

                step = Step::Join;
            }
            Step::Join => {
                if !matches!(t, Token::Join(_)) {
                    return Err(Error::Unexpected(format!(
                        "Expected && or ||, got {} ({})",
                        t.literal(),
                        t.kind()
                    )));
                }

                join = match JoinOp::from_str(t.literal()) {
                    Some(join) => join,
                    None => {
                        return Err(Error::Unexpected(format!(
                            "Expected && or ||, got {} ({})",
                            t.literal(),
                            t.kind()
                        )))
                    }
                };

                step = Step::BeforeSign;
            }
        }
    }

    if step != Step::Join {
        if result.len() == 0 && expr.is_zero() {
            return Err(Error::Empty("Empty filter expression".to_owned()));
        }

        return Err(Error::Incomplete(
            "Invalid or incomplete filter expression".to_owned(),
        ));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::{parse, Expr},
        scanner::Token,
        SignOp,
    };

    #[test]
    fn test_expr_is_zero() {
        struct Scenario {
            expr: Expr,
            result: bool,
        }

        let scenarios = vec![
            Scenario {
                expr: Expr::default(),
                result: true,
            },
            Scenario {
                expr: Expr {
                    op: SignOp::AnyEq,
                    ..Default::default()
                },
                result: false,
            },
            Scenario {
                expr: Expr {
                    left: Token::Number("123".to_owned()),
                    ..Default::default()
                },
                result: false,
            },
            Scenario {
                expr: Expr {
                    left: Token::Ws("".to_owned()),
                    ..Default::default()
                },
                result: false,
            },
            Scenario {
                expr: Expr {
                    right: Token::Number("123".to_owned()),
                    ..Default::default()
                },
                result: false,
            },
            Scenario {
                expr: Expr {
                    right: Token::Ws("".to_owned()),
                    ..Default::default()
                },
                result: false,
            },
        ];

        for (i, s) in scenarios.iter().enumerate() {
            let v = s.expr.is_zero();
            assert!(
                v == s.result,
                "({}) Expected {}, got {} for \n{}",
                i,
                s.result,
                v,
                s.expr
            )
        }
    }

    #[test]
    fn test_parse() {
        struct Scenario {
            input: &'static str,
            expected_error: bool,
            expected_print: &'static str,
        }

        let scenarios = vec![
            Scenario {
                input: r"> 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"a >",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"a > >",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"a > %",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"a ! 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"a - 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"a + 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"1 - 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"1 + 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"> a 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"a || 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"a && 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test > 1 &&",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"|| test = 1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = 1 && ||",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = 1 && a",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r#"test = 1 && "a""#,
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = 1 a",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r#"test = 1 "a""#,
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = 1@test",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = .@test",
                expected_error: true,
                expected_print: r"[]",
            },
            // mismatched text quotes
            Scenario {
                input: r#"test = "demo'"#,
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r#"test = 'demo""#,
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r#"test = 'demo'""#,
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = 'demo''",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r#"test = "demo"'"#,
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r#"test = "demo"""#,
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r#"test = ""demo""#,
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = ''demo''",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = `demo`",
                expected_error: true,
                expected_print: r"[]",
            },
            // comments
            Scenario {
                input: r"test = / demo",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = // demo",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"// demo",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"test = 123 // demo",
                expected_error: false,
                expected_print: r"[{&& {{identifier test} = {number 123}}}]",
            },
            Scenario {
                input: "test = // demo\n123",
                expected_error: false,
                expected_print: r"[{&& {{identifier test} = {number 123}}}]",
            },
            Scenario {
                input: r"
                    a = 123 &&
                    // demo
                    b = 456
                ",
                expected_error: false,
                expected_print: r"[{&& {{identifier a} = {number 123}}} {&& {{identifier b} = {number 456}}}]",
            },
            // valid simple expression and sign operators check
            Scenario {
                input: r"1=12",
                expected_error: false,
                expected_print: r"[{&& {{number 1} = {number 12}}}]",
            },
            Scenario {
                input: r"   1    =    12    ",
                expected_error: false,
                expected_print: r"[{&& {{number 1} = {number 12}}}]",
            },
            Scenario {
                input: r#""demo" != test"#,
                expected_error: false,
                expected_print: r"[{&& {{text demo} != {identifier test}}}]",
            },
            Scenario {
                input: r"a~1",
                expected_error: false,
                expected_print: r"[{&& {{identifier a} ~ {number 1}}}]",
            },
            Scenario {
                input: r"a !~ 1",
                expected_error: false,
                expected_print: r"[{&& {{identifier a} !~ {number 1}}}]",
            },
            Scenario {
                input: r"test>12",
                expected_error: false,
                expected_print: r"[{&& {{identifier test} > {number 12}}}]",
            },
            Scenario {
                input: r"test > 12",
                expected_error: false,
                expected_print: r"[{&& {{identifier test} > {number 12}}}]",
            },
            Scenario {
                input: r#"test >="test""#,
                expected_error: false,
                expected_print: r"[{&& {{identifier test} >= {text test}}}]",
            },
            Scenario {
                input: r"test<@demo.test2",
                expected_error: false,
                expected_print: r"[{&& {{identifier test} < {identifier @demo.test2}}}]",
            },
            Scenario {
                input: r#"1<="test""#,
                expected_error: false,
                expected_print: r"[{&& {{number 1} <= {text test}}}]",
            },
            Scenario {
                input: r#"1<="te'st""#,
                expected_error: false,
                expected_print: r"[{&& {{number 1} <= {text te'st}}}]",
            },
            Scenario {
                input: r#"demo='te\'st'"#,
                expected_error: false,
                expected_print: r"[{&& {{identifier demo} = {text te'st}}}]",
            },
            Scenario {
                input: r#"demo="te\'st""#,
                expected_error: false,
                expected_print: r"[{&& {{identifier demo} = {text te\'st}}}]",
            },
            Scenario {
                input: r#"demo="te\"st""#,
                expected_error: false,
                expected_print: r#"[{&& {{identifier demo} = {text te"st}}}]"#,
            },
            // invalid parenthesis
            Scenario {
                input: r"(a=1",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"a=1)",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"((a=1)",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"{a=1}",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"[a=1]",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"((a=1 || a=2) && c=1))",
                expected_error: true,
                expected_print: r"[]",
            },
            // valid parenthesis
            Scenario {
                input: r"()",
                expected_error: true,
                expected_print: r"[]",
            },
            Scenario {
                input: r"(a=1)",
                expected_error: false,
                expected_print: r"[{&& [{&& {{identifier a} = {number 1}}}]}]",
            },
            Scenario {
                input: r#"(a="test(")"#,
                expected_error: false,
                expected_print: r"[{&& [{&& {{identifier a} = {text test(}}}]}]",
            },
            Scenario {
                input: r#"(a="test)")"#,
                expected_error: false,
                expected_print: r"[{&& [{&& {{identifier a} = {text test)}}}]}]",
            },
            Scenario {
                input: r"((a=1))",
                expected_error: false,
                expected_print: r"[{&& [{&& [{&& {{identifier a} = {number 1}}}]}]}]",
            },
            Scenario {
                input: r"a=1 || 2!=3",
                expected_error: false,
                expected_print: r"[{&& {{identifier a} = {number 1}}} {|| {{number 2} != {number 3}}}]",
            },
            Scenario {
                input: r"a=1 && 2!=3",
                expected_error: false,
                expected_print: r"[{&& {{identifier a} = {number 1}}} {&& {{number 2} != {number 3}}}]",
            },
            Scenario {
                input: r#"a=1 && 2!=3 || "b"=a"#,
                expected_error: false,
                expected_print: r"[{&& {{identifier a} = {number 1}}} {&& {{number 2} != {number 3}}} {|| {{text b} = {identifier a}}}]",
            },
            Scenario {
                input: r#"(a=1 && 2!=3) || "b"=a"#,
                expected_error: false,
                expected_print: r"[{&& [{&& {{identifier a} = {number 1}}} {&& {{number 2} != {number 3}}}]} {|| {{text b} = {identifier a}}}]",
            },
            Scenario {
                input: r"((a=1 || a=2) && (c=1))",
                expected_error: false,
                expected_print: r"[{&& [{&& [{&& {{identifier a} = {number 1}}} {|| {{identifier a} = {number 2}}}]} {&& [{&& {{identifier c} = {number 1}}}]}]}]",
            },
            // https://github.com/pocketbase/pocketbase/issues/5017
            Scenario {
                input: r#"(a='"')"#,
                expected_error: false,
                expected_print: r#"[{&& [{&& {{identifier a} = {text "}}}]}]"#,
            },
            Scenario {
                input: r"(a='\'')",
                expected_error: false,
                expected_print: r"[{&& [{&& {{identifier a} = {text '}}}]}]",
            },
            Scenario {
                input: r#"(a="'")"#,
                expected_error: false,
                expected_print: r"[{&& [{&& {{identifier a} = {text '}}}]}]",
            },
            Scenario {
                input: r#"(a="\"")"#,
                expected_error: false,
                expected_print: r#"[{&& [{&& {{identifier a} = {text "}}}]}]"#,
            },
        ];

        for (i, scenario) in scenarios.iter().enumerate() {
            let v = match parse(scenario.input) {
                Ok(v) => {
                    assert!(
                        !scenario.expected_error,
                        "({}) Expected error, got ok ({})",
                        i, v
                    );
                    v
                }
                Err(err) => {
                    assert!(
                        scenario.expected_error,
                        "({}) Did not expect error, got {} ({}).",
                        i, err, scenario.input
                    );
                    continue;
                }
            };

            let v_print = v.to_string();

            assert!(
                v_print == scenario.expected_print,
                "({}) Expected {}, got {}",
                i,
                scenario.expected_print,
                v_print
            )
        }
    }
}
