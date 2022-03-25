//! XML Pull Reader

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryInto;
use std::marker::PhantomData;
use std::rc::Rc;
use std::str::FromStr;

use xml_chars::{XmlAsciiChar, XmlChar};

use crate::parser::core::{kleene, optional, plus, raw, Plus};
use crate::parser::helper::map_error;
use crate::parser::string::{bytes, chars, lit};
use crate::parser::Parser;
use crate::reader::dtd::DocTypeDeclToken;
use crate::XmlError::{UnexpectedCharacter, UnexpectedEof};
use crate::XmlEvent::Characters;
use crate::{Attribute, Cursor, ETag, XmlDecl, XmlError, XmlEvent, PI};

pub mod dtd;
pub mod entities;

// Common

#[inline]
pub fn xml_lit<'a>(literal: &'static str) -> impl Parser<'a, Attribute = (), Error = XmlError> {
    map_error(lit(literal), move |_| XmlError::ExpectToken(literal))
}

struct TerminatedChars(&'static str);

impl<'a> Parser<'a> for TerminatedChars {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Some(pos) = cursor.rest().find(self.0) {
            let res = cursor.advance2(pos);
            if let Some(c) = res.0.chars().find(|c| !c.is_xml_char()) {
                return Err(XmlError::IllegalChar(c));
            }
            Ok(res)
        } else {
            Err(XmlError::UnexpectedEof)
        }
    }
}

fn xml_terminated<T: Fn(char) -> bool>(predicate: T, terminator: u8) -> CharTerminated<T> {
    CharTerminated {
        predicate,
        terminator,
    }
}

struct CharTerminated<T> {
    predicate: T,
    terminator: u8,
}

impl<'a, T: Fn(char) -> bool> Parser<'a> for CharTerminated<T> {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Some((pos, _)) = cursor
            .rest_bytes()
            .iter()
            .enumerate()
            .find(|(_, &c)| c == self.terminator)
        {
            let res = cursor.advance2(pos);
            if let Some(c) = res.0.chars().find(|&c| !(self.predicate)(c)) {
                return Err(XmlError::IllegalChar(c));
            }
            Ok(res)
        } else {
            Err(XmlError::UnexpectedEof)
        }
    }
}

// XML

// 2.3 Common Syntactic Constructs

/// White Space
/// S ::= (#x20 | #x9 | #xD | #xA)+
struct SToken;

impl<'a> Parser<'a> for SToken {
    type Attribute = ();
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let size = cursor
            .rest_bytes()
            .iter()
            .take_while(|c| c.is_xml_whitespace())
            .count();
        if size > 0 {
            Ok(((), cursor.advance(size)))
        } else {
            Err(XmlError::ExpectedWhitespace)
        }
    }
}

struct NameToken;

impl<'a> Parser<'a> for NameToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let mut chars = cursor.rest().char_indices();

        match chars.next() {
            Some((_, c)) if c.is_xml_name_start_char() => {}
            Some((_, c)) => return Err(XmlError::IllegalNameStartChar(c)),
            None => return Err(XmlError::UnexpectedEof),
        }

        if let Some((i, _)) = chars.find(|(_, c)| !c.is_xml_name_char()) {
            Ok(cursor.advance2(i))
        } else {
            Err(XmlError::UnexpectedEof)
        }
    }
}

struct AttValueToken;

impl<'a> Parser<'a> for AttValueToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Some(c) = cursor.next_byte(0) {
            if c == b'"' {
                let start = cursor.advance(1);
                if let Some((i, c)) = start
                    .rest_bytes()
                    .iter()
                    .enumerate()
                    .find(|(_, &c)| c == b'"')
                {
                    return Ok((start.rest().split_at(i).0, start.advance(i + 1)));
                }
                return Err(XmlError::ExpectedAttrValue);
            }
            if c == b'\'' {
                let start = cursor.advance(1);
                if let Some((i, c)) = start
                    .rest_bytes()
                    .iter()
                    .enumerate()
                    .find(|(_, &c)| c == b'\'')
                {
                    return Ok((start.rest().split_at(i).0, start.advance(i + 1)));
                }
                return Err(XmlError::ExpectedAttrValue);
            }
        }

        Err(XmlError::ExpectedAttrValue)
    }
}

struct EqLiteralToken;

impl<'a> Parser<'a> for EqLiteralToken {
    type Attribute = ();
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if cursor.next_byte(0) == Some(b'=') {
            Ok(((), cursor.advance(1)))
        } else {
            Err(XmlError::ExpectedEquals)
        }
    }
}

