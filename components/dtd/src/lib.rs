/*use std::borrow::Cow;
use std::io;

fn is_whitespace(c: u8) -> bool {
    c == b'\x20' || c == b'\x09' || c == b'\x0D' || c == b'\x0A'
}

pub type DtdResult<T> = std::result::Result<T, DtdError>;

pub trait Read<'de> {
    fn next(&mut self) -> DtdResult<Option<u8>>;
    fn peek(&mut self) -> DtdResult<Option<u8>>;
    fn peek_forward(&mut self) -> DtdResult<Option<u8>>;
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
            None => match self.bytes.next() {
                Some(Ok(peek)) => Ok(Some(peek)),
                Some(Err(err)) => Err(DtdError::io(err)),
                None => Ok(None),
            },
        }
    }

    fn peek_forward(&mut self) -> DtdResult<Option<u8>> {
        if let Some(_peek) = &self.peek {
            self.offset += 1;
            match self.bytes.next() {
                Some(Ok(peek)) => {
                    self.peek = Some(peek);
                    Ok(Some(peek))
                }
                Some(Err(err)) => Err(DtdError::io(err)),
                None => Ok(None),
            }
        } else {
            self.peek()
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
                Some(Err(err)) => Err(DtdError::io(err)),
                None => Ok(None),
            }
        }
    }

    fn offset(&self) -> usize {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct DtdError {
    reason: DtdErrorReason,
    offset: usize,
    length: usize,
}

impl DtdError {
    pub fn new(reason: DtdErrorReason, offset: usize, length: usize) -> Self {
        Self {
            reason,
            offset,
            length,
        }
    }

    pub fn io(err: io::Error) -> Self {
        Self {
            reason: DtdErrorReason::Io(err),
            offset: 0,
            length: 0,
        }
    }
}

#[derive(Debug)]
pub enum DtdErrorReason {
    Io(io::Error),
    ExpectItem,
    ExpectIdent,
    ExpectWhitespace,
    Eof,
}

pub enum Item<'a> {
    Element {
        name: Cow<'a, str>,
        content_spec: String,
    },
    Attlist(),
    Entity(),
}

pub enum ContentSpec<'a> {
    Empty,
    Any,
    Mixed(Vec<Cow<'a, str>>),
    Children(),
}

pub struct ContentParticle<'a> {
    ty: ContentParticleElement<'a>,
    repeat: Repeat,
}

pub enum Repeat {
    One,
    Optional,
    OptionalMultiple,
    Multiple,
}

pub enum ContentParticleElement<'a> {
    Name(Cow<'a, str>),
    Choice(Vec<ContentParticle<'a>>),
    Seq(Vec<ContentParticle<'a>>),
}

pub struct DtdParser<R> {
    reader: R,
}

impl<'de, R: Read<'de>> DtdParser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    fn error(&self, reason: DtdErrorReason) -> DtdError {
        DtdError::new(reason, self.reader.offset(), 0)
    }

    fn prefix_error(&self, start_offset: usize) -> DtdError {
        DtdError::new(
            DtdErrorReason::ExpectIdent,
            start_offset,
            self.reader.offset() - start_offset,
        )
    }

    fn parse_prefix(&mut self, prefix: &[u8], start_offset: usize) -> DtdResult<()> {
        for c in prefix {
            match self.reader.next()? {
                None => return Err(self.error(DtdErrorReason::Eof)),
                Some(char) => {
                    if char != *c {
                        return Err(self.prefix_error(start_offset));
                    }
                }
            }
        }
        self.expect_whitespace()?;
        self.skip_whitespace()?;
        Ok(())
    }

    fn skip_whitespace(&mut self) -> DtdResult<()> {
        loop {
            match self.reader.peek_forward()? {
                Some(c) if is_whitespace(c) => continue,
                _ => return Ok(()),
            }
        }
    }

    fn expect_whitespace(&mut self) -> DtdResult<()> {
        match self.expect_next()? {
            c if is_whitespace(c) => Ok(()),
            _ => Err(self.error(DtdErrorReason::ExpectWhitespace)),
        }
    }

    fn collect_name(&mut self) {}

    fn parse_entity(&mut self) -> DtdResult<Option<Item<'de>>> {
        match self.expect_peek()? {
            b'E' => self.parse_prefix(b"MPTY", self.reader.offset()),
            b'A' => self.parse_prefix(b"NY", self.reader.offset()),
        }

        Ok(Some())
    }

    fn parse_element(&mut self) -> DtdResult<Option<Item<'de>>> {
        Ok(Some())
    }

    fn expect_next(&mut self) -> DtdResult<u8> {
        match self.reader.next()? {
            Some(c) => Ok(c),
            None => Err(self.error(DtdErrorReason::Eof)),
        }
    }

    fn expect_peek(&mut self) -> DtdResult<u8> {
        match self.reader.peek()? {
            Some(c) => Ok(c),
            None => Err(self.error(DtdErrorReason::Eof)),
        }
    }

    pub fn next_item(&mut self) -> DtdResult<Option<Item<'de>>> {
        loop {
            match self.reader.next()? {
                Some(b'<') => {
                    let start_offset = self.reader.offset();
                    self.parse_prefix(b"!", start_offset)?;
                    return match self.expect_next()? {
                        b'E' => match self.expect_next()? {
                            b'N' => {
                                // ENTITY
                                self.parse_prefix(b"TITY", start_offset)?;
                                self.parse_entity()
                            }
                            b'L' => {
                                // ELEMENT
                                self.parse_prefix(b"EMENT", start_offset)?;
                                self.parse_element()
                            }
                            _ => Err(self.prefix_error(start_offset)),
                        },
                        b'-' => {
                            self.parse_prefix(b"-", start_offset)?;
                            self.parse_comment()
                        }
                        _ => Err(self.error(DtdErrorReason::ExpectItem)),
                    };
                }
                Some(e) if is_whitespace(e) => (),
                Some(_) => return Err(self.error(DtdErrorReason::ExpectItem)),
                None => return Ok(None),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
*/
