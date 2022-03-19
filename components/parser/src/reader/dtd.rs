use xml_chars::{XmlAsciiChar, XmlChar};

use crate::dtd::{DocTypeDecl, ExternalId, IntSubset, MarkupDeclEntry};
use crate::parser::core::{kleene, optional, Kleene, Optional};
use crate::parser::helper::map_error;
use crate::parser::string::lit;
use crate::parser::Parser;
use crate::reader::{xml_lit, xml_terminated, CharTerminated, NameToken, SToken, TerminatedChars};
use crate::{Cursor, XmlError};

// 2.3 Common Syntactic Constructs
// Literals

/// External identifier literal
/// `SystemLiteral ::= ('"' [^"]* '"') | ("'" [^']* "'")`
pub struct SystemLiteralToken;

impl<'a> Parser<'a> for SystemLiteralToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (quote_char, cursor) = if let Ok((_, cursor)) = xml_lit("\"").parse(cursor) {
            (b'\"', cursor)
        } else if let Ok((_, cursor)) = xml_lit("\'").parse(cursor) {
            (b'\'', cursor)
        } else {
            return Err(XmlError::ExpectToken("SystemLiteral"));
        };

        if let Some((pos, _)) = cursor
            .rest_bytes()
            .iter()
            .enumerate()
            .find(|(_, &c)| c == quote_char)
        {
            let (res, cursor) = cursor.advance2(pos);
            Ok((res, cursor.advance(1)))
        } else {
            Err(XmlError::UnexpectedEof)
        }
    }
}

/// Identifier literal
/// `PubidLiteral ::= '"' PubidChar* '"' | "'" (PubidChar - "'")* "'"`
pub struct PubidLiteralToken;

impl<'a> Parser<'a> for PubidLiteralToken {
    type Attribute = &'a str;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (quote_char, cursor) = if let Ok((_, cursor)) = xml_lit("\"").parse(cursor) {
            (b'\"', cursor)
        } else if let Ok((_, cursor)) = xml_lit("\'").parse(cursor) {
            (b'\'', cursor)
        } else {
            return Err(XmlError::ExpectToken("PubidLiteral"));
        };

        if let Some((pos, _)) = cursor
            .rest_bytes()
            .iter()
            .enumerate()
            .find(|(_, &c)| c == quote_char)
        {
            let (res, cursor) = cursor.advance2(pos);
            if let Some(c) = res.chars().find(|&c| !c.is_xml_pubid_char()) {
                return Err(XmlError::IllegalChar(c));
            }
            Ok((res, cursor.advance(1)))
        } else {
            Err(XmlError::UnexpectedEof)
        }
    }
}

// 2.8 Prolog and Document Type Declaration
// Document Type Declaration

/// doctypedecl ::= '<!DOCTYPE' S Name (S ExternalID)? S? ('\[' intSubset '\]' S?)? '>'
pub struct DocTypeDeclToken;

impl<'a> Parser<'a> for DocTypeDeclToken {
    type Attribute = DocTypeDecl<'a>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = xml_lit("<!DOCTYPE").parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (name, cursor) = NameToken.parse(cursor)?;

        let (external_id, cursor) = optional((SToken, ExternalIdToken)).parse(cursor)?;
        let external_id = external_id.map(|v| v.1);
        let (_, cursor) = optional(SToken).parse(cursor)?;
        // let (_, cursor) = optional((xml_lit("["), IntSubsetToken, xml_lit("]"), optional(SToken)))
        //     .parse(cursor)?;
        let (_, cursor) = xml_lit(">").parse(cursor)?;

        Ok((
            DocTypeDecl::new(name, external_id, Some(IntSubset::new(vec![]))),
            cursor,
        ))
    }
}

/// DeclSep ::= PEReference | S
/*pub struct DeclSepToken;

impl<'a> Parser<'a> for DeclSepToken {
    type Attribute = DocTypeDecl<'a>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        todo!()
        // if let Ok((_, cursor)) = PeReferenceToken.parse(cursor) {
        //     return Ok((DtdTypeDecl::new(name), cursor));
        // } else if let Ok((_, cursor)) = SToken.parse(cursor) {
        //     return Ok((DtdTypeDecl::new(name), cursor));
        // } else {
        //     return Err(XmlError::ExpectToken("DeclSep"));
        // }
    }
}
*/
// intSubset ::= (markupdecl | DeclSep)*
// https://www.w3.org/TR/REC-xml/#NT-intSubset

pub struct IntSubsetToken;

impl<'a> Parser<'a> for IntSubsetToken {
    type Attribute = IntSubset<'a>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        kleene(MarkupDeclToken)
            .parse(cursor)
            .map(|(decls, cursor)| (IntSubset::new(decls), cursor))
    }
}

/// Parser for Markup Declaration or Declaration Seperator
///
/// markupdecl ::= elementdecl | AttlistDecl | EntityDecl | NotationDecl | PI | Comment
/// DeclSep ::= PEReference | S
pub struct MarkupDeclToken;

impl<'a> Parser<'a> for MarkupDeclToken {
    type Attribute = MarkupDeclEntry<'a>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = optional(SToken).parse(cursor)?;

