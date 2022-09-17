//! XML Pull Reader

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::marker::PhantomData;
use std::rc::Rc;
use std::str::FromStr;

use xrs_chars::{XmlAsciiChar, XmlChar};

use crate::parser::core::{kleene, optional, plus, raw, Plus};
use crate::parser::helper::map_error;
use crate::parser::string::{bytes, chars, lit};
use crate::parser::Parser;
use crate::reader::chars::is_ascii_content_char;
use crate::reader::dtd::DocTypeDeclToken;
use crate::XmlError::{UnexpectedCharacter, UnexpectedEof};
use crate::XmlEvent::Characters;
use crate::{Attribute, Cursor, ETag, XmlDecl, XmlError, XmlEvent, PI};

pub mod chars;
pub mod dtd;

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
///
///     S ::= (#x20 | #x9 | #xD | #xA)+
///
pub(crate) struct SToken;

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

pub(crate) struct NameToken;

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

pub(crate) struct AttValueToken;

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

pub(crate) struct EqLiteralToken;

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

pub(crate) struct CommentToken;

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
pub(crate) struct PIToken;

impl<'a> Parser<'a> for PIToken {
    type Attribute = (&'a str, Option<&'a str>);
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let (_, cursor) = xml_lit("<?").parse(cursor)?;
        let (target, cursor) = NameToken.parse(cursor)?;
        if target.eq_ignore_ascii_case("xml") {
            return Err(XmlError::InvalidPITarget);
        }
        let (maybe_data, cursor) = optional((SToken, TerminatedChars("?>"))).parse(cursor)?;
        let (_, cursor) = xml_lit("?>").parse(cursor)?;

        Ok(((target, maybe_data.map(|data| data.1)), cursor))
    }
}

// 2.7 CDATA Sections

/// CDATA Section
///
/// CDSect  ::= CDStart CData CDEnd
/// CDStart ::= '<![CDATA['
/// CData   ::= (Char* - (Char* ']]>' Char*))
/// CDEnd   ::= ']]>'
pub(crate) struct CDataToken;

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

pub(crate) struct XmlDeclToken;

impl<'a> Parser<'a> for XmlDeclToken {
    type Attribute = XmlDecl;
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
                version: version.to_string(),
                encoding: encoding.map(|encoding| encoding.to_string()),
                standalone,
            },
            cursor,
        ))
    }
}

pub(crate) struct VersionInfoToken;

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

pub(crate) struct EqToken;

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

pub(crate) struct VersionNumToken;

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

pub(crate) struct SDDeclToken;

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
pub(crate) struct CharRefToken;

fn take_till_ascii_char(
    cursor: Cursor,
    f: impl Fn(u8) -> bool,
) -> Result<(&str, Cursor), XmlError> {
    if let Some((i, c)) = cursor
        .rest_bytes()
        .iter()
        .copied()
        .enumerate()
        .find(|(i, c)| (f)(*c))
    {
        Ok(cursor.advance2(i))
    } else {
        Err(XmlError::UnexpectedEof)
    }
}

impl<'a> Parser<'a> for CharRefToken {
    type Attribute = char;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        if !cursor.has_next_str("&#") {
            return Err(XmlError::InvalidCharacterReference(String::new()));
        }

        let cursor = cursor.advance(2);
        let (radix, cursor) = if cursor.has_next_str("x") {
            (16, cursor.advance(1))
        } else {
            (10, cursor)
        };

        let (code, cursor) = take_till_ascii_char(cursor, |c| c == b';')?;
        u32::from_str_radix(code, radix)
            .ok()
            .and_then(|code| char::try_from(code).ok())
            .filter(|c| c.is_xml_char())
            .map(move |c| (c, cursor.advance(1)))
            .ok_or_else(|| XmlError::InvalidCharacterReference(code.to_string()))
    }
}

/// Entity Reference
///
/// `EntityRef ::= '&' Name ';'`
pub(crate) struct EntityRefToken;

impl<'a> Parser<'a> for EntityRefToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), XmlError> {
        let ((_, name, _), cursor) = (xml_lit("&"), NameToken, xml_lit(";")).parse(cursor)?;
        Ok((name, cursor))
    }
}

