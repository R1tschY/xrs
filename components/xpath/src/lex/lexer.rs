use std::str::FromStr;

use xml_chars::{XmlAsciiChar, XmlChar};

use crate::lex::token::{Token, Tokens};
use crate::lex::{Ident, LexError, Literal, Number, Punct, Spacing};

use super::lex_cursor::LexCursor;

struct Reject;

impl FromStr for Tokens {
    type Err = LexError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        lex_expr(LexCursor::for_str(input))
    }
}

fn lex_expr(mut input: LexCursor) -> Result<Tokens, LexError> {
    let mut tokens: Vec<Token> = vec![];

    while !input.eof() {
        input = skip_whitespace(&input);

        if let Ok((cursor, punct)) = lex_number(&input) {
            tokens.push(Token::Number(punct));
            input = cursor;
        } else if let Ok((cursor, punct)) = lex_punct(&input) {
            tokens.push(Token::Punct(punct));
            input = cursor;
        } else if let Ok((cursor, ident)) = lex_ncname(&input) {
            tokens.push(Token::Ident(ident));
            input = cursor;
        } else if let Ok((cursor, literal)) = lex_literal(&input) {
            tokens.push(Token::Literal(literal));
            input = cursor;
        } else if input.eof() {
            break;
        } else {
            panic!("error: {:?}", input);
        }
    }

    Ok(Tokens::new(tokens))
}

fn skip_whitespace<'a>(input: &LexCursor<'a>) -> LexCursor<'a> {
    let len = input
        .bytes()
        .take_while(|&ch| ch.is_xml_whitespace())
        .count();
    if len > 0 {
        input.advance(len)
    } else {
        *input
    }
}

fn lex_punct<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Punct), Reject> {
    if let Some(ch) = input.next_byte() {
        if ch.is_xml_punct() {
            let cursor = input.advance(1);
            let spacing = if cursor.next_byte().filter(|&ch| ch.is_xml_punct()).is_some() {
                Spacing::Joined
            } else {
                Spacing::Alone
            };
            return Ok((cursor, Punct::new(ch as char, spacing, input.span(1))));
        }
    }

    Err(Reject)
}

fn lex_ncname<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Ident), Reject> {
    let mut buffer = String::new();
    let mut chars = input.chars();

    match chars.next() {
        Some(ch) if ch != ':' && ch.is_xml_name_start_char() => {
            buffer.push(ch);
        }
        _ => return Err(Reject),
    }

    for ch in chars {
        if ch == ':' || !ch.is_xml_name_char() {
            break;
        } else {
            buffer.push(ch);
        }
    }

    let len = buffer.as_bytes().len();
    Ok((input.advance(len), Ident::new(buffer, input.span(len))))
}

fn lex_literal<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Literal), Reject> {
    let mut chars = input.chars();

    let literal: String = match chars.next() {
        Some('\'') => chars.take_while(|&ch| ch != '\'').collect(),
        Some('\"') => chars.take_while(|&ch| ch != '\"').collect(),
        _ => return Err(Reject),
    };

    let len = literal.as_bytes().len() + 2;
    Ok((
        input.advance(len),
        Literal {
            value: literal,
            span: input.span(len),
        },
    ))
}

fn lex_number<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Number), Reject> {
    let mut chars = input.bytes();

    let mut pre: usize = 0;
    let mut mid: Option<u8> = None;
    for ch in &mut chars {
        if ch.is_ascii_digit() {
            pre += 1;
        } else {
            mid = Some(ch);
            break;
        }
    }

    let mut dot: usize = 0;
    let mut suf: usize = 0;
    if let Some(b'.') = mid {
        dot += 1;
        for ch in &mut chars {
            if ch.is_ascii_digit() {
                suf += 1;
            } else {
                break;
            }
        }
    }

    if pre == 0 && suf == 0 {
        return Err(Reject);
    }

    let len = pre + dot + suf;
    let (span, cursor) = input.advance_to(len);

    let value: f64 = if pre == 0 {
        // .000 syntax
        format!("0{}", span).parse().unwrap()
    } else {
        span.parse().unwrap()
    };

    Ok((
        cursor,
        Number {
            value,
            span: input.span(len),
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::lex::lex_cursor::LexCursor;
    use crate::lex::Tokens;

    use super::*;

    #[test]
    fn it_works() {
        let test = "array:append([],\"3\",.0,1.0)";
        let tokens = lex_expr(LexCursor::for_str(test)).unwrap();
        println!("{:?}", tokens);
    }

    fn assert_tokens(actual: Tokens, expected: &[&'static str]) {
        let mut iter = actual.into_iter();
        for expected_token in expected {
            let token = iter.next().map(|x| x.to_string());
            assert_eq!(Some(*expected_token), token.as_ref().map(|x| x as &str));
        }
        let token = iter.next().map(|x| x.to_string());
        assert_eq!(None, token);
    }

    #[test]
    fn tokenize_ident_1() {
        assert_tokens("azAZ190_".parse().unwrap(), &["azAZ190_"]);
    }

    #[test]
    fn tokenize_ident_underline() {
        assert_tokens("_1".parse().unwrap(), &["_1"]);
    }

    #[test]
    fn tokenize_number_full() {
        assert_tokens("1.2".parse().unwrap(), &["1.2"]);
    }

    #[test]
    fn tokenize_integer() {
        assert_tokens("123456789".parse().unwrap(), &["123456789"]);
    }

    #[test]
    fn tokenize_number_short() {
        assert_tokens(".123456789".parse().unwrap(), &["0.123456789"]);
    }

    #[test]
    fn tokenize_literal_single_quote() {
        assert_tokens("'abc'".parse().unwrap(), &["\"abc\""]);
    }

    #[test]
    fn tokenize_literal_double_quote() {
        assert_tokens("\"abc\"".parse().unwrap(), &["\"abc\""]);
    }

    #[test]
    fn tokenize_punct() {
        assert_tokens(
            ".,()[]@:".parse().unwrap(),
            &[".", ",", "(", ")", "[", "]", "@", ":"],
        );
    }

    #[test]
    fn tokenize_function() {
        assert_tokens(
            "array:append([],\"3\")".parse().unwrap(),
            &["array", ":", "append", "(", "[", "]", ",", "\"3\"", ")"],
        );
    }

    #[test]
    fn tokenize_skip_whitespace() {
        assert_tokens(
            " \t1\n\r + 2\t\t\t-6 \r\n  ".parse().unwrap(),
            &["1", "+", "2", "-", "6"],
        );
    }
}
