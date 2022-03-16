use crate::dtd::{DocTypeDecl, IntSubset, MarkupDeclEntry};
use crate::parser::core::{kleene, optional, Kleene, Optional};
use crate::parser::helper::map_error;
use crate::parser::string::lit;
use crate::parser::Parser;
use crate::reader::{xml_lit, NameToken, SToken};
use crate::{Cursor, XmlError};

// 2.8 Prolog and Document Type Declaration
// Document Type Declaration

///	doctypedecl ::= '<!DOCTYPE' S Name (S ExternalID)? S? ('\[' intSubset '\]' S?)? '>'
pub struct DocTypeDeclToken;

impl<'a> Parser<'a> for DocTypeDeclToken {
    type Attribute = DocTypeDecl<'a>;
    type Error = XmlError;

    fn parse(&self, cursor: Cursor<'a>) -> Result<(Self::Attribute, Cursor<'a>), Self::Error> {
        todo!()
        // let (_, cursor) = xml_lit("<!DOCTYPE").parse(cursor)?;
        // let (_, cursor) = SToken.parse(cursor)?;
        // let (name, cursor) = NameToken.parse(cursor)?;
        // let ((_, external_id), cursor) = optional((SToken, ExternalIdToken)).parse(cursor)?;
        // let (_, cursor) = optional(SToken).parse(cursor)?;
        // let (_, cursor) = optional((xml_lit("["), IntSubsetToken, xml_lit("]"), optional(SToken)))
        //     .parse(cursor)?;
        // let (_, cursor) = xml_lit(">").parse(cursor)?;
        //
        // Ok((DocTypeDecl::new(name), cursor))
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
pub struct ExternalID;

// NDataDecl ::= S 'NDATA' S Name