// 4.3.3 Character Encoding in Entities

pub(crate) struct EncodingDeclToken;

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

pub(crate) struct EncNameToken;

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
    text: Rc<str>,
}

impl Entity {
    pub fn new(name: impl ToString, text: impl Into<Rc<str>>) -> Self {
        Self {
            name: name.to_string(),
            external: false,
            text: text.into(),
        }
    }
}

pub struct Entities {
    defined: HashMap<String, Rc<Entity>>,
}

impl Default for Entities {
    fn default() -> Self {
        let mut result = Entities {
            defined: HashMap::with_capacity(5),
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
    pub fn register(&mut self, name: impl ToString, value: impl Into<Rc<str>>) {
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

trait InternalXmlParser<'a> {
    fn stack_push(&mut self, tag: &'a str);
    fn attributes_push(&mut self, name: &'a str, value: &'a str);
    fn stack_pop(&mut self) -> Option<Cow<'a, str>>;
    fn set_version(&mut self, version: String);
    fn set_empty(&mut self, v: bool);
    fn set_seen_root(&mut self);
    fn set_cursor(&mut self, cur: Cursor<'a>);

    fn exists_attribute_name(&self, name: &'a str) -> bool;
    fn get_version(&self) -> Option<&str>;
    fn is_empty(&self) -> bool;
    fn is_after_root(&self) -> bool;
    fn cursor(&self) -> Cursor<'a>;

    // Parser functions

    fn parse_stag(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if self.is_after_root() {
            return Err(XmlError::ExpectedDocumentEnd);
        }

        let (name, mut cursor) = NameToken.parse(self.cursor())?;
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
                    self.set_cursor(cursor.advance(2));
                    self.set_empty(true);
                    self.set_seen_root();
                    self.stack_push(name);
                    Ok(Some(XmlEvent::stag(name, true)))
                } else {
                    Err(XmlError::ExpectedElementEnd)
                };
            }

            // normal end
            if c == b'>' {
                self.set_cursor(cursor.advance(1));
                self.set_empty(false);
                self.set_seen_root();
                self.stack_push(name);
                return Ok(Some(XmlEvent::stag(name, false)));
            }

            // attribute
            if !got_whitespace {
                return Err(XmlError::ExpectedWhitespace);
            }

            let (attr_name, cur) = NameToken.parse(cursor)?;
            let (_, cur) = EqToken.parse(cur)?;
            let (value, cur) = AttValueToken.parse(cur)?;
            if let Ok((_, cur)) = SToken.parse(cur) {
                cursor = cur;
                got_whitespace = true;
            } else {
                cursor = cur;
                got_whitespace = false;
            }

            if self.exists_attribute_name(attr_name) {
                return Err(XmlError::NonUniqueAttribute {
                    attribute: attr_name.to_string(),
                });
            }

            self.attributes_push(attr_name, value);
        }

        Err(XmlError::ExpectedElementEnd)
    }

    fn parse_etag(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        // TODO: xml_lit(self.stack.pop()) should be faster
        let (name, cursor) = NameToken.parse(self.cursor())?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let cursor = expect_byte(cursor, b'>', || XmlError::ExpectedElementEnd)?;
        self.set_cursor(cursor);

        if let Some(expected_name) = self.stack_pop() {
            if expected_name == name {
                Ok(Some(XmlEvent::etag(name)))
            } else {
                Err(XmlError::WrongETagName {
                    expected_name: expected_name.to_string(),
                })
            }
        } else {
            Err(XmlError::ETagAfterRootElement)
        }
    }

    fn parse_decl(&mut self, doc: &mut DocumentContext) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (decl, cursor) = XmlDeclToken.parse(self.cursor())?;

        self.set_version(decl.version.to_string());
        doc.standalone = decl.standalone;

        if let Some(encoding) = &decl.encoding {
            if !encoding.eq_ignore_ascii_case("UTF-8") {
                return Err(XmlError::UnsupportedEncoding(encoding.to_string()));
            }
        }

        self.set_cursor(cursor);
        Ok(Some(XmlEvent::XmlDecl(decl)))
    }

    fn parse_doctypedecl(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (decl, cursor) = DocTypeDeclToken.parse(self.cursor())?;
        self.set_cursor(cursor);
        // TODO: add new entities
        Ok(Some(XmlEvent::Dtd(Box::new(decl))))
    }

    fn parse_pi(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (pi, cursor) = PIToken.parse(self.cursor())?;
        self.set_cursor(cursor);
        Ok(Some(XmlEvent::PI(PI {
            target: Cow::Borrowed(pi.0),
            data: pi.1.map(Cow::Borrowed),
        })))
    }

    fn parse_comment(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (comment, cursor) = CommentToken.parse(self.cursor())?;
        self.set_cursor(cursor);
        Ok(Some(XmlEvent::Comment(Cow::Borrowed(comment))))
    }

    fn parse_cdata(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let (cdata, cursor) = CDataToken.parse(self.cursor())?;
        self.set_cursor(cursor);
        Ok(Some(XmlEvent::Characters(cdata.into())))
    }

    fn parse_characters(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        if let Some((i, c)) = self
            .cursor()
            .rest()
            .char_indices()
            .find(|(_, c)| !is_ascii_content_char(*c))
        {
            if c.is_xml_char() {
                debug_assert!(i > 0);
                let (chars, cursor) = self.cursor().advance2(i);
                self.set_cursor(cursor);
                // TODO: ]]> not allowed
                Ok(Some(Characters(chars.into())))
            } else {
                Err(XmlError::InvalidCharacter(c))
            }
        } else {
            Err(UnexpectedEof)
        }
    }

    fn parse_reference(
        &mut self,
        ctx: &mut DocumentContext,
    ) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let cur = self.cursor();
        if let Some(c) = cur.next_byte(1) {
            if c == b'#' {
                let (character, cursor) = CharRefToken.parse(cur)?;
                self.set_cursor(cursor);
                Ok(Some(XmlEvent::Characters(character.to_string().into())))
            } else {
                let (entity_ref, cursor) = EntityRefToken.parse(cur)?;
                if let Some(entity) = ctx.entities.get_rc(entity_ref) {
                    self.set_cursor(cursor);
                    ctx.next_entity = Some(entity);
                    Ok(None)
                } else {
                    Err(XmlError::UnknownEntity(entity_ref.to_string()))
                }
            }
        } else {
            Err(XmlError::IllegalReference)
        }
    }

    fn parse_carriage_return(&mut self) -> Option<XmlEvent<'a>> {
        let cursor = self.cursor();
        let c = cursor.next_byte(1);
        self.set_cursor(cursor.advance(1));
        if c == Some(b'\n') {
            None
        } else {
            Some(XmlEvent::characters("\n"))
        }
    }
}

