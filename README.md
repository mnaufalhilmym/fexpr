[Latest Version]: https://img.shields.io/crates/v/fexpr.svg
[crates.io]: https://crates.io/crates/fexpr

fexpr
[![Latest Version]][crates.io]
================================================================================
Rewrite [Go fexpr](https://github.com/ganigeorgiev/fexpr) in Rust

**fexpr** is a filter query language parser that generates easy to work with AST structure so that you can create safely SQL, Elasticsearch, etc. queries from user input.

Or in other words, transform the string `"id > 1"` into the struct `[{&& {{identifier id} > {number 1}}}]`.

Supports parenthesis and various conditional expression operators (see [Grammar](#grammar)).

## Example usage

```
cargo add fexpr
```

```rust
fn main() {
    let result = fexpr::parse("id=123 && status='active'");
    if let Ok(result) = result {
        println!("{}", result)
    }
}

// Output:
// [{&& {{identifier id} = {number 123}}} {&& {{identifier status} = {text active}}}]
```

> Note that each parsed expression statement contains a join/union operator (`&&` or `||`) so that the result can be consumed on small chunks without having to rely on the group/nesting context.

## Grammar

**fexpr** grammar resembles the SQL `WHERE` expression syntax. It recognizes several token types (identifiers, numbers, quoted text, expression operators, whitespaces, etc.).

> You could find all supported tokens in [`scanner.rs`](src/scanner.rs).

#### Operators

- **`=`** Equal operator (eg. `a=b`)
- **`!=`** NOT Equal operator (eg. `a!=b`)
- **`>`** Greater than operator (eg. `a>b`)
- **`>=`** Greater than or equal operator (eg. `a>=b`)
- **`<`** Less than or equal operator (eg. `a<b`)
- **`<=`** Less than or equal operator (eg. `a<=b`)
- **`~`** Like/Contains operator (eg. `a~b`)
- **`!~`** NOT Like/Contains operator (eg. `a!~b`)
- **`?=`** Array/Any equal operator (eg. `a?=b`)
- **`?!=`** Array/Any NOT Equal operator (eg. `a?!=b`)
- **`?>`** Array/Any Greater than operator (eg. `a?>b`)
- **`?>=`** Array/Any Greater than or equal operator (eg. `a?>=b`)
- **`?<`** Array/Any Less than or equal operator (eg. `a?<b`)
- **`?<=`** Array/Any Less than or equal operator (eg. `a?<=b`)
- **`?~`** Array/Any Like/Contains operator (eg. `a?~b`)
- **`?!~`** Array/Any NOT Like/Contains operator (eg. `a?!~b`)
- **`&&`** AND join operator (eg. `a=b && c=d`)
- **`||`** OR join operator (eg. `a=b || c=d`)
- **`()`** Parenthesis (eg. `(a=1 && b=2) || (a=3 && b=4)`)

#### Numbers

Number tokens are any integer or decimal numbers.

_Example_: `123`, `10.50`, `-14`.

#### Identifiers

Identifier tokens are literals that start with a letter, `_`, `@` or `#` and could contain further any number of letters, digits, `.` (usually used as a separator) or `:` (usually used as modifier) characters.

_Example_: `id`, `a.b.c`, `field123`, `@request.method`, `author.name:length`.

#### Quoted text

Text tokens are any literals that are wrapped by `'` or `"` quotes.

_Example_: `'Lorem ipsum dolor 123!'`, `"escaped \"word\""`, `"mixed 'quotes' are fine"`.

#### Comments

Comment tokens are any single line text literals starting with `//`.
Similar to whitespaces, comments are ignored by `fexpr::parse()`.

_Example_: `// test`.

## Using only the scanner

The tokenizer (aka. `fexpr::Scanner`) could be used without the parser's state machine so that you can write your own custom tokens processing:

```rust
use std::io::BufReader;

fn main() {
    let s = fexpr::Scanner::new(BufReader::new("id > 123".as_bytes()));

    if let Ok(mut s) = s {
        loop {
            let t = s.scan();

            if let Err(_) = t {
                break;
            }

            if let Ok(t) = t {
                if matches!(t, fexpr::Token::Eof(_)) {
                    break;
                }

                println!("{t}")
            }
        }
    }
}

// Output:
// {identifier id}
// {whitespace  }
// {sign >}
// {whitespace  }
// {number 123}
```
