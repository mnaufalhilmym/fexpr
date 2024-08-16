use std::io::{BufReader, Read};

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{bytes, error::Error};

// EOF represents a marker char for the end of the reader.
const EOF: char = '\0';

// JoinOp represents a join type operator.
#[derive(Clone, Copy)]
pub enum JoinOp {
    // supported join type operators
    And,
    Or,
}

impl JoinOp {
    pub fn from_str(str: &str) -> Option<Self> {
        match str {
            "&&" => Some(Self::And),
            "||" => Some(Self::Or),
            _ => None,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::And => "&&",
            Self::Or => "||",
        }
    }
}

impl std::fmt::Display for JoinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// SignOp represents an expression sign operator.
#[derive(Default, PartialEq, Clone)]
pub enum SignOp {
    // supported expression sign operators
    #[default]
    None,
    Eq,
    Neq,
    Like,
    Nlike,
    Lt,
    Lte,
    Gt,
    Gte,
    // supported expression sign operators - array/any operators
    AnyEq,
    AnyNeq,
    AnyLike,
    AnyNlike,
    AnyLt,
    AnyLte,
    AnyGt,
    AnyGte,
}

impl SignOp {
    pub fn from_str(str: &str) -> Option<Self> {
        match str {
            "=" => Some(Self::Eq),
            "!=" => Some(Self::Neq),
            "~" => Some(Self::Like),
            "!~" => Some(Self::Nlike),
            "<" => Some(Self::Lt),
            "<=" => Some(Self::Lte),
            ">" => Some(Self::Gt),
            ">=" => Some(Self::Gte),
            "?=" => Some(Self::AnyEq),
            "?!=" => Some(Self::AnyNeq),
            "?~" => Some(Self::AnyLike),
            "?!~" => Some(Self::AnyNlike),
            "?<" => Some(Self::AnyLt),
            "?<=" => Some(Self::AnyLte),
            "?>" => Some(Self::AnyGt),
            "?>=" => Some(Self::AnyGte),
            _ => None,
        }
    }

    fn as_str(&self) -> &str {
        match self {
            Self::None => "",
            Self::Eq => "=",
            Self::Neq => "!=",
            Self::Like => "~",
            Self::Nlike => "!~",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Gt => ">",
            Self::Gte => ">=",
            Self::AnyEq => "?=",
            Self::AnyNeq => "?!=",
            Self::AnyLike => "?~",
            Self::AnyNlike => "?!~",
            Self::AnyLt => "?<",
            Self::AnyLte => "?<=",
            Self::AnyGt => "?>",
            Self::AnyGte => "?>=",
        }
    }
}

impl std::fmt::Display for SignOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Token represents a token kind and its literal.
// Token represents a single scanned literal (one or more combined chars).
#[derive(Default, PartialEq, Clone)]
pub enum Token {
    // token kind constants
    #[default]
    None,
    Eof(String),
    Ws(String),
    Join(String),
    Sign(String),
    Identifier(String),
    Number(String),
    Text(String),
    Group(String),
    Comment(String),
}

impl Token {
    pub fn kind(&self) -> &str {
        match self {
            Self::None => "",
            Self::Eof(_) => "eof",
            Self::Ws(_) => "whitespace",
            Self::Join(_) => "join",
            Self::Sign(_) => "sign",
            Self::Identifier(_) => "identifier", // variable, column name, placeholder, etc.
            Self::Number(_) => "number",
            Self::Text(_) => "text",   // ' or " quoted string
            Self::Group(_) => "group", // groupped/nested tokens
            Self::Comment(_) => "comment",
        }
    }

    pub fn literal(&self) -> &str {
        match self {
            Self::None => "",
            Self::Eof(value) => value,
            Self::Ws(value) => value,
            Self::Join(value) => value,
            Self::Sign(value) => value,
            Self::Identifier(value) => value,
            Self::Number(value) => value,
            Self::Text(value) => value,
            Self::Group(value) => value,
            Self::Comment(value) => value,
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{} {}}}", self.kind(), self.literal())
    }
}

// Scanner represents a filter and lexical scanner.
pub struct Scanner {
    buffer: Vec<u8>,
    pos: usize,
}

impl Scanner {
    pub fn new(mut r: BufReader<impl Read>) -> Result<Self, Error> {
        let mut buffer = Vec::new();
        r.read_to_end(&mut buffer)
            .map_err(|err| Error::Buffer(err.to_string()))?;
        Ok(Scanner { buffer, pos: 0 })
    }