struct DocumentParser<'a> {
    cursor: Cursor<'a>,
    attributes: Vec<Attribute<'a>>,
    empty: bool,
    seen_root: bool,
    stack: Vec<&'a str>,
    version: Option<String>,
}

impl<'a> InternalXmlParser<'a> for DocumentParser<'a> {
    fn stack_push(&mut self, tag: &'a str) {
        self.stack.push(tag);
    }

    fn attributes_push(&mut self, name: &'a str, value: &'a str) {
        self.attributes.push(Attribute::new(name, value));
    }

    fn stack_pop(&mut self) -> Option<Cow<'a, str>> {
        self.stack.pop().map(|tag| tag.into())
    }

    fn set_version(&mut self, version: String) {
        self.version = Some(version);
    }

    fn set_empty(&mut self, v: bool) {
        self.empty = v;
    }

    fn set_seen_root(&mut self) {
        self.seen_root = true;
    }

    fn set_cursor(&mut self, cur: Cursor<'a>) {
        self.cursor = cur;
    }

    fn exists_attribute_name(&self, name: &'a str) -> bool {
        self.attributes.iter().any(|attr| attr.name == name)
    }

    fn get_version(&self) -> Option<&str> {
        self.version.as_ref().map(|v| v as &str)
    }

    fn is_empty(&self) -> bool {
        self.empty
    }

    fn is_after_root(&self) -> bool {
        self.stack.is_empty() && self.seen_root
    }

    fn cursor(&self) -> Cursor<'a> {
        self.cursor
    }
}