        todo!()
        // return if let Ok((pe_ref, cursor)) = PeReferenceToken.parse(cursor) {
        //     Ok((MarkupDeclEntry::PEReference(pe_ref), cursor))
        // } else if let Ok((element, cursor)) = ElementDeclToken.parse(cursor) {
        //     Ok((MarkupDeclEntry::Element(element), cursor))
        // } else if let Ok((att_list, cursor)) = AttListDeclToken.parse(cursor) {
        //     Ok((MarkupDeclEntry::AttList(att_list), cursor))
        // } else if let Ok((entity, cursor)) = EntityDeclToken.parse(cursor) {
        //     Ok((MarkupDeclEntry::Entity(entity), cursor))
        // } else if let Ok((notation, cursor)) = NotationDeclToken.parse(cursor) {
        //     Ok((MarkupDeclEntry::Notation(notation), cursor))
        // } else if let Ok((pi, cursor)) = PIToken.parse(cursor) {
        //     Ok((MarkupDeclEntry::PI(pi), cursor))
        // } else if let Ok((comment, cursor)) = CommentToken.parse(cursor) {
        //     Ok((MarkupDeclEntry::Comment(comment), cursor))
        // } else {
        //     Err(XmlError::ExpectToken("markupdecl or DeclSep"))
        // };
    }
}

// 3.2 Element Type Declarations

// elementdecl	   ::=   	'<!ELEMENT' S Name S contentspec S? '>'
// contentspec	   ::=   	'EMPTY' | 'ANY' | Mixed | children

// examples
// <!ELEMENT br EMPTY>
// <!ELEMENT p (#PCDATA|emph)* >
// <!ELEMENT %name.para; %content.para; >
// <!ELEMENT container ANY>

// 3.2.1 Element Content

// children	   ::=   	(choice | seq) ('?' | '*' | '+')?
// cp   	   ::=   	(Name | choice | seq) ('?' | '*' | '+')?
// choice	   ::=   	'(' S? cp ( S? '|' S? cp )+ S? ')'
// seq  	   ::=   	'(' S? cp ( S? ',' S? cp )* S? ')'

// examples
// <!ELEMENT spec (front, body, back?)>
// <!ELEMENT div1 (head, (p | list | note)*, div2*)>
// <!ELEMENT dictionary-body (%div.mix; | %dict.mix;)*>

// 3.2.2 Mixed Content

// 	Mixed	   ::=   	'(' S? '#PCDATA' (S? '|' S? Name)* S? ')*' | '(' S? '#PCDATA' S? ')'

// examples
// <!ELEMENT p (#PCDATA|a|ul|b|i|em)*>
// <!ELEMENT p (#PCDATA | %font; | %phrase; | %special; | %form;)* >
// <!ELEMENT b (#PCDATA)>

// 4.1 Character and Entity References

// PEReference	   ::=   	'%' Name ';'

// 4.2.2 External Entities

/// ExternalID ::= 'SYSTEM' S SystemLiteral | 'PUBLIC' S PubidLiteral S SystemLiteral
pub struct ExternalIdToken;

impl<'a> Parser<'a> for ExternalIdToken {
    type Attribute = ExternalId<'a>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (pub_id, cursor) = if let Ok((_, cursor)) = xml_lit("SYSTEM").parse(cursor) {
            (None, cursor)
        } else if let Ok(((_, _, pubid), cursor)) =
            (xml_lit("PUBLIC"), SToken, PubidLiteralToken).parse(cursor)
        {
            (Some(pubid), cursor)
        } else {
            return Err(XmlError::ExpectToken("ExternalID"));
        };

        let (_, cursor) = SToken.parse(cursor)?;
        let (system, cursor) = SystemLiteralToken.parse(cursor)?;

        if let Some(pub_id) = pub_id {
            Ok((ExternalId::Public { pub_id, system }, cursor))
        } else {
            Ok((ExternalId::System { system }, cursor))
        }
    }
}

// NDataDecl ::= S 'NDATA' S Name

#[cfg(test)]
mod tests {
    use super::*;

    mod name {
        use crate::parser::Parser;
        use crate::reader::dtd::DocTypeDeclToken;
        use crate::{Cursor, XmlError};

        #[test]
        fn pass() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new("<!DOCTYPE elem>"))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!("elem", dtd.root_element_name())
        }

        #[test]
        fn accept_whitespace() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new("<!DOCTYPE\t\r\nelem\t\r\n>"))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!("elem", dtd.root_element_name())
        }

        #[test]
        fn fail_invalid_name() {
            let err = DocTypeDeclToken.parse(Cursor::new("<!DOCTYPE $e>"));
            assert_eq!(Err(XmlError::IllegalNameStartChar('$')), err)
        }
    }

    mod external_id {
        use crate::dtd::ExternalId;
        use crate::parser::Parser;
        use crate::reader::dtd::DocTypeDeclToken;
        use crate::{Cursor, XmlError};

        #[test]
        fn pass_system() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new("<!DOCTYPE e SYSTEM \"abc\">"))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                Some(ExternalId::System { system: "abc" }),
                dtd.external_id()
            );
        }

        #[test]
        fn pass_system_single_quote() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new("<!DOCTYPE e SYSTEM \'abc\'>"))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                Some(ExternalId::System { system: "abc" }),
                dtd.external_id()
            );
        }

        #[test]
        fn pass_public() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new("<!DOCTYPE e PUBLIC \"pubid\" 'system'>"))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                Some(ExternalId::Public {
                    pub_id: "pubid",
                    system: "system"
                }),
                dtd.external_id()
            );
        }

        #[test]
        fn pass_public_single_quote() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new("<!DOCTYPE e PUBLIC 'pubid' 'system'>"))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                Some(ExternalId::Public {
                    pub_id: "pubid",
                    system: "system"
                }),
                dtd.external_id()
            );
        }

        #[test]
        fn fail_invalid_pubid() {
            let result = DocTypeDeclToken.parse(Cursor::new("<!DOCTYPE e PUBLIC '{}' 'system'>"));
            assert!(result.is_err()); // TODO
        }

        #[test]
        fn fail_missing_system_id() {
            let result = DocTypeDeclToken.parse(Cursor::new("<!DOCTYPE e PUBLIC 'public'>"));
            assert!(result.is_err()); // TODO
        }
    }
}
