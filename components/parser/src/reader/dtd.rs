use xml_chars::{XmlAsciiChar, XmlChar};

use crate::dtd::{
    ContentParticle, ContentParticleEntry, ContentSpec, DocTypeDecl, Element, EntityDef,
    ExternalId, GEDecl, IntSubset, MarkupDeclEntry, PEDecl, PEDef, Repetition,
};
use crate::parser::core::{kleene, optional, separated, Kleene, Optional, Separated};
use crate::parser::helper::map_error;
use crate::parser::string::lit;
use crate::parser::Parser;
use crate::reader::{xml_lit, xml_terminated, CharTerminated, NameToken, SToken, TerminatedChars};
use crate::{Cursor, XmlDtdError, XmlError};

// 2.3 Common Syntactic Constructs
// Literals

/// External identifier literal
/// `SystemLiteral ::= ('"' [^"]* '"') | ("'" [^']* "'")`
struct SystemLiteralToken;

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
struct PubidLiteralToken;

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

/// `EntityValue ::= '"' ([^%&"] | PEReference | Reference)* '"'
///               |  "'" ([^%&'] | PEReference | Reference)* "'"`
pub struct EntityValueToken;

impl<'a> Parser<'a> for EntityValueToken {
    type Attribute = String;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (quote_char, cursor) = if let Ok((_, cursor)) = xml_lit("\"").parse(cursor) {
            (b'\"', cursor)
        } else if let Ok((_, cursor)) = xml_lit("\'").parse(cursor) {
            (b'\'', cursor)
        } else {
            return Err(XmlError::ExpectToken("EntityValue"));
        };

        // TODO: detect PEReference and Reference
        if let Some((pos, _)) = cursor
            .rest_bytes()
            .iter()
            .enumerate()
            .find(|(_, &c)| c == quote_char)
        {
            let (res, cursor) = cursor.advance2(pos);
            Ok((res.to_string(), cursor.advance(1)))
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
    type Attribute = DocTypeDecl;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = xml_lit("<!DOCTYPE").parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (name, cursor) = NameToken.parse(cursor)?;

        let (external_id, cursor) = optional((SToken, ExternalIdToken)).parse(cursor)?;
        let external_id = external_id.map(|v| v.1);
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (raw_internal, cursor) =
            optional((xml_lit("["), IntSubsetToken, xml_lit("]"), optional(SToken)))
                .parse(cursor)?;
        let (_, cursor) = xml_lit(">").parse(cursor)?;

        Ok((
            DocTypeDecl::new(
                name.to_string(),
                external_id,
                raw_internal.map(|subset| subset.1),
            ),
            cursor,
        ))
    }
}

/// Internal Subset
///
///     intSubset ::= (markupdecl | DeclSep)*
struct IntSubsetToken;

impl<'a> Parser<'a> for IntSubsetToken {
    type Attribute = IntSubset;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        kleene(MarkupDeclToken)
            .parse(cursor)
            .map(|(decls, cursor)| {
                (
                    IntSubset::new(decls.into_iter().filter_map(|decl| decl).collect()),
                    cursor,
                )
            })
    }
}

/// Parser for Markup Declaration or Declaration Seperator
///
///     markupdecl ::= elementdecl | AttlistDecl | EntityDecl | NotationDecl | PI | Comment
///     DeclSep ::= PEReference | S
///
struct MarkupDeclToken;