impl<'a> DocumentParser<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self, ctx: &mut DocumentContext) -> Result<Option<XmlEvent<'a>>, XmlError> {
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

                            if self.is_after_root() {
                                return Err(XmlError::ExpectedDocumentEnd);
                            }
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
                b'\r' => {
                    if let Some(evt) = self.parse_carriage_return() {
                        Ok(Some(evt))
                    } else {
                        continue;
                    }
                }
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

    pub fn attributes(&self) -> &[Attribute<'a>] {
        &self.attributes
    }

    pub fn drain_attributes(&mut self) -> Vec<Attribute<'a>> {
        self.attributes.drain(..).collect()
    }

    pub fn offset(&self) -> usize {
        self.cursor.offset()
    }
}

struct EntityParserState {
    entity: Rc<Entity>,
    state: InnerEntityParserState,
}

impl EntityParserState {
    fn new(entity: Rc<Entity>) -> Self {
        Self {
            entity,
            state: InnerEntityParserState {
                offset: 0,
                attributes: vec![],
                empty: false,
                seen_root: false,
                stack: vec![],
                version: None,
            },
        }
    }

    fn entity(&self) -> &Entity {
        &self.entity
    }

    fn attributes(&self) -> &[Attribute<'static>] {
        &self.state.attributes
    }

    fn drain_attributes(&mut self) -> Vec<Attribute<'static>> {
        self.state.attributes.drain(..).collect()
    }

    fn offset(&self) -> usize {
        self.state.offset
    }
}

struct InnerEntityParserState {
    offset: usize,
    attributes: Vec<Attribute<'static>>,
    empty: bool,
    seen_root: bool,
    stack: Vec<String>,
    version: Option<String>,
}

struct EntityParser<'a> {
    cursor: Cursor<'a>,
    state: &'a mut InnerEntityParserState,
}

impl<'a> EntityParser<'a> {
    fn new(parser: &'a mut InnerEntityParserState, entity: &'a Entity) -> Self {
        Self {
            cursor: Cursor::new(&entity.text).advance(parser.offset),
            state: parser,
        }
    }
}

impl<'a> InternalXmlParser<'a> for EntityParser<'a> {
    fn stack_push(&mut self, tag: &'a str) {
        self.state.stack.push(tag.to_string())
    }

    fn attributes_push(&mut self, name: &'a str, value: &'a str) {
        self.state
            .attributes
            .push(Attribute::new(name.to_string(), value.to_string()))
    }

    fn stack_pop(&mut self) -> Option<Cow<'a, str>> {
        self.state.stack.pop().map(|tag| tag.into())
    }

    fn set_version(&mut self, version: String) {
        self.state.version = Some(version);
    }

    fn set_empty(&mut self, v: bool) {
        self.state.empty = v;
    }

    fn set_seen_root(&mut self) {
        self.state.seen_root = true;
    }

    fn set_cursor(&mut self, cur: Cursor<'a>) {
        self.cursor = cur;
        self.state.offset = cur.offset();
    }

    fn exists_attribute_name(&self, name: &str) -> bool {
        self.state.attributes.iter().any(|attr| attr.name == name)
    }

    fn get_version(&self) -> Option<&str> {
        self.state.version.as_ref().map(|s| s as &str)
    }

    fn is_empty(&self) -> bool {
        self.state.empty
    }

    fn is_after_root(&self) -> bool {
        self.state.stack.is_empty() && self.state.seen_root
    }

    fn cursor(&self) -> Cursor<'a> {
        self.cursor
    }
}

impl<'a> EntityParser<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self, ctx: &mut DocumentContext) -> Result<Option<XmlEvent<'a>>, XmlError> {
        self.state.attributes.clear();
        if self.state.empty {
            self.state.empty = false;
            if let Some(name) = self.state.stack.pop() {
                return Ok(Some(XmlEvent::etag(name)));
            }
            unreachable!()
        }