// 2.5 Comments

struct CommentToken;

impl<'a> Parser<'a> for CommentToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = xml_lit("<!--").parse(cursor)?;
        let (comment, cursor) = TerminatedChars("--").parse(cursor)?;
        let (_, cursor) =
            map_error(xml_lit("-->"), |_| XmlError::CommentColonColon).parse(cursor)?;

        Ok((comment, cursor))
    }
}

// 2.6 Processing Instructions

/// Processing Instruction
/// PI ::= '<?' PITarget (S (Char* - (Char* '?>' Char*)))? '?>'
/// PITarget ::= Name - (('X' | 'x') ('M' | 'm') ('L' | 'l'))
struct PIToken;

impl<'a> Parser<'a> for PIToken {
    type Attribute = PI<'a>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = xml_lit("<?").parse(cursor)?;
        let (target, cursor) = NameToken.parse(cursor)?;
        if target.eq_ignore_ascii_case("xml") {
            return Err(XmlError::InvalidPITarget);
        }
        let (maybe_data, cursor) = optional((SToken, TerminatedChars("?>"))).parse(cursor)?;
        let (_, cursor) = xml_lit("?>").parse(cursor)?;

        Ok((
            PI {
                target,
                data: maybe_data.map(|data| data.1).unwrap_or(""),
            },
            cursor,
        ))
    }
}

// 2.7 CDATA Sections

/// CDATA Section
///
/// CDSect  ::= CDStart CData CDEnd
/// CDStart ::= '<![CDATA['
/// CData   ::= (Char* - (Char* ']]>' Char*))
/// CDEnd   ::= ']]>'
struct CDataToken;

impl<'a> Parser<'a> for CDataToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = xml_lit("<![CDATA[").parse(cursor)?;
        let (chars, cursor) = TerminatedChars("]]>").parse(cursor)?;
        let (_, cursor) = xml_lit("]]>").parse(cursor)?;
        Ok((chars, cursor))
    }
}

// 2.8 Prolog and Document Type Declaration

struct XmlDeclToken;

impl<'a> Parser<'a> for XmlDeclToken {
    type Attribute = XmlDecl<'a>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = xml_lit("<?xml").parse(cursor)?;
        let (version, cursor) = VersionInfoToken.parse(cursor)?;
        let (encoding, cursor) = optional(EncodingDeclToken).parse(cursor)?;
        let (standalone, cursor) = optional(SDDeclToken).parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (_, cursor) = xml_lit("?>").parse(cursor)?;

        Ok((
            XmlDecl {
                version,
                encoding,
                standalone,
            },
            cursor,
        ))
    }
}

struct VersionInfoToken;

impl<'a> Parser<'a> for VersionInfoToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = SToken.parse(cursor)?;
        let (_, cursor) = expect_token(cursor, "version")?;
        let (_, cursor) = EqToken.parse(cursor)?;

        let c = cursor.next_byte(0);
        Ok(if c == Some(b'\'') {
            let cursor = cursor.advance(1);
            let (version, cursor) = VersionNumToken.parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\'")?;
            (version, cursor)
        } else if c == Some(b'\"') {
            let cursor = cursor.advance(1);
            let (version, cursor) = VersionNumToken.parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\"")?;
            (version, cursor)
        } else {
            return Err(XmlError::ExpectToken("' or \""));
        })
    }
}

struct EqToken;

impl<'a> Parser<'a> for EqToken {
    type Attribute = ();
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (_, cursor) = xml_lit("=").parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        Ok(((), cursor))
    }
}

struct VersionNumToken;

impl<'a> Parser<'a> for VersionNumToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        map_error(
            raw((lit("1."), chars(|c: char| c.is_ascii_digit()))),
            |_| XmlError::ExpectToken("1.[0-9]+"),
        )
        .parse(cursor)
    }
}

// 2.9 Standalone Document Declaration

struct SDDeclToken;

impl<'a> Parser<'a> for SDDeclToken {
    type Attribute = bool;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = SToken.parse(cursor)?;
        let (_, cursor) = expect_token(cursor, "standalone")?;
        let (_, cursor) = EqToken.parse(cursor)?;

