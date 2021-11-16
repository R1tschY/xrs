use std::borrow::Cow;
use std::io;

fn is_whitespace(c: u8) -> bool {
    c == b'\x20' || c == b'\x09' || c == b'\x0D' || c == b'\x0A'
}

pub type DtdResult<T> = std::result::Result<T, Error>;

pub trait Read<'de> {
    fn next(&mut self) -> DtdResult<Option<u8>>;
    fn peek(&mut self) -> DtdResult<Option<u8>>;
    fn offset(&self) -> usize;
}

struct StreamRead<R> {
    bytes: io::Bytes<R>,
    offset: usize,
    peek: Option<u8>,
}

impl<'de, R: io::Read> Read<'de> for StreamRead<R> {
    fn next(&mut self) -> DtdResult<Option<u8>> {
        self.offset += 1;
        match self.peek.take() {
            Some(peek) => Ok(Some(peek)),
            _ => match self.bytes.next() {
                Some(Ok(peek)) => Ok(Some(peek)),
                Some(Err(err)) => Err(Error::io(err)),
                None => Ok(None),
            },
        }
    }

    fn peek(&mut self) -> DtdResult<Option<u8>> {
        if let Some(peek) = &self.peek {
            Ok(Some(*peek))
        } else {
            match self.bytes.next() {
                Some(Ok(peek)) => {
                    self.peek = Some(peek);
                    Ok(Some(peek))
                }
                Some(Err(err)) => Err(Error::io(err)),
                None => Ok(None),
            }
        }
    }

    fn offset(&self) -> usize {
        self.offset
    }
}

struct SliceRead<'de> {
    bytes: &'de [u8],
    offset: usize,
    peek: Option<u8>,
}

#[derive(Debug)]
pub struct Error {
    reason: ErrorReason,
    offset: usize,
    length: usize,
}

impl Error {
    pub fn new(reason: ErrorReason, offset: usize, length: usize) -> Self {
        Self {
            reason,
            offset,
            length,
        }
    }

    pub fn io(err: io::Error) -> Self {
        Self {
            reason: ErrorReason::Io(err),
            offset: 0,
            length: 0,
        }
    }
}

#[derive(Debug)]
pub enum ErrorReason {
    Io(io::Error),
    ExpectAttributeOrClose,
    ExpectAttributeValue,
    WrongElementClose,
    ExpectWhitespace,
    Eof,
}

pub enum Event<'a> {
    Element {
        name: Cow<'a, str>,
        content_spec: String,
    },
    Attlist(),
    Entity(),
}

pub struct XmlParser<R> {
    reader: R,
}

impl<'de, R: Read<'de>> XmlParser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    fn error(&self, reason: ErrorReason) -> Error {
        Error::new(reason, self.reader.offset(), 0)
    }

    // fn skip_whitespace(&mut self) -> DtdResult<()> {
    //     loop {
    //         match self.reader.peek_forward()? {
    //             Some(c) if is_whitespace(c) => continue,
    //             _ => return Ok(()),
    //         }
    //     }
    // }

    fn expect_whitespace(&mut self) -> DtdResult<()> {
        match self.read_exact_one()? {
            c if is_whitespace(c) => Ok(()),
            _ => Err(self.error(ErrorReason::ExpectWhitespace)),
        }
    }

    fn collect_name(&mut self) {}

    fn read_exact_one(&mut self) -> DtdResult<u8> {
        match self.reader.next()? {
            Some(c) => Ok(c),
            None => Err(self.error(ErrorReason::Eof)),
        }
    }

    fn peek_exact_one(&mut self) -> DtdResult<u8> {
        match self.reader.peek()? {
            Some(c) => Ok(c),
            None => Err(self.error(ErrorReason::Eof)),
        }
    }

    // pub fn next_element(&mut self) -> DtdResult<Option<Item<'de>>> {
    //     loop {
    //         match self.reader.next()? {
    //             Some(b'<') => {
    //                 let start_offset = self.reader.offset();
    //                 self.parse_prefix(b"!", start_offset)?;
    //                 return match self.expect_next()? {
    //                     b'E' => match self.expect_next()? {
    //                         b'N' => {
    //                             // ENTITY
    //                             self.parse_prefix(b"TITY", start_offset)?;
    //                             self.parse_entity()
    //                         }
    //                         b'L' => {
    //                             // ELEMENT
    //                             self.parse_prefix(b"EMENT", start_offset)?;
    //                             self.parse_element()
    //                         }
    //                         _ => Err(self.prefix_error(start_offset)),
    //                     },
    //                     b'-' => {
    //                         self.parse_prefix(b"-", start_offset)?;
    //                         self.parse_comment()
    //                     }
    //                     _ => Err(self.error(ErrorReason::ExpectItem)),
    //                 };
    //             }
    //             Some(e) if is_whitespace(e) => (),
    //             Some(_) => return Err(self.error(ErrorReason::ExpectItem)),
    //             None => return Ok(None),
    //         }
    //     }
    // }
}