        let cursor = self.cursor();
        while let Some(c) = cursor.next_byte(0) {
            return match c {
                b'<' => {
                    if let Some(c) = cursor.next_byte(1) {
                        if c == b'/' {
                            self.set_cursor(cursor.advance(2));
                            self.parse_etag()
                        } else if c == b'?' {
                            if self.is_prolog() && self.state.version.is_none() {
                                // TODO: not correct
                                if cursor.has_next_str("<?xml") {
                                    self.parse_decl(ctx)
                                } else {
                                    self.parse_pi()
                                }
                            } else {
                                self.parse_pi()
                            }
                        } else if c == b'!' {
                            if cursor.has_next_str("<!--") {
                                self.parse_comment()
                            } else if cursor.has_next_str("<!DOCTYPE") {
                                self.parse_doctypedecl()
                            } else if cursor.has_next_str("<![CDATA[") {
                                self.parse_cdata()
                            } else {
                                Err(XmlError::ExpectedElementStart)
                            }
                        } else {
                            self.set_cursor(cursor.advance(1));
                            self.parse_stag()
                        }
                    } else {
                        Err(XmlError::ExpectedElementStart)
                    }
                }
                b'&' => self.parse_reference(ctx),
                b'\r' => {
                    if let Some(evt) = self.parse_carriage_return() {
                        Ok(Some(evt))
                    } else {
                        continue;
                    }
                }
                _ => {
                    if self.state.stack.is_empty() {
                        // only white space allowed
                        if c.is_xml_whitespace() {
                            let (_, cur) = SToken.parse(cursor)?;
                            self.set_cursor(cur);
                            continue;
                        } else {
                            Err(UnexpectedCharacter(cursor.next_char().unwrap()))
                        }
                    } else {
                        self.parse_characters()
                    }
                }
            };
        }

        if !self.state.stack.is_empty() {
            Err(XmlError::OpenElementAtEof)
        } else {
            Ok(None)
        }
    }

    #[inline]
    pub fn is_prolog(&self) -> bool {
        !self.state.seen_root
    }
}

struct DocumentContext {
    standalone: Option<bool>,
    version: Option<String>,
    entities: Entities,
    next_entity: Option<Rc<Entity>>,
}

/// XMl Pull Parser
pub struct Reader<'a> {
    root_parser: DocumentParser<'a>,
    sub_parsers: Vec<EntityParserState>,
    ctx: DocumentContext,
}

