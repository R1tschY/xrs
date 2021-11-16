use crate::{
    characters, Ident, LexCursor, LexError, Literal, Number, Punct, Reject, Spacing, Span, Token,
    Tokens,
};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::str::FromStr;

pub fn lex_expr(mut input: LexCursor) -> Result<Tokens, LexError> {
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

    Ok(Tokens { tokens })
}

fn skip_whitespace<'a>(input: &LexCursor<'a>) -> LexCursor<'a> {
    let len = input
        .bytes()
        .take_while(|&ch| characters::is_whitespace_byte(ch))
        .count();
    if len > 0 {
        input.advance(len)
    } else {
        input.clone()
    }
}

fn lex_punct<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Punct), Reject> {
    if let Some(ch) = input.next_byte() {
        if characters::is_punct_char(ch) {
            let cursor = input.advance(1);
            let spacing = if cursor
                .next_byte()
                .filter(|&ch| characters::is_punct_char(ch))
                .is_some()
            {
                Spacing::Joined
            } else {
                Spacing::Alone
            };
            return Ok((
                cursor,
                Punct::new(
                    ch as char,
                    spacing,
                    Span::new(input.offset, input.offset + 1),
                ),
            ));
        }
    }

    Err(Reject)
}

fn lex_ncname<'a>(input: &LexCursor<'a>) -> Result<(LexCursor<'a>, Ident), Reject> {
    let mut buffer = String::new();
    let mut chars = input.chars();

    match chars.next() {
        Some(ch) if ch != ':' && characters::is_name_start_char(ch) => {
            buffer.push(ch);
        }
        _ => return Err(Reject),
    }

    for ch in chars {
        if ch == ':' || !characters::is_name_continue_char(ch) {
            break;
        } else {
            buffer.push(ch);
        }
    }

    let len = buffer.as_bytes().len();
    Ok((
        input.advance(len),
        Ident {
            sym: buffer,
            span: Span::new(input.offset, input.offset + len),
        },
    ))
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
            span: Span::new(input.offset, input.offset + len),
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
            span: Span::new(input.offset, input.offset + len),
        },
    ))
}