    pub fn scan(&mut self) -> Result<Token, Error> {
        let ch = self.read();

        if is_whitespace_char(ch) {
            self.unread();
            return self.scan_whitespace();
        }

        if is_group_start_char(ch) {
            self.unread();
            return self.scan_group();
        }

        if is_identifier_start_char(ch) {
            self.unread();
            return self.scan_identifier();
        }

        if is_number_start_char(ch) {
            self.unread();
            return self.scan_number();
        }

        if is_text_start_char(ch) {
            self.unread();
            return self.scan_text(false);
        }

        if is_sign_start_char(ch) {
            self.unread();
            return self.scan_sign();
        }

        if is_join_start_char(ch) {
            self.unread();
            return self.scan_join();
        }

        if is_comment_start_char(ch) {
            self.unread();
            return self.scan_comment();
        }

        if ch == EOF {
            return Ok(Token::Eof(ch.to_string()));
        }

        Err(Error::Unexpected(format!("Unexpected character {ch}")))
    }

    fn scan_whitespace(&mut self) -> Result<Token, Error> {
        let mut buf = bytes::Buffer::new();

        // Reads every subsequent whitespace character into the buffer.
        // Non-whitespace chars and EOF will cause the loop to exit.
        loop {
            let ch = self.read();

            if ch == EOF {
                break;
            }

            if !is_whitespace_char(ch) {
                self.unread();
                break;
            }

            // write the whitespace char
            buf.write_char(ch)?;
        }

        Ok(Token::Ws(buf.into_string()?))
    }

    // scanIdentifier consumes all contiguous ident chars.
    fn scan_identifier(&mut self) -> Result<Token, Error> {
        let mut buf = bytes::Buffer::new();

        // Read every subsequent identifier char into the buffer.
        // Non-ident chars and EOF will cause the loop to exit.
        loop {
            let ch = self.read();

            if ch == EOF {
                break;
            }

            if !is_identifier_start_char(ch) && !is_digit_char(ch) && ch != '.' && ch != ':' {
                self.unread();
                break;
            }

            // write the ident char
            buf.write_char(ch)?
        }

        let literal = buf.into_string()?;

        if !is_identifier(&literal) {
            return Err(Error::Invalid(format!("Invalid identifier {literal}")));
        }

        Ok(Token::Identifier(literal))
    }

    // scanNumber consumes all contiguous digit chars.
    fn scan_number(&mut self) -> Result<Token, Error> {
        let mut buf = bytes::Buffer::new();

        // read the number first char to skip the sign (if exist)
        buf.write_char(self.read())?;

        // Read every subsequent digit char into the buffer.
        // Non-digit chars and EOF will cause the loop to exit.
        loop {
            let ch = self.read();

            if ch == EOF {
                break;
            }

            if !is_digit_char(ch) && ch != '.' {
                self.unread();
                break;
            }

            // write the digit char
            buf.write_char(ch)?;
        }

        let literal = buf.into_string()?;

        if !is_number(&literal) {
            return Err(Error::Invalid(format!("Invalid number {literal}")));
        }
        Ok(Token::Number(literal))
    }

    // scanText consumes all contiguous quoted text chars.
    fn scan_text(&mut self, preserve_quotes: bool) -> Result<Token, Error> {
        let mut buf = bytes::Buffer::new();

        // read the first char to determine the quotes type
        let first_ch = self.read();
        buf.write_char(first_ch)?;
        let mut prev_ch = '\0';
        let mut has_matching_quotes = false;

        // Read every subsequent text char into the buffer.
        // EOF and matching unescaped ending quote will cause the loop to exit.
        loop {
            let ch = self.read();

            if ch == EOF {
                break;
            }

            // write the text char
            buf.write_char(ch)?;

            // unescaped matching quote, aka. the end
            if ch == first_ch && prev_ch != '\\' {
                has_matching_quotes = true;
                break;
            }

            prev_ch = ch;
        }

        let mut literal = buf.into_string()?;

        if !has_matching_quotes {
            return Err(Error::Invalid(format!("Invalid quoted text {literal}")));
        } else if !preserve_quotes {
            // unquote
            literal = literal[1..literal.len() - 1].to_string();
            // remove escaped quotes prefix (aka. \)
            let first_ch_str = first_ch.to_string();
            literal = literal.replace(&("\\".to_owned() + &first_ch_str), &first_ch_str);
        }

        Ok(Token::Text(literal))
    }