        let (yes_no, cursor) = if cursor.next_byte(0) == Some(b'\'') {
            let cursor = cursor.advance(1);
            let (yes_no, cursor) = map_error(raw(plus(chars(|c: char| c != '\''))), |_| {
                XmlError::ExpectToken("'yes' | 'no'")
            })
            .parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\'")?;
            (yes_no, cursor)
        } else if cursor.next_byte(0) == Some(b'\"') {
            let cursor = cursor.advance(1);
            let (yes_no, cursor) = map_error(raw(plus(chars(|c: char| c != '\"'))), |_| {
                XmlError::ExpectToken("'yes' | 'no'")
            })
            .parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\"")?;
            (yes_no, cursor)
        } else {
            return Err(XmlError::ExpectToken("' or \""));
        };

        if yes_no == "yes" {
            Ok((true, cursor))
        } else if yes_no == "no" {
            Ok((false, cursor))
        } else {
            Err(XmlError::IllegalAttributeValue("Expected yes or no"))
        }
    }
}

// 4.1 Character and Entity References

/// Character Reference
///
/// `CharRef ::= '&#' [0-9]+ ';' | '&#x' [0-9a-fA-F]+ ';'`
struct CharRefToken;

impl<'a> Parser<'a> for CharRefToken {
    type Attribute = String;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        // TODO: char's do not accept high-surrogates und low-surrogates
        let (code, cursor) = if let Ok(((_, code, _), cursor)) = (
            xml_lit("&#"),
            map_error(bytes(|c| c.is_ascii_digit()), |_| {
                XmlError::InvalidCharacterReference("contains non-digit".to_string())
            }),
            xml_lit(";"),
        )
            .parse(cursor)
        {
            let character = u32::from_str(code)
                .map_err(|_| XmlError::InvalidCharacterReference(code.to_string()))?;
            (character, cursor)
        } else if let Ok(((_, code, _), cursor)) = (
            xml_lit("&#x"),
            map_error(bytes(|c| c.is_ascii_hexdigit()), |_| {
                XmlError::InvalidCharacterReference("contains non-hex-digit".to_string())
            }),
            xml_lit(";"),
        )
            .parse(cursor)
        {
            let character = u32::from_str_radix(code, 16)
                .map_err(|_| XmlError::InvalidCharacterReference(code.to_string()))?;
            (character, cursor)
        } else {
            return Err(XmlError::InvalidCharacterReference(String::new()));
        };

        let character: char = code
            .try_into()
            .map_err(|_| XmlError::InvalidCharacterReference(code.to_string()))?;
        if !character.is_xml_char() {
            Err(XmlError::InvalidCharacterReference(code.to_string()))
        } else {
            Ok((character.to_string(), cursor))
        }
    }
}

/// Entity Reference
///
/// `EntityRef ::= '&' Name ';'`
struct EntityRefToken;

impl<'a> Parser<'a> for EntityRefToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let ((_, name, _), cursor) = (xml_lit("&"), NameToken, xml_lit(";")).parse(cursor)?;
        Ok((name, cursor))
    }
}

// 4.3.3 Character Encoding in Entities

struct EncodingDeclToken;

impl<'a> Parser<'a> for EncodingDeclToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = SToken.parse(cursor)?;
        let (_, cursor) = expect_token(cursor, "encoding")?;
        let (_, cursor) = EqToken.parse(cursor)?;

        Ok(if cursor.next_byte(0) == Some(b'\'') {
            let cursor = cursor.advance(1);
            let (encoding, cursor) = EncNameToken.parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\'")?;
            (encoding, cursor)
        } else if cursor.next_byte(0) == Some(b'\"') {
            let cursor = cursor.advance(1);
            let (encoding, cursor) = EncNameToken.parse(cursor)?;
            let (_, cursor) = expect_token(cursor, "\"")?;
            (encoding, cursor)
        } else {
            return Err(XmlError::ExpectToken("' or \""));
        })
    }
}

struct EncNameToken;

impl<'a> Parser<'a> for EncNameToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        map_error(
            raw((
                chars(|c: char| c.is_ascii_alphabetic()),
                kleene(chars(|c: char| {
                    c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'
                })),
            )),
            |_| XmlError::ExpectToken("Encoding name: [a-zA-Z][a-zA-Z0-9._-]+"),
        )
        .parse(cursor)
    }
}

fn expect_token<'a>(cursor: Cursor<'a>, token: &'static str) -> Result<((), Cursor<'a>), XmlError> {
    if !cursor.has_next_str(token) {
        Err(XmlError::ExpectToken(token))
    } else {
        Ok(((), cursor.advance(token.len())))
    }
}

fn expect_byte(cursor: Cursor, c: u8, err: fn() -> XmlError) -> Result<Cursor, XmlError> {
    if cursor.next_byte(0) == Some(c) {
        Ok(cursor.advance(1))
    } else {
        Err(err())
    }
}

