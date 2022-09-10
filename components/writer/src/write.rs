use std::{fmt, io};

pub trait UnicodeWrite {
    fn write_all(&mut self, s: &str) -> io::Result<()>;

    fn flush(&mut self) -> io::Result<()>;

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        struct PersistErrorWrapper<'a, T: 'a + ?Sized> {
            writer: &'a mut T,
            err: Option<io::Error>,
        }

        impl<'a, T: 'a + ?Sized + UnicodeWrite> fmt::Write for PersistErrorWrapper<'a, T> {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.writer.write_all(s).map_err(|err| {
                    self.err = Some(err);
                    fmt::Error
                })
            }
        }

        let mut writer = PersistErrorWrapper {
            writer: self,
            err: None,
        };
        fmt::write(&mut writer, fmt).map_err(|err| {
            if let Some(err) = writer.err {
                err
            } else {
                io::Error::new(io::ErrorKind::Other, "formatter error")
            }
        })
    }
}

#[macro_export]
macro_rules! unicode_write {
    ($writer:expr, $($args:tt)*) => ($writer.write_fmt(std::format_args!($($arg)*)))
}

impl UnicodeWrite for &mut String {
    fn write_all(&mut self, s: &str) -> io::Result<()> {
        self.push_str(s);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        fmt::write(self, fmt).map_err(|_| io::Error::new(io::ErrorKind::Other, "formatter error"))
    }
}

impl<'a, T: ?Sized + UnicodeWrite> UnicodeWrite for &mut T {
    fn write_all(&mut self, s: &str) -> io::Result<()> {
        (**self).write_all(s)
    }

    fn flush(&mut self) -> io::Result<()> {
        (**self).flush()
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        (**self).write_fmt(fmt)
    }
}

pub struct Uft8Writer<T: io::Write>(T);

impl<T: io::Write> UnicodeWrite for Uft8Writer<T> {
    fn write_all(&mut self, s: &str) -> io::Result<()> {
        self.0.write_all(s.as_bytes())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        self.0.write_fmt(fmt)
    }
}