    // scan_sign consumes all contiguous sign operator chars.
    fn scan_sign(&mut self) -> Result<Token, Error> {
        let mut buf = bytes::Buffer::new();

        // Read every subsequent sign char into the buffer.
        // Non-sign chars and EOF will cause the loop to exit.
        loop {
            let ch = self.read();

            if ch == EOF {
                break;
            }

            if !is_sign_start_char(ch) {
                self.unread();
                break;
            }

            // write the sign char
            buf.write_char(ch)?;
        }

        let literal = buf.into_string()?;

        if !is_sign_operator(&literal) {
            return Err(Error::Invalid(format!("Invalid sign operator {literal}")));
        }

        Ok(Token::Sign(literal))
    }

    // scan_join consumes all contiguous join operator chars.
    fn scan_join(&mut self) -> Result<Token, Error> {
        let mut buf = bytes::Buffer::new();

        // Read every subsequent join operator char into the buffer.
        // Non-join chars and EOF will cause the loop to exit.
        loop {
            let ch = self.read();

            if ch == EOF {
                break;
            }

            if !is_join_start_char(ch) {
                self.unread();
                break;
            }

            // write the join operator char
            buf.write_char(ch)?;
        }

        let literal = buf.into_string()?;

        if !is_join_operator(&literal) {
            return Err(Error::Invalid(format!("Invalid join operator {literal}",)));
        }

        Ok(Token::Join(literal))
    }

    // scanGroup consumes all chars within a group/parenthesis.
    fn scan_group(&mut self) -> Result<Token, Error> {
        let mut buf = bytes::Buffer::new();

        // read the first group bracket without writing it to the buffer
        let first_char = self.read();
        let mut open_groups = 1;

        // Read every subsequent text char into the buffer.
        // EOF and matching unescaped ending quote will cause the loop to exit.
        loop {
            let ch = self.read();

            if ch == EOF {
                break;
            }

            if is_group_start_char(ch) {
                open_groups += 1;
                buf.write_char(ch)?;
            } else if is_text_start_char(ch) {
                self.unread();
                let t = self.scan_text(true)?; // with quotes to preserve the exact text start/end runes

                buf.write_string(t.literal())?
            } else if ch == ')' {
                open_groups -= 1;

                if open_groups <= 0 {
                    // main group end
                    break;
                } else {
                    buf.write_char(ch)?;
                }
            } else {
                buf.write_char(ch)?;
            }
        }

        let literal = buf.into_string()?;

        if !is_group_start_char(first_char) || open_groups > 0 {
            return Err(Error::Invalid(format!(
                "Invalid formatted group - missing {open_groups} closing bracket(s)"
            )));
        }

        Ok(Token::Group(literal))
    }

    // scan_comment consumes all contiguous single line comment chars until
    // a new character (\n) or EOF is reached.
    fn scan_comment(&mut self) -> Result<Token, Error> {
        let mut buf = bytes::Buffer::new();

        // Read the first 2 characters without writting them to the buffer.
        if !is_comment_start_char(self.read()) || !is_comment_start_char(self.read()) {
            return Err(Error::Invalid("Invalid comment".to_owned()));
        }

        // Read every subsequent comment text char into the buffer.
        // \n and EOF will cause the loop to exit.
        loop {
            let ch = self.read();

            if ch == EOF || ch == '\n' {
                break;
            }

            buf.write_char(ch)?;
        }

        let literal = buf.into_string()?;

        Ok(Token::Comment(literal.trim().to_owned()))
    }

    // read reads the next char from the buffered reader.
    // Returns the `\0` if an error occurs.
    fn read(&mut self) -> char {
        if self.pos == self.buffer.len() {
            return EOF;
        }
        let ch = char::from(self.buffer[self.pos]);
        self.pos += 1;
        ch
    }

    // unread places the previously read char back on the reader.
    fn unread(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }
}

// Lexical helpers:
// -------------------------------------------------------------------

// is_whitespace_char checks if a char is a space, tab, or newline.
fn is_whitespace_char(ch: char) -> bool {
    ch == ' ' || ch == '\t' || ch == '\n'
}

// is_letter_char checks if a char is a letter.
fn is_letter_char(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_uppercase()
}