pub struct Entity {
    name: String,
    external: bool,
    text: String,
}

impl Entity {
    pub fn new(name: impl ToString, text: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            external: false,
            text: text.to_string(),
        }
    }
}

pub struct Entities {
    defined: HashMap<String, Rc<Entity>>,
}

impl Default for Entities {
    fn default() -> Self {
        let mut result = Entities {
            defined: HashMap::default(),
        };
        result.register("lt", "&#60;");
        result.register("gt", "&#62;");
        result.register("amp", "&#38;");
        result.register("apos", "&#39;");
        result.register("quot", "&#34;");
        result
    }
}

impl Entities {
    pub fn register(&mut self, name: impl ToString, value: impl ToString) {
        self.defined
            .insert(name.to_string(), Rc::new(Entity::new(name, value)));
    }

    pub fn get_ref(&self, name: &str) -> Option<&Entity> {
        self.defined.get(name).map(|rc| rc.as_ref())
    }

    pub fn get_rc(&self, name: &str) -> Option<Rc<Entity>> {
        self.defined.get(name).cloned()
    }
}

struct Subparser<'a> {
    cursor: Cursor<'a>,
    attributes: Vec<Attribute<'a>>,
    empty: bool,
    seen_root: bool,
    stack: Vec<&'a str>,
    version: Option<&'a str>,
}

impl<'a> Subparser<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self, ctx: &mut ParserContext) -> Result<Option<XmlEvent<'a>>, XmlError> {
        self.attributes.clear();
        if self.empty {
            self.empty = false;
            if let Some(name) = self.stack.pop() {
                return Ok(Some(XmlEvent::etag(name)));
            }
            unreachable!()
        }

        while let Some(c) = self.cursor.next_byte(0) {
            return match c {
                b'<' => {
                    if let Some(c) = self.cursor.next_byte(1) {
                        if c == b'/' {
                            self.cursor = self.cursor.advance(2);
                            self.parse_etag()
                        } else if c == b'?' {
                            if self.is_prolog() && self.version.is_none() {
                                // TODO: not correct
                                if self.cursor.has_next_str("<?xml") {
                                    self.parse_decl(ctx)
                                } else {
                                    self.parse_pi()
                                }
                            } else {
                                self.parse_pi()
                            }
                        } else if c == b'!' {
                            if self.cursor.has_next_str("<!--") {
                                self.parse_comment()
                            } else if self.cursor.has_next_str("<!DOCTYPE") {
                                self.parse_doctypedecl()
                            } else if self.cursor.has_next_str("<![CDATA[") {
                                self.parse_cdata()
                            } else {
                                Err(XmlError::ExpectedElementStart)
                            }
                        } else {
                            self.cursor = self.cursor.advance(1);
                            self.parse_stag()
                        }
                    } else {
                        Err(XmlError::ExpectedElementStart)
                    }
                }
                b'&' => self.parse_reference(ctx),
                _ => {
                    if self.stack.is_empty() {
                        // only white space allowed
                        if c.is_xml_whitespace() {
                            let (_, cur) = SToken.parse(self.cursor)?;
                            self.cursor = cur;
                            continue;
                        } else {
                            Err(UnexpectedCharacter(self.cursor.next_char().unwrap()))
                        }
                    } else {
                        self.parse_characters()
                    }
                }
            };
        }