impl<'a> Parser<'a> for MarkupDeclToken {
    type Attribute = Option<MarkupDeclEntry>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Ok((_, cursor)) = SToken.parse(cursor) {
            Ok((None, cursor))
        } else if let Ok((element, cursor)) = ElementDeclToken.parse(cursor) {
            Ok((Some(MarkupDeclEntry::Element(element)), cursor))
        } else if let Ok((entity, cursor)) = GEDeclToken.parse(cursor) {
            Ok((Some(MarkupDeclEntry::GeneralEntity(entity)), cursor))
        } else if let Ok((entity, cursor)) = PEDeclToken.parse(cursor) {
            Ok((Some(MarkupDeclEntry::ParameterEntity(entity)), cursor))
        } else {
            // TODO
            Err(XmlError::UnexpectedDtdEntry)
        }

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

/// Element Type Declaration
///
///     elementdecl	   ::=   	'<!ELEMENT' S Name S contentspec S? '>'
///
struct ElementDeclToken;

impl<'a> Parser<'a> for ElementDeclToken {
    type Attribute = Element;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = xml_lit("<!ELEMENT").parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (name, cursor) = NameToken.parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (content_spec, cursor) = ContentSpecToken.parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (_, cursor) = xml_lit(">").parse(cursor)?;

        let element = Element {
            name: name.to_string(),
            content_spec,
        };
        Ok((element, cursor))
    }
}

/// Content Spec
///
///     contentspec	   ::=   	'EMPTY' | 'ANY' | Mixed | children
///
struct ContentSpecToken;

impl<'a> Parser<'a> for ContentSpecToken {
    type Attribute = ContentSpec;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Ok((_, cursor)) = xml_lit("EMPTY").parse(cursor) {
            Ok((ContentSpec::Empty, cursor))
        } else if let Ok((_, cursor)) = xml_lit("ANY").parse(cursor) {
            Ok((ContentSpec::Any, cursor))
        } else if let Ok((spec, cursor)) = MixedToken.parse(cursor) {
            Ok((spec, cursor))
        } else if let Ok((spec, cursor)) = ChildrenToken.parse(cursor) {
            Ok((spec, cursor))
        } else {
            Err(XmlError::DtdError(XmlDtdError::SyntaxError))
        }
    }
}

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
struct ChildrenToken;

impl<'a> Parser<'a> for ChildrenToken {
    type Attribute = ContentSpec;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (entry, cursor) = if let Ok((choice, cursor)) = ChoiceToken.parse(cursor) {
            (choice, cursor)
        } else if let Ok((seq, cursor)) = SeqToken.parse(cursor) {
            (seq, cursor)
        } else {
            return Err(XmlError::DtdError(XmlDtdError::SyntaxError));
        };
        let (repetition, cursor) = RepetitionToken.parse(cursor)?;

        Ok((
            ContentSpec::Children(ContentParticle { entry, repetition }),
            cursor,
        ))
    }
}

struct CpToken;

impl<'a> Parser<'a> for CpToken {
    type Attribute = ContentParticle;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (entry, cursor) = if let Ok((name, cursor)) = NameToken.parse(cursor) {
            (ContentParticleEntry::Name(name.to_string()), cursor)
        } else if let Ok((choice, cursor)) = ChoiceToken.parse(cursor) {
            (choice, cursor)
        } else if let Ok((seq, cursor)) = SeqToken.parse(cursor) {
            (seq, cursor)
        } else {
            return Err(XmlError::DtdError(XmlDtdError::SyntaxError));
        };
        let (repetition, cursor) = RepetitionToken.parse(cursor)?;

        Ok((ContentParticle { entry, repetition }, cursor))
    }
}

struct RepetitionToken;

impl<'a> Parser<'a> for RepetitionToken {
    type Attribute = Repetition;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Ok((choice, cursor)) = xml_lit("?").parse(cursor) {
            Ok((Repetition::ZeroOrOne, cursor))
        } else if let Ok((seq, cursor)) = xml_lit("*").parse(cursor) {
            Ok((Repetition::ZeroOrMore, cursor))
        } else if let Ok((seq, cursor)) = xml_lit("+").parse(cursor) {
            Ok((Repetition::OneOrMore, cursor))
        } else {
            Ok((Repetition::One, cursor))
        }
    }
}

struct ChoiceToken;

impl<'a> Parser<'a> for ChoiceToken {
    type Attribute = ContentParticleEntry;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = xml_lit("(").parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (cps, cursor) =
            separated(CpToken, (optional(SToken), xml_lit("|"), optional(SToken))).parse(cursor)?;