// is_digit_char checks if a char is a digit.
fn is_digit_char(ch: char) -> bool {
    ch.is_ascii_digit()
}

// is_identifier_start_char checks if a char is valid identifier's first character.
fn is_identifier_start_char(ch: char) -> bool {
    is_letter_char(ch) || ch == '_' || ch == '@' || ch == '#'
}

// is_text_start_char checks if a char is a valid quoted text first character
// (aka. single or double quote).
fn is_text_start_char(ch: char) -> bool {
    ch == '\'' || ch == '"'
}

// is_number_start_char checks if a char is a valid number start character (aka. digit).
fn is_number_start_char(ch: char) -> bool {
    ch == '-' || is_digit_char(ch)
}

// is_sign_start_char checks if a char is a valid sign operator start character.
fn is_sign_start_char(ch: char) -> bool {
    ch == '=' || ch == '?' || ch == '!' || ch == '>' || ch == '<' || ch == '~'
}

// is_join_start_char checks if a char is a valid join type start character.
fn is_join_start_char(ch: char) -> bool {
    ch == '&' || ch == '|'
}

// is_group_start_char checks if a char is a valid group/parenthesis start character.
fn is_group_start_char(ch: char) -> bool {
    ch == '('
}

// is_comment_start_char checks if a char is a valid comment start character.
fn is_comment_start_char(ch: char) -> bool {
    ch == '/'
}

// is_sign_operator checks if a literal is a valid sign operator.
fn is_sign_operator(literal: &str) -> bool {
    SignOp::from_str(literal).is_some()
}

// is_join_operator checks if a literal is a valid join type operator.
fn is_join_operator(literal: &str) -> bool {
    JoinOp::from_str(literal).is_some()
}

// is_number checks if a literal is numeric.
fn is_number(literal: &str) -> bool {
    if literal.is_empty() || literal.ends_with('.') {
        return false;
    }

    literal.parse::<f64>().is_ok()
}

