use xml_chars::{XmlAsciiChar, XmlChar};

use crate::dtd::{ContentSpec, DocTypeDecl, Element, ExternalId, IntSubset, MarkupDeclEntry};
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
pub struct IntSubsetToken;

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
pub struct MarkupDeclToken;

impl<'a> Parser<'a> for MarkupDeclToken {
    type Attribute = Option<MarkupDeclEntry>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Ok((_, cursor)) = SToken.parse(cursor) {
            Ok((None, cursor))
        } else if let Ok((element, cursor)) = ElementDeclToken.parse(cursor) {
            Ok((Some(MarkupDeclEntry::Element(element)), cursor))
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
pub struct ElementDeclToken;

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
pub struct ContentSpecToken;

impl<'a> Parser<'a> for ContentSpecToken {
    type Attribute = ContentSpec;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        if let Ok((_, cursor)) = xml_lit("EMPTY").parse(cursor) {
            Ok((ContentSpec::Empty, cursor))
        } else if let Ok((_, cursor)) = xml_lit("ANY").parse(cursor) {
            Ok((ContentSpec::Any, cursor))
        } else {
            todo!()
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
                    "greeting".to_string(),
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
                    "b".to_string(),
                    ContentSpec::Children(ContentParticle {
                        entry: ContentParticleEntry::Seq(vec![
                            ContentParticle {
                                entry: ContentParticleEntry::Name("front".to_string()),
                                repetition: Repetition::Once
                            },
                            ContentParticle {
                                entry: ContentParticleEntry::Name("body".to_string()),
                                repetition: Repetition::Once
                            },
                            ContentParticle {
                                entry: ContentParticleEntry::Name("back".to_string()),
                                repetition: Repetition::ZeroOrOne
                            }
                        ]),
                        repetition: Repetition::Once
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
                                repetition: Repetition::Once
                            },
                            ContentParticle {
                                entry: ContentParticleEntry::Choice(vec![
                                    ContentParticle {
                                        entry: ContentParticleEntry::Name("p".to_string()),
                                        repetition: Repetition::Once
                                    },
                                    ContentParticle {
                                        entry: ContentParticleEntry::Name("list".to_string()),
                                        repetition: Repetition::Once
                                    },
                                    ContentParticle {
                                        entry: ContentParticleEntry::Name("note".to_string()),
                                        repetition: Repetition::Once
                                    }
                                ]),
                                repetition: Repetition::ZeroOrMore
                            },
                            ContentParticle {
                                entry: ContentParticleEntry::Name("div2".to_string()),
                                repetition: Repetition::ZeroOrMore
                            }
                        ]),
                        repetition: Repetition::Once
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
}