        if !self.stack.is_empty() {
            Err(XmlError::OpenElementAtEof)
        } else {
            Ok(None)
        }
    }

    #[inline]
    pub fn is_prolog(&self) -> bool {
        !self.seen_root
    }

    fn is_after_root(&self) -> bool {
        self.stack.is_empty() && self.seen_root
    }

    fn parse_stag(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if self.is_after_root() {
            return Err(XmlError::ExpectedDocumentEnd);
        }

        let (name, mut cursor) = NameToken.parse(self.cursor)?;
        let mut got_whitespace = if let Ok((_, cur)) = SToken.parse(cursor) {
            cursor = cur;
            true
        } else {
            false
        };

        while let Some(c) = cursor.next_byte(0) {
            // /> empty end
            if c == b'/' {
                return if Some(b'>') == cursor.next_byte(1) {
                    self.cursor = cursor.advance(2);
                    self.empty = true;
                    self.seen_root = true;
                    self.stack.push(name);
                    Ok(Some(XmlEvent::stag(name, true)))
                } else {
                    Err(XmlError::ExpectedElementEnd)
                };
            }

            // normal end
            if c == b'>' {
                self.cursor = cursor.advance(1);
                self.empty = false;
                self.seen_root = true;
                self.stack.push(name);
                return Ok(Some(XmlEvent::stag(name, false)));
            }

            // attribute
            if !got_whitespace {
                return Err(XmlError::ExpectedWhitespace);
            }

            let (attr_name, cur) = NameToken.parse(cursor)?;
            let (_, cur) = EqToken.parse(cur)?;
            let (raw_value, cur) = AttValueToken.parse(cur)?;
            if let Ok((_, cur)) = SToken.parse(cur) {
                cursor = cur;
                got_whitespace = true;
            } else {
                cursor = cur;
                got_whitespace = false;
            }

            if self.attributes.iter().any(|attr| attr.name == attr_name) {
                return Err(XmlError::NonUniqueAttribute {
                    attribute: attr_name.to_string(),
                });
            }

            self.attributes.push(Attribute {
                name: attr_name,
                raw_value,
            });
        }

        Err(XmlError::ExpectedElementEnd)
    }

    fn parse_etag(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if self.is_after_root() {
            return Err(XmlError::ExpectedDocumentEnd);
        }

        // TODO: xml_lit(self.stack.pop()) should be faster
        let (name, cursor) = NameToken.parse(self.cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let cursor = expect_byte(cursor, b'>', || XmlError::ExpectedElementEnd)?;
        self.cursor = cursor;

        if let Some(expected_name) = self.stack.pop() {
            if expected_name == name {
                Ok(Some(XmlEvent::ETag(ETag { name })))
            } else {
                Err(XmlError::WrongETagName {
                    expected_name: expected_name.to_string(),
                })
            }
        } else {
            Err(XmlError::ETagAfterRootElement)
        }
    }

    fn parse_decl(&mut self, doc: &mut ParserContext) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (decl, cursor) = XmlDeclToken.parse(self.cursor)?;

        self.version = Some(decl.version);
        doc.standalone = decl.standalone;

        if let Some(encoding) = decl.encoding {
            if encoding != "UTF-8" {
                return Err(XmlError::UnsupportedEncoding(encoding.to_string()));
            }
        }

        self.cursor = cursor;
        Ok(Some(XmlEvent::XmlDecl(decl)))
    }

    fn parse_doctypedecl(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (decl, cursor) = DocTypeDeclToken.parse(self.cursor)?;
        self.cursor = cursor;
        Ok(Some(XmlEvent::Dtd(decl)))
    }

    fn parse_pi(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (pi, cursor) = PIToken.parse(self.cursor)?;
        self.cursor = cursor;
        Ok(Some(XmlEvent::PI(pi)))
    }

    fn parse_comment(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (comment, cursor) = CommentToken.parse(self.cursor)?;
        self.cursor = cursor;
        Ok(Some(XmlEvent::Comment(comment)))
    }

    fn parse_cdata(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (cdata, cursor) = CDataToken.parse(self.cursor)?;
        self.cursor = cursor;
        Ok(Some(XmlEvent::Characters(cdata.into())))
    }

    fn parse_characters(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if let Some((i, _)) = self
            .cursor
            .rest_bytes()
            .iter()
            .enumerate()
            .find(|(_, &c)| c == b'<' || c == b'&')
        {
            let (chars, cur) = self.cursor.advance2(i);
            self.cursor = cur;
            // TODO: ]]> not allowed
            Ok(Some(Characters(chars.into())))
        } else {
            Err(UnexpectedEof)
        }
    }

    fn parse_reference(
        &mut self,
        ctx: &mut ParserContext,
    ) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if let Some(c) = self.cursor.next_byte(1) {
            if c == b'#' {
                let (character, cursor) = CharRefToken.parse(self.cursor)?;
                self.cursor = cursor;
                Ok(Some(XmlEvent::Characters(character.into())))
            } else {
                let (entity_ref, cursor) = EntityRefToken.parse(self.cursor)?;
                if let Some(entity) = ctx.entities.get_rc(entity_ref) {
                    self.cursor = cursor;
                    todo!()
                } else {
                    Err(XmlError::UnknownEntity(entity_ref.to_string()))
                }
            }
        } else {
            Err(XmlError::IllegalReference)
        }
    }
}

pub struct ParserContext {
    standalone: Option<bool>,
    entities: Entities,
    xml_lang: Option<String>,

    next_entity: Option<Entity>,
}