        if cps.len() < 2 {
            return Err(XmlError::DtdError(XmlDtdError::SyntaxError));
        }

        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (_, cursor) = xml_lit(")").parse(cursor)?;

        Ok((ContentParticleEntry::Choice(cps), cursor))
    }
}

struct SeqToken;

impl<'a> Parser<'a> for SeqToken {
    type Attribute = ContentParticleEntry;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = xml_lit("(").parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (cps, cursor) =
            separated(CpToken, (optional(SToken), xml_lit(","), optional(SToken))).parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (_, cursor) = xml_lit(")").parse(cursor)?;

        Ok((ContentParticleEntry::Seq(cps), cursor))
    }
}

// 3.2.2 Mixed Content

/// Mixed Content
///
///     Mixed	   ::=   	'(' S? '#PCDATA' (S? '|' S? Name)* S? ')*' | '(' S? '#PCDATA' S? ')'
struct MixedToken;

impl<'a> Parser<'a> for MixedToken {
    type Attribute = ContentSpec;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = xml_lit("(").parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (_, cursor) = optional(xml_lit("#PCDATA")).parse(cursor)?;
        let (names, cursor) =
            kleene((optional(SToken), xml_lit("|"), optional(SToken), NameToken)).parse(cursor)?;
        let (_, cursor) = xml_lit(")").parse(cursor)?;

        if let Ok((_, cursor)) = xml_lit("*").parse(cursor) {
            Ok((
                ContentSpec::Mixed(names.into_iter().map(|name| name.3.to_string()).collect()),
                cursor,
            ))
        } else {
            if names.len() == 0 {
                Ok((ContentSpec::PCData, cursor))
            } else {
                Err(XmlError::DtdError(XmlDtdError::SyntaxError))
            }
        }
    }
}

// 4.1 Character and Entity References

// PEReference	   ::=   	'%' Name ';'

// 4.2 Entity Declarations

/// `GEDecl ::= '<!ENTITY' S Name S EntityDef S? '>'`
struct GEDeclToken;

impl<'a> Parser<'a> for GEDeclToken {
    type Attribute = GEDecl;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = xml_lit("<!ENTITY").parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (name, cursor) = NameToken.parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (def, cursor) = EntityDefToken.parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;
        let (_, cursor) = xml_lit(">").parse(cursor)?;

        Ok((
            GEDecl {
                name: name.to_string(),
                def,
            },
            cursor,
        ))
    }
}

/// `EntityDef ::= EntityValue | (ExternalID NDataDecl?)`
struct EntityDefToken;

impl<'a> Parser<'a> for EntityDefToken {
    type Attribute = EntityDef;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Ok((value, cursor)) = EntityValueToken.parse(cursor) {
            Ok((EntityDef::Internal(value), cursor))
        } else if let Ok(((external_id, ndata), cursor)) =
            (ExternalIdToken, optional(NDataDeclToken)).parse(cursor)
        {
            Ok((EntityDef::External { external_id, ndata }, cursor))
        } else {
            Err(XmlError::DtdError(XmlDtdError::SyntaxError))
        }
    }
}

/// `PEDecl ::= '<!ENTITY' S '%' S Name S PEDef S? '>'`
struct PEDeclToken;

impl<'a> Parser<'a> for PEDeclToken {
    type Attribute = PEDecl;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = xml_lit("<!ENTITY").parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (_, cursor) = xml_lit("%").parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (name, cursor) = NameToken.parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (def, cursor) = PEDefToken.parse(cursor)?;
        let (_, cursor) = optional(SToken).parse(cursor)?;

        Ok((
            PEDecl {
                name: name.to_string(),
                def,
            },
            cursor,
        ))
    }
}

/// `PEDef ::= EntityValue | ExternalID`
struct PEDefToken;