// is_identifier checks if a literal is properly formatted identifier.
fn is_identifier(literal: &str) -> bool {
    static IDENTIFIER_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^[\@\#\_]?[\w\.\:]*\w+$").unwrap());
    IDENTIFIER_REGEX.is_match(literal)
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use crate::scanner::Token;

    use super::Scanner;

    #[test]
    fn test_new_scanner() {
        let s = Scanner::new(BufReader::new("test".as_bytes())).unwrap();
        let data_bytes = &s.buffer[0..4];
        let data = std::str::from_utf8(data_bytes).unwrap();

        assert!(
            data == "test",
            "Expected the scanner reader data to be \"test\", got {data}"
        )
    }

    #[test]
    fn test_scanner_scan() {
        struct Output {
            error: bool,
            print: &'static str,
        }

        struct TestScenario {
            text: &'static str,
            expects: Vec<Output>,
        }

        let test_scenarios = vec![
            // whitespace
            TestScenario {
                text: r"   ",
                expects: vec![Output {
                    error: false,
                    print: r"{whitespace    }",
                }],
            },
            TestScenario {
                text: r"test 123",
                expects: vec![
                    Output {
                        error: false,
                        print: r"{identifier test}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{number 123}",
                    },
                ],
            },
            // identifier
            TestScenario {
                text: r"test",
                expects: vec![Output {
                    error: false,
                    print: r"{identifier test}",
                }],
            },
            TestScenario {
                text: r"@test.123",
                expects: vec![Output {
                    error: false,
                    print: r"{identifier @test.123}",
                }],
            },
            TestScenario {
                text: r"_test.123",
                expects: vec![Output {
                    error: false,
                    print: r"{identifier _test.123}",
                }],
            },
            TestScenario {
                text: r"#test.123:456",
                expects: vec![Output {
                    error: false,
                    print: r"{identifier #test.123:456}",
                }],
            },
            TestScenario {
                text: r".test.123",
                expects: vec![
                    Output {
                        error: true,
                        print: r"{unexpected .}",
                    },
                    Output {
                        error: false,
                        print: r"{identifier test.123}",
                    },
                ],
            },
            TestScenario {
                text: r":test.123",
                expects: vec![
                    Output {
                        error: true,
                        print: r"{unexpected :}",
                    },
                    Output {
                        error: false,
                        print: r"{identifier test.123}",
                    },
                ],
            },
            TestScenario {
                text: r"test#@",
                expects: vec![Output {
                    error: true,
                    print: r"{identifier test#@}",
                }],
            },
            TestScenario {
                text: r"test'",
                expects: vec![
                    Output {
                        error: false,
                        print: r"{identifier test}",
                    },
                    Output {
                        error: true,
                        print: r"{text '}",
                    },
                ],
            },
            TestScenario {
                text: r#"test"d"#,
                expects: vec![
                    Output {
                        error: false,
                        print: r"{identifier test}",
                    },
                    Output {
                        error: true,
                        print: r#"{text \"d}"#,
                    },
                ],
            },
            // number
            TestScenario {
                text: r"123",
                expects: vec![Output {
                    error: false,
                    print: r"{number 123}",
                }],
            },
            TestScenario {
                text: r"-123",
                expects: vec![Output {
                    error: false,
                    print: r"{number -123}",
                }],
            },
            TestScenario {
                text: r"-123.456",
                expects: vec![Output {
                    error: false,
                    print: r"{number -123.456}",
                }],
            },
            TestScenario {
                text: r"123.456",
                expects: vec![Output {
                    error: false,
                    print: r"{number 123.456}",
                }],
            },
            TestScenario {
                text: r".123",
                expects: vec![
                    Output {
                        error: true,
                        print: r"{unexpected .}",
                    },
                    Output {
                        error: false,
                        print: r"{number 123}",
                    },
                ],
            },
            TestScenario {
                text: r"- 123",
                expects: vec![
                    Output {
                        error: true,
                        print: r"{number -}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{number 123}",
                    },
                ],
            },
            TestScenario {
                text: r"12-3",
                expects: vec![
                    Output {
                        error: false,
                        print: r"{number 12}",
                    },
                    Output {
                        error: false,
                        print: r"{number -3}",
                    },
                ],
            },
            TestScenario {
                text: r"123.abc",
                expects: vec![
                    Output {
                        error: true,
                        print: r"{number 123.}",
                    },
                    Output {
                        error: false,
                        print: r"{identifier abc}",
                    },
                ],
            },
            // text
            TestScenario {
                text: r#""""#,
                expects: vec![Output {
                    error: false,
                    print: r"{text }",
                }],
            },
            TestScenario {
                text: r"''",
                expects: vec![Output {
                    error: false,
                    print: r"{text }",
                }],
            },
            TestScenario {
                text: r"'test'",
                expects: vec![Output {
                    error: false,
                    print: r"{text test}",
                }],
            },
            TestScenario {
                text: r"'te\'st'",
                expects: vec![Output {
                    error: false,
                    print: r"{text te'st}",
                }],
            },
            TestScenario {
                text: r#""te\"st""#,
                expects: vec![Output {
                    error: false,
                    print: r#"{text te"st}"#,
                }],
            },
            TestScenario {
                text: r#""tes@#,;!@#%^'\"t""#,
                expects: vec![Output {
                    error: false,
                    print: r#"{text tes@#,;!@#%^'"t}"#,
                }],
            },
            TestScenario {
                text: r#"'tes@#,;!@#%^\'"t'"#,
                expects: vec![Output {
                    error: false,
                    print: r#"{text tes@#,;!@#%^'"t}"#,
                }],
            },
            TestScenario {
                text: r#""test"#,
                expects: vec![Output {
                    error: true,
                    print: r#"{text "test}"#,
                }],
            },
            TestScenario {
                text: r"'test",
                expects: vec![Output {
                    error: true,
                    print: r"{text 'test}",
                }],
            },
            // join types
            TestScenario {
                text: r"&&||",
                expects: vec![Output {
                    error: true,
                    print: r"{join &&||}",
                }],
            },
            TestScenario {
                text: r"&& ||",
                expects: vec![
                    Output {
                        error: false,
                        print: r"{join &&}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{join ||}",
                    },
                ],
            },
            TestScenario {
                text: r"'||test&&'&&123",
                expects: vec![
                    Output {
                        error: false,
                        print: r"{text ||test&&}",
                    },
                    Output {
                        error: false,
                        print: r"{join &&}",
                    },
                    Output {
                        error: false,
                        print: r"{number 123}",
                    },
                ],
            },
            // expression signs
            TestScenario {
                text: r"=!=",
                expects: vec![Output {
                    error: true,
                    print: r"{sign =!=}",
                }],
            },
            TestScenario {
                text: r"= != ~ !~ > >= < <= ?= ?!= ?~ ?!~ ?> ?>= ?< ?<=",
                expects: vec![
                    Output {
                        error: false,
                        print: r"{sign =}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign !=}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign ~}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign !~}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign >}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign >=}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign <}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign <=}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign ?=}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign ?!=}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign ?~}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign ?!~}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign ?>}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign ?>=}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign ?<}",
                    },
                    Output {
                        error: false,
                        print: r"{whitespace  }",
                    },
                    Output {
                        error: false,
                        print: r"{sign ?<=}",
                    },
                ],
            },
            // groups/parenthesis
            TestScenario {
                text: r"a)",
                expects: vec![
                    Output {
                        error: false,
                        print: r"{identifier a}",
                    },
                    Output {
                        error: true,
                        print: r"{unexpected )}",
                    },
                ],
            },
            TestScenario {
                text: r"(a b c",
                expects: vec![Output {
                    error: true,
                    print: r"{group a b c}",
                }],
            },
            TestScenario {
                text: r"(a b c)",
                expects: vec![Output {
                    error: false,
                    print: r"{group a b c}",
                }],
            },
            TestScenario {
                text: r"((a b c))",
                expects: vec![Output {
                    error: false,
                    print: r"{group (a b c)}",
                }],
            },
            TestScenario {
                text: r"((a )b c))",
                expects: vec![
                    Output {
                        error: false,
                        print: r"{group (a )b c}",
                    },
                    Output {
                        error: true,
                        print: r"{unexpected )}",
                    },
                ],
            },
            TestScenario {
                text: r#"("ab)("c)"#,
                expects: vec![Output {
                    error: false,
                    print: r#"{group "ab)("c}"#,
                }],
            },
            TestScenario {
                text: r#"("ab)(c)"#,
                expects: vec![Output {
                    error: true,
                    print: r#"{group "ab)(c)}"#,
                }],
            },
            // comments
            TestScenario {
                text: r"/ test",
                expects: vec![
                    Output {
                        error: true,
                        print: r"{comment }",
                    },
                    Output {
                        error: false,
                        print: r"{identifier test}",
                    },
                ],
            },
            TestScenario {
                text: r"/ / test",
                expects: vec![
                    Output {
                        error: true,
                        print: r"{comment }",
                    },
                    Output {
                        error: true,
                        print: r"{comment }",
                    },
                    Output {
                        error: false,
                        print: r"{identifier test}",
                    },
                ],
            },
            TestScenario {
                text: r"//",
                expects: vec![Output {
                    error: false,
                    print: r"{comment }",
                }],
            },
            TestScenario {
                text: r"//test",
                expects: vec![Output {
                    error: false,
                    print: r"{comment test}",
                }],
            },
            TestScenario {
                text: r"// test",
                expects: vec![Output {
                    error: false,
                    print: r"{comment test}",
                }],
            },
            TestScenario {
                text: r"//   test1 //test2  ",
                expects: vec![Output {
                    error: false,
                    print: r"{comment test1 //test2}",
                }],
            },
            TestScenario {
                text: r"///test",
                expects: vec![Output {
                    error: false,
                    print: r"{comment /test}",
                }],
            },
        ];

        for (i, scenario) in test_scenarios.iter().enumerate() {
            let mut s = Scanner::new(BufReader::new(scenario.text.as_bytes())).unwrap();

            // scan the text tokens
            for (j, expect) in scenario.expects.iter().enumerate() {
                let token = match s.scan() {
                    Ok(token) => {
                        assert!(
                            !expect.error,
                            "({}.{}) Expected error, got ok ({})",
                            i, j, token
                        );
                        token
                    }
                    Err(err) => {
                        assert!(
                            expect.error,
                            "({}.{}) Did not expect error, got {} ({})",
                            i, j, err, scenario.text
                        );
                        continue;
                    }
                };

                let token_print = token.to_string();

                assert!(
                    token_print == expect.print,
                    "({}.{}) Expected token {}, got {}",
                    i,
                    j,
                    expect.print,
                    token_print
                );
            }

            // the last remaining token should be the eof
            let last_token = s.scan().unwrap();
            assert!(
                matches!(last_token, Token::Eof(_)),
                "({}) Expected EOF token, got {}",
                i,
                last_token
            );
        }
    }
}