/// XMl Pull Parser
pub struct Reader<'a> {
    root_parser: Subparser<'a>,
    sub_parsers: Vec<Subparser<'static>>,
    ctx: ParserContext,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            root_parser: Subparser {
                cursor: Cursor::new(input),
                attributes: Vec::with_capacity(4),
                empty: false,
                seen_root: false,
                version: None,
                stack: vec![],
            },
            sub_parsers: vec![],
            ctx: ParserContext {
                standalone: None,
                entities: Default::default(),
                xml_lang: None,

                next_entity: None,
            },
        }
    }

    fn get_current_parser(&self) -> &Subparser<'a> {
        if let Some(parser) = self.sub_parsers.last() {
            &parser
        } else {
            &self.root_parser
        }
    }

    #[inline]
    pub fn attributes(&self) -> &[Attribute<'a>] {
        &self.get_current_parser().attributes
    }

    #[inline]
    pub fn cursor_offset(&self) -> usize {
        self.get_current_parser().cursor.offset()
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let evt = if let Some(parser) = self.sub_parsers.last_mut() {
            let result = parser.next(&mut self.ctx);
            return if result == Ok(None) {
                self.sub_parsers.pop();
                self.next()
            } else {
                result
            };
        } else {
            self.root_parser.next(&mut self.ctx)
        };

        match evt {
            Ok(None) => {
                if let Some(entity) = &self.ctx.next_entity {
                    todo!()
                    // self.sub_parsers.push(Subparser {
                    //     cursor: Cursor::new(entity.text),
                    //     attributes: vec![],
                    //     empty: false,
                    //     seen_root: false,
                    //     version: None,
                    //     stack: vec![],
                    // });
                } else {
                    Ok(None)
                }
            }
            evt => evt,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::reader::Reader;
    use crate::XmlEvent;
    use crate::{Attribute, XmlError};

    macro_rules! assert_evt {
        ($exp:expr, $reader:expr) => {
            assert_eq!($exp, $reader.next(), "error at {}", $reader.cursor_offset())
        };
    }

    macro_rules! assert_evt_matches {
        ($exp:pat, $reader:expr) => {
            assert!(
                matches!($reader.next(), $exp),
                "error at {}",
                $reader.cursor_offset()
            )
        };
    }

    fn empty_array<T>() -> &'static [T] {
        &[]
    }

    mod stag {
        use crate::reader::tests::empty_array;
        use crate::reader::Reader;
        use crate::{Attribute, XmlError, XmlEvent};

        #[test]
        fn single_element() {
            let mut reader = Reader::new("<elem></elem>");
            assert_evt!(Ok(Some(XmlEvent::stag("elem", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn single_element_whitespace() {
            let mut reader = Reader::new("<elem  ></elem   >");
            assert_evt!(Ok(Some(XmlEvent::stag("elem", false))), reader);
            assert_eq!(empty_array::<Attribute>(), reader.attributes());
            assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn empty_element() {
            let mut reader = Reader::new("<elem/>");
            assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
            assert_eq!(empty_array::<Attribute>(), reader.attributes());
            assert_evt!(Ok(None), reader);
        }
    }

    mod attributes {
        use crate::reader::Reader;
        use crate::{Attribute, XmlError, XmlEvent};

        #[test]
        fn attribute() {
            let mut reader = Reader::new("<elem attr=\"value\"/>");
            assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
            assert_eq!(&[Attribute::new("attr", "value")], reader.attributes());
            assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn attribute_whitespace() {
            let mut reader = Reader::new("<elem \t \n \r attr  =  \"value\"  />");
            assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
            assert_eq!(&[Attribute::new("attr", "value")], reader.attributes());
            assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn single_quote_attribute() {
            let mut reader = Reader::new("<elem attr='value'/>");
            assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
            assert_eq!(&[Attribute::new("attr", "value")], reader.attributes());
            assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn single_quote_attribute_whitespace() {
            let mut reader = Reader::new("<elem attr  =  'value'  />");
            assert_evt!(Ok(Some(XmlEvent::stag("elem", true))), reader);
            assert_eq!(&[Attribute::new("attr", "value")], reader.attributes());
            assert_evt!(Ok(Some(XmlEvent::etag("elem"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn multiple_attributes() {
            let mut reader = Reader::new("<e a='v' b='w' />");
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_eq!(
                &[Attribute::new("a", "v"), Attribute::new("b", "w")],
                reader.attributes()
            );
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn attribute_duplicate() {
            let mut reader = Reader::new("<e a='' a='' />");
            assert_evt!(
                Err(XmlError::NonUniqueAttribute {
                    attribute: "a".to_string()
                }),
                reader
            );
        }
    }

    mod etag {
        use crate::reader::Reader;
        use crate::{XmlError, XmlEvent};

        #[test]
        fn fail_on_missing_etag() {
            let mut reader = Reader::new("<e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::OpenElementAtEof), reader);
        }

        #[test]
        fn fail_on_open_etag() {
            let mut reader = Reader::new("<e></e></e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Err(XmlError::ExpectedDocumentEnd), reader);
        }

        #[test]
        fn fail_on_wrong_etag() {
            let mut reader = Reader::new("<e></d>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::WrongETagName {
                    expected_name: "e".to_string(),
                }),
                reader
            );
        }

        #[test]
        fn fail_on_wrong_etag_in_depth_graph() {
            let mut reader = Reader::new("<a><e><e></e><e/></d></a>");
            assert_evt!(Ok(Some(XmlEvent::stag("a", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(
                Err(XmlError::WrongETagName {
                    expected_name: "e".to_string(),
                }),
                reader
            );
        }
    }

    mod top_level_content {
        use crate::reader::Reader;
        use crate::{XmlError, XmlEvent};

        #[test]
        fn only_one_top_level_element_empty() {
            let mut reader = Reader::new("<e/><e/>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Err(XmlError::ExpectedDocumentEnd), reader);
        }

        #[test]
        fn accept_whitespace_after_root() {
            let mut reader = Reader::new("<e/> \r\t\n");
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn only_one_top_level_element() {
            let mut reader = Reader::new("<e></e><e/>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Err(XmlError::ExpectedDocumentEnd), reader);
        }
    }

    mod decl {
        use crate::reader::Reader;
        use crate::{XmlError, XmlEvent};

        #[test]
        fn parse_minimal_decl() {
            let mut reader = Reader::new("<?xml version='1.0' ?><e/>");
            assert_evt!(Ok(Some(XmlEvent::decl("1.0", None, None))), reader);
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn parse_full_decl() {
            let mut reader =
                Reader::new("<?xml version='1.0' encoding='UTF-8' standalone='yes' ?><e/>");
            assert_evt!(
                Ok(Some(XmlEvent::decl("1.0", Some("UTF-8"), Some(true)))),
                reader
            );
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn parse_decl_double_qoute() {
            let mut reader =
                Reader::new("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?><e/>");
            assert_evt!(
                Ok(Some(XmlEvent::decl("1.0", Some("UTF-8"), Some(true)))),
                reader
            );
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn parse_decl_whitespace() {
            let mut reader = Reader::new(
                "<?xml version =\t'1.0' encoding\n = \r'UTF-8' standalone =  'yes'?><e/>",
            );
            assert_evt!(
                Ok(Some(XmlEvent::decl("1.0", Some("UTF-8"), Some(true)))),
                reader
            );
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }
    }

    mod characters {
        use crate::reader::Reader;
        use crate::{XmlError, XmlEvent};

        #[test]
        fn parse_chars() {
            let mut reader = Reader::new("<e>abc</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("abc"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn fail_on_chars_in_prolog() {
            let mut reader = Reader::new("abc <e/>");
            assert_evt!(Err(XmlError::UnexpectedCharacter('a')), reader);
        }

        #[test]
        fn fail_on_chars_in_epilog() {
            let mut reader = Reader::new("<e/>abc");
            assert_evt!(Ok(Some(XmlEvent::stag("e", true))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Err(XmlError::UnexpectedCharacter('a')), reader);
        }

        #[test]
        #[ignore]
        fn fail_on_cdata_section_end() {
            let mut reader = Reader::new("<e>]]></e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::CDataEndInContent), reader);
        }
    }

    mod comment {
        use crate::reader::Reader;
        use crate::{XmlDecl, XmlError, XmlEvent};

        #[test]
        fn parse_comment() {
            let mut reader = Reader::new("<!-- declarations for <head> & <body> -->");
            assert_evt!(
                Ok(Some(XmlEvent::comment(
                    " declarations for <head> & <body> "
                ))),
                reader
            );
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn parse_empty_comment() {
            let mut reader = Reader::new("<!---->");
            assert_evt!(Ok(Some(XmlEvent::comment(""))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn parse_invalid_comment() {
            let mut reader = Reader::new("<!-- B+, B, or B--->");
            assert_evt!(Err(XmlError::CommentColonColon), reader);
        }
    }

    mod pi {
        use crate::reader::Reader;
        use crate::{XmlDecl, XmlError, XmlEvent};

        #[test]
        fn parse_pi() {
            let mut reader = Reader::new("<?e?>");
            assert_evt!(Ok(Some(XmlEvent::pi("e", ""))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn parse_pi_data() {
            let mut reader = Reader::new("<?e abc=gdsfh ?>");
            assert_evt!(Ok(Some(XmlEvent::pi("e", "abc=gdsfh "))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        #[ignore]
        fn parse_pi_starting_with_xml_1() {
            let mut reader = Reader::new("<?xml-abc?>");
            assert_evt!(Ok(Some(XmlEvent::pi("xml-abc", ""))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn parse_pi_starting_with_xml_2() {
            let mut reader = Reader::new("<?xml version='1.0'?><?xml-abc?>");
            assert_evt_matches!(Ok(Some(XmlEvent::XmlDecl(_))), reader);
            assert_evt!(Ok(Some(XmlEvent::pi("xml-abc", ""))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        #[ignore]
        fn invalid_1() {
            let mut reader = Reader::new("<?e/fsdg?>");
            assert_evt!(
                Err(XmlError::IllegalName {
                    name: "e/fsdg".to_string()
                }),
                reader
            );
        }

        #[test]
        fn invalid_target_name_1() {
            let mut reader = Reader::new("<?xml version='1.0'?><?xml?>");
            assert_evt_matches!(Ok(Some(XmlEvent::XmlDecl(_))), reader);
            assert_evt!(Err(XmlError::InvalidPITarget), reader);
        }

        #[test]
        fn invalid_target_name_2() {
            let mut reader = Reader::new("<?xml version='1.0'?><?XmL?>");
            assert_evt_matches!(Ok(Some(XmlEvent::XmlDecl(_))), reader);
            assert_evt!(Err(XmlError::InvalidPITarget), reader);
        }

        #[test]
        fn missing_end() {
            let mut reader = Reader::new("<?e abc=gdsfh");
            assert_evt!(Err(XmlError::ExpectToken("?>")), reader);
        }
    }

    mod cdata {
        use crate::reader::Reader;
        use crate::{XmlDecl, XmlError, XmlEvent};

        #[test]
        fn pass1() {
            let mut reader = Reader::new("<e><![CDATA[<greeting>Hello, world!</greeting>]]></e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Ok(Some(XmlEvent::characters(
                    "<greeting>Hello, world!</greeting>"
                ))),
                reader
            );
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn pass2() {
            let mut reader = Reader::new("<e><![CDATA[]]]]></e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("]]"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn pass3() {
            let mut reader = Reader::new("<e><![CDATA[[]]]></e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("[]"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn pass4() {
            let mut reader = Reader::new("<e><![CDATA[]]></e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters(""))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn fail1() {
            let mut reader = Reader::new("<e><![CDATA[]></e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::UnexpectedEof), reader);
        }
    }

    mod char_ref {
        use crate::reader::Reader;
        use crate::{XmlDecl, XmlError, XmlEvent};

        #[test]
        fn pass_ascii_char() {
            let mut reader = Reader::new("<e>&#x20;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{20}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn pass_decimal() {
            let mut reader = Reader::new("<e>&#32;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{20}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn pass_emoji() {
            let mut reader = Reader::new("<e>&#x1F600;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{1F600}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn pass_ref_in_chars() {
            let mut reader = Reader::new("<e>test&#x20;seq</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("test"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters(" "))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("seq"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn fail_invalid_char() {
            let mut reader = Reader::new("<e>&#x0;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("0".to_string())),
                reader
            );
        }

        #[test]
        fn fail_too_big() {
            let mut reader = Reader::new("<e>&#x10000000;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("268435456".to_string())),
                reader
            );
        }

        #[test]
        fn fail_too_big_decimal() {
            let mut reader = Reader::new("<e>&#10000000;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("10000000".to_string())),
                reader
            );
        }

        #[test]
        fn fail_non_digit() {
            let mut reader = Reader::new("<e>&#xFGH;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("".to_string())),
                reader
            );
        }

        #[test]
        fn fail_non_digit_decimal() {
            let mut reader = Reader::new("<e>&#1F;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("".to_string())),
                reader
            );
        }
    }

    mod entity_replacement {
        use crate::reader::Reader;
        use crate::{XmlDecl, XmlError, XmlEvent};

        #[test]
        fn replace_lt() {
            let mut reader = Reader::new("<e>&lt;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("<"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }
    }
}