impl<'a> Parser<'a> for PEDefToken {
    type Attribute = PEDef;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Ok((value, cursor)) = EntityValueToken.parse(cursor) {
            Ok((PEDef::Internal(value), cursor))
        } else if let Ok((external_id, cursor)) = ExternalIdToken.parse(cursor) {
            Ok((PEDef::External(external_id), cursor))
        } else {
            Err(XmlError::DtdError(XmlDtdError::SyntaxError))
        }
    }
}

// 4.2.2 External Entities

/// ExternalID ::= 'SYSTEM' S SystemLiteral | 'PUBLIC' S PubidLiteral S SystemLiteral
pub struct ExternalIdToken;

impl<'a> Parser<'a> for ExternalIdToken {
    type Attribute = ExternalId;
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
            Ok((
                ExternalId::Public {
                    pub_id: pub_id.to_string(),
                    system: system.to_string(),
                },
                cursor,
            ))
        } else {
            Ok((
                ExternalId::System {
                    system: system.to_string(),
                },
                cursor,
            ))
        }
    }
}

/// `NDataDecl ::= S 'NDATA' S Name`
pub struct NDataDeclToken;

impl<'a> Parser<'a> for NDataDeclToken {
    type Attribute = String;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        let (_, cursor) = SToken.parse(cursor)?;
        let (_, cursor) = xml_lit("NDATA").parse(cursor)?;
        let (_, cursor) = SToken.parse(cursor)?;
        let (name, cursor) = NameToken.parse(cursor)?;
        Ok((name.to_string(), cursor))
    }
}

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
                Some(ExternalId::System {
                    system: "abc".to_string()
                }),
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
                Some(ExternalId::System {
                    system: "abc".to_string()
                }),
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
                    pub_id: "pubid".to_string(),
                    system: "system".to_string()
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
                    pub_id: "pubid".to_string(),
                    system: "system".to_string()
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

    /// 3.2 Element Type Declarations
    mod element {
        use crate::dtd::{
            ContentParticle, ContentParticleEntry, ContentSpec, Element, ExternalId, IntSubset,
            MarkupDeclEntry, Repetition,
        };
        use crate::parser::Parser;
        use crate::reader::dtd::DocTypeDeclToken;
        use crate::{Cursor, XmlError};

        #[test]
        fn empty() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new("<!DOCTYPE e [ <!ELEMENT br EMPTY> ]>"))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_element(
                    "br".to_string(),
                    ContentSpec::Empty
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        fn any() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new("<!DOCTYPE e [ <!ELEMENT container ANY> ]>"))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_element(
                    "container".to_string(),
                    ContentSpec::Any
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        fn mixed_data_1() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE greeting [ <!ELEMENT greeting (#PCDATA)> ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_element(
                    "greeting".to_string(),
                    ContentSpec::PCData
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        fn mixed_data_2() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ELEMENT p (#PCDATA|emph)* > ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_element(
                    "p".to_string(),
                    ContentSpec::Mixed(vec!["emph".to_string()])
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        fn mixed_data_3() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ELEMENT p (#PCDATA|a|ul|b|i|em)*> ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_element(
                    "p".to_string(),
                    ContentSpec::Mixed(vec![
                        "a".to_string(),
                        "ul".to_string(),
                        "b".to_string(),
                        "i".to_string(),
                        "em".to_string()
                    ])
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        #[ignore]
        fn mixed_data_4() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ELEMENT p (#PCDATA | %font; | %phrase; | %special; | %form;)* > ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_element(
                    "p".to_string(),
                    ContentSpec::Mixed(vec![])
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        fn element_content_1() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ELEMENT spec (front, body, back?)> ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_element(
                    "spec".to_string(),
                    ContentSpec::Children(ContentParticle {
                        entry: ContentParticleEntry::Seq(vec![
                            ContentParticle {
                                entry: ContentParticleEntry::Name("front".to_string()),
                                repetition: Repetition::One
                            },
                            ContentParticle {
                                entry: ContentParticleEntry::Name("body".to_string()),
                                repetition: Repetition::One
                            },
                            ContentParticle {
                                entry: ContentParticleEntry::Name("back".to_string()),
                                repetition: Repetition::ZeroOrOne
                            }
                        ]),
                        repetition: Repetition::One
                    })
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        fn element_content_2() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ELEMENT div1 (head, (p | list | note)*, div2*)> ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_element(
                    "div1".to_string(),
                    ContentSpec::Children(ContentParticle {
                        entry: ContentParticleEntry::Seq(vec![
                            ContentParticle {
                                entry: ContentParticleEntry::Name("head".to_string()),
                                repetition: Repetition::One
                            },
                            ContentParticle {
                                entry: ContentParticleEntry::Choice(vec![
                                    ContentParticle {
                                        entry: ContentParticleEntry::Name("p".to_string()),
                                        repetition: Repetition::One
                                    },
                                    ContentParticle {
                                        entry: ContentParticleEntry::Name("list".to_string()),
                                        repetition: Repetition::One
                                    },
                                    ContentParticle {
                                        entry: ContentParticleEntry::Name("note".to_string()),
                                        repetition: Repetition::One
                                    }
                                ]),
                                repetition: Repetition::ZeroOrMore
                            },
                            ContentParticle {
                                entry: ContentParticleEntry::Name("div2".to_string()),
                                repetition: Repetition::ZeroOrMore
                            }
                        ]),
                        repetition: Repetition::One
                    })
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        #[ignore]
        fn element_content_3() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ELEMENT dictionary-body (%div.mix; | %dict.mix;)*> ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(&Some(IntSubset::new(vec![])), dtd.internal_subset());
        }
    }

    /// 4.2 Entity Declarations
    mod entities {
        use crate::dtd::{
            ContentParticle, ContentParticleEntry, ContentSpec, Element, EntityDef, ExternalId,
            GEDecl, IntSubset, MarkupDeclEntry, Repetition,
        };
        use crate::parser::Parser;
        use crate::reader::dtd::DocTypeDeclToken;
        use crate::{Cursor, XmlError};

        #[test]
        fn internal() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ENTITY Pub-Status \"This is a pre-release of the specification.\"> ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_entity(
                    "Pub-Status".to_string(),
                    EntityDef::Internal("This is a pre-release of the specification.".to_string())
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        fn external1() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ENTITY open-hatch SYSTEM \"http://www.textuality.com/boilerplate/OpenHatch.xml\"> ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_entity(
                    "open-hatch".to_string(),
                    EntityDef::External {
                        external_id: ExternalId::System {
                            system: "http://www.textuality.com/boilerplate/OpenHatch.xml"
                                .to_string()
                        },
                        ndata: None
                    }
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        fn external2() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ENTITY open-hatch PUBLIC \"-//Textuality//TEXT Standard open-hatch boilerplate//EN\" \"http://www.textuality.com/boilerplate/OpenHatch.xml\"> ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_entity(
                    "open-hatch".to_string(),
                    EntityDef::External {
                        external_id: ExternalId::Public {
                            pub_id: "-//Textuality//TEXT Standard open-hatch boilerplate//EN"
                                .to_string(),
                            system: "http://www.textuality.com/boilerplate/OpenHatch.xml"
                                .to_string()
                        },
                        ndata: None
                    }
                )])),
                dtd.internal_subset()
            );
        }

        #[test]
        fn external3() {
            let (dtd, cursor) = DocTypeDeclToken
                .parse(Cursor::new(
                    "<!DOCTYPE e [ <!ENTITY hatch-pic SYSTEM \"../grafix/OpenHatch.gif\" NDATA gif> ]>",
                ))
                .unwrap();
            assert!(cursor.is_at_end());
            assert_eq!(
                &Some(IntSubset::new(vec![MarkupDeclEntry::new_entity(
                    "hatch-pic".to_string(),
                    EntityDef::External {
                        external_id: ExternalId::System {
                            system: "../grafix/OpenHatch.gif".to_string()
                        },
                        ndata: Some("gif".to_string())
                    }
                )])),
                dtd.internal_subset()
            );
        }
    }
}
