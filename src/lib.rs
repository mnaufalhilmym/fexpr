mod bytes;
mod error;
mod parser;
mod scanner;

pub use error::Error;

pub use parser::parse;
pub use parser::ExprGroupItem;

pub use scanner::JoinOp;
pub use scanner::Scanner;
pub use scanner::SignOp;
pub use scanner::Token;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let groups = parse("user = 'venyo' && category ~ '理财'").unwrap();
        print!("{}", groups);
    }
}