impl<'a> Reader<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            root_parser: DocumentParser {
                cursor: Cursor::new(input),
                attributes: Vec::with_capacity(4),
                empty: false,
                seen_root: false,
                version: None,
                stack: vec![],
            },
            sub_parsers: vec![],
            ctx: DocumentContext {
                standalone: None,
                version: None,
                entities: Default::default(),
                next_entity: None,
            },
        }
    }

    pub fn top_name(&self) -> Option<&str> {
        if let Some(parser) = self.sub_parsers.last() {
            if let Some(e) = parser.state.stack.last() {
                return Some(e as &str);
            }
        }

        self.root_parser.stack.last().copied()
    }

    pub fn top_name_cow(&self) -> Option<Cow<'a, str>> {
        if let Some(parser) = self.sub_parsers.last() {
            if let Some(e) = parser.state.stack.last() {
                return Some(Cow::Owned(e.clone()));
            }
        }

        self.root_parser
            .stack
            .last()
            .map(|name| Cow::Borrowed(*name))
    }

    pub fn attributes(&self) -> &[Attribute<'a>] {
        if let Some(parser) = self.sub_parsers.last() {
            parser.attributes()
        } else {
            self.root_parser.attributes()
        }
    }

    pub fn drain_attributes(&mut self) -> Vec<Attribute<'a>> {
        if let Some(parser) = self.sub_parsers.last_mut() {
            parser.drain_attributes()
        } else {
            self.root_parser.drain_attributes()
        }
    }

    pub fn unparsed(&self) -> &str {
        if let Some(parser) = self.sub_parsers.last() {
            &parser.entity.text[parser.state.offset..]
        } else {
            self.root_parser.cursor.rest()
        }
    }

    pub fn cursor_offset(&self) -> usize {
        if let Some(parser) = self.sub_parsers.last() {
            parser.offset()
        } else {
            self.root_parser.offset()
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<XmlEvent<'a>>, XmlError> {
        let evt = if let Some(parser) = self.sub_parsers.last_mut() {
            let mut tmp_parser = EntityParser::new(&mut parser.state, &parser.entity);
            let result = tmp_parser.next(&mut self.ctx);
            return if result == Ok(None) {
                self.sub_parsers.pop();
                self.next()
            } else {
                result.map(|evt| evt.map(|evt| evt.into_owned()))
            };
        } else {
            self.root_parser.next(&mut self.ctx)
        };

        match evt {
            Ok(None) => {
                if let Some(entity) = self.ctx.next_entity.take() {
                    self.sub_parsers.push(EntityParserState::new(entity));
                    self.next()
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
        ($exp:expr, $reader:expr) => {{
            let evt = $reader.next();
            assert_eq!($exp, evt, "error at {}", $reader.cursor_offset())
        }};
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
            assert_evt!(Err(XmlError::IllegalCDataSectionEnd), reader);
        }

        #[test]
        fn valid_content() {
            let mut reader =
                Reader::new("<e>\u{9}\u{A}\u{20}\u{D7FF}\u{E000}\u{FFFD}\u{10000}\u{10FFFF}</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Ok(Some(XmlEvent::characters(
                    "\u{9}\u{A}\u{20}\u{D7FF}\u{E000}\u{FFFD}\u{10000}\u{10FFFF}"
                ))),
                reader
            );
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn valid_content_entities() {
            let mut reader = Reader::new(
                "<e>&#x9;&#xA;&#xD;&#x20;&#xD7FF;&#xE000;&#xFFFD;&#x10000;&#x10FFFF;</e>",
            );
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{9}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{A}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{D}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{20}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{D7FF}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{E000}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{FFFD}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{10000}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\u{10FFFF}"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn invalid_content1() {
            let mut reader = Reader::new("<e>\u{1}</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{1}')), reader);
        }

        #[test]
        fn invalid_content2() {
            let mut reader = Reader::new("<e>&#1;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("1".to_string())),
                reader
            );
        }

        #[test]
        fn invalid_content3() {
            let mut reader = Reader::new("<e>\u{8}</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{8}')), reader);
        }

        #[test]
        fn invalid_content4() {
            let mut reader = Reader::new("<e>\u{B}</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{B}')), reader);
        }

        #[test]
        fn invalid_content5() {
            let mut reader = Reader::new("<e>\u{1F}</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{1F}')), reader);
        }

        #[test]
        fn invalid_content6() {
            let mut reader = Reader::new("<e>&#1F;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("1F".to_string())),
                reader
            );
        }

        #[test]
        fn invalid_content8() {
            let mut reader = Reader::new("<e>&#D800;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("D800".to_string())),
                reader
            );
        }

        #[test]
        fn invalid_content10() {
            let mut reader = Reader::new("<e>&#DFFF;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("DFFF".to_string())),
                reader
            );
        }

        #[test]
        fn invalid_content11() {
            let mut reader = Reader::new("<e>\u{FFFE}</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{FFFE}')), reader);
        }

        #[test]
        fn invalid_content12() {
            let mut reader = Reader::new("<e>&#FFFE;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("FFFE".to_string())),
                reader
            );
        }

        #[test]
        fn invalid_content13() {
            let mut reader = Reader::new("<e>\u{FFFF}</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::InvalidCharacter('\u{FFFF}')), reader);
        }

        #[test]
        fn invalid_content14() {
            let mut reader = Reader::new("<e>&#xFFFF;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("FFFF".to_string())),
                reader
            );
        }

        #[test]
        fn invalid_content15() {
            let mut reader = Reader::new("<e>&#x110000;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("110000".to_string())),
                reader
            );
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
        use std::borrow::Cow;

        use crate::reader::Reader;
        use crate::{XmlDecl, XmlError, XmlEvent};

        #[test]
        fn parse_pi() {
            let mut reader = Reader::new("<?e?>");
            assert_evt!(Ok(Some(XmlEvent::pi("e", None))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn parse_pi_data() {
            let mut reader = Reader::new("<?e abc=gdsfh ?>");
            assert_evt!(
                Ok(Some(XmlEvent::pi("e", Some(Cow::Borrowed("abc=gdsfh "))))),
                reader
            );
            assert_evt!(Ok(None), reader);
        }

        #[test]
        #[ignore]
        fn parse_pi_starting_with_xml_1() {
            let mut reader = Reader::new("<?xml-abc?>");
            assert_evt!(Ok(Some(XmlEvent::pi("xml-abc", None))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn parse_pi_starting_with_xml_2() {
            let mut reader = Reader::new("<?xml version='1.0'?><?xml-abc?>");
            assert_evt_matches!(Ok(Some(XmlEvent::XmlDecl(_))), reader);
            assert_evt!(Ok(Some(XmlEvent::pi("xml-abc", None))), reader);
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
            assert_evt!(Ok(Some(XmlEvent::characters("\u{20}"))), reader);
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
                Err(XmlError::InvalidCharacterReference("10000000".to_string())),
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
                Err(XmlError::InvalidCharacterReference("FGH".to_string())),
                reader
            );
        }

        #[test]
        fn fail_non_digit_decimal() {
            let mut reader = Reader::new("<e>&#1F;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(
                Err(XmlError::InvalidCharacterReference("1F".to_string())),
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

        #[test]
        fn replace_gt() {
            let mut reader = Reader::new("<e>&gt;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters(">"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn replace_amp() {
            let mut reader = Reader::new("<e>&amp;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("&"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn replace_apos() {
            let mut reader = Reader::new("<e>&apos;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("'"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn replace_quot() {
            let mut reader = Reader::new("<e>&quot;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\""))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn fail_on_open() {
            let mut reader = Reader::new("<e>&quot</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::ExpectToken(";")), reader);
        }

        #[test]
        fn fail_on_unknown_entity() {
            let mut reader = Reader::new("<e>&nent;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::UnknownEntity("nent".to_string())), reader);
        }

        #[test]
        fn fail_on_open2() {
            let mut reader = Reader::new("<e>&lt&gt;</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Err(XmlError::ExpectToken(";")), reader);
        }
    }

    /// 4.3.3 Character Encoding in Entities
    mod encoding {
        use crate::reader::Reader;
        use crate::{XmlDecl, XmlError, XmlEvent};

        #[test]
        fn utf8_lower() {
            let mut reader = Reader::new("<?xml version='1.0' encoding='utf-8'?><e/>");
            assert_evt!(
                Ok(Some(XmlEvent::decl("1.0", Option::Some("utf-8"), None))),
                reader
            );
        }

        #[test]
        fn utf8_upper() {
            let mut reader = Reader::new("<?xml version='1.0' encoding='UTF-8'?><e/>");
            assert_evt!(
                Ok(Some(XmlEvent::decl("1.0", Option::Some("UTF-8"), None))),
                reader
            );
        }

        #[test]
        fn unsupported() {
            let mut reader = Reader::new("<?xml version='1.0' encoding='UTF128'?><e/>");
            assert_evt!(
                Err(XmlError::UnsupportedEncoding("UTF128".to_string())),
                reader
            );
        }
    }

    /// 2.11 End-of-Line Handling
    mod end_of_line_handling {
        use crate::reader::Reader;
        use crate::{XmlDecl, XmlError, XmlEvent};

        #[test]
        fn passthrough_line_feed() {
            let mut reader = Reader::new("<e>a\nb</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("a\nb"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn convert_carriage_return() {
            let mut reader = Reader::new("<e>a\rb</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("a"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\n"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("b"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn ignore_carriage_return_before_line_feed() {
            let mut reader = Reader::new("<e>a\r\nb</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("a"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\nb"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn ignore_carriage_return_before_line_feed2() {
            let mut reader = Reader::new("<e>a\r\n</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("a"))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\n"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }

        #[test]
        fn ignore_carriage_return_before_line_feed3() {
            let mut reader = Reader::new("<e>\r\n</e>");
            assert_evt!(Ok(Some(XmlEvent::stag("e", false))), reader);
            assert_evt!(Ok(Some(XmlEvent::characters("\n"))), reader);
            assert_evt!(Ok(Some(XmlEvent::etag("e"))), reader);
            assert_evt!(Ok(None), reader);
        }
    }
}
