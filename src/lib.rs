//! `slog_logfmt` - a [`logfmt`](https://brandur.org/logfmt) formatter for slog.
//!
//! This crate exposes a `slog` drain that formats messages as logfmt.
//!
//! # Example
//! ```rust
//! use slog_logfmt::Logfmt;
//! use slog::{debug, o, Drain, Logger};
//! use std::io::stdout;
//!
//! let drain = Logfmt::new(stdout()).build().fuse();
//! let drain = slog_async::Async::new(drain).build().fuse();
//! let logger = Logger::root(drain, o!("logger" => "tests"));
//! debug!(logger, #"tag", "hi there"; "foo" => "bar'baz\"");
//! ```
//!
//! Writes:
//! ```text
//! logger="tests" msg="hi there" foo="bar\'baz\""
//! ```
//!

use slog::{Error, Key, OwnedKVList, Record, Value, KV};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Arguments;
use std::io;

/// A drain & formatter for [logfmt](https://brandur.org/logfmt)-formatted messages.
///
/// # Format
/// The default format looks like the somewhat-more-human-readable
/// format in https://brandur.org/logfmt#human. You can customize it
/// with the [`LogfmtBuilder`] method `set_prefix`.
pub struct Logfmt<W: io::Write> {
    io: RefCell<W>,
    prefix: Option<fn(&mut dyn io::Write, &Record) -> slog::Result>,
    skip_fields: HashSet<Key>,
}

impl<W: io::Write> Logfmt<W> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(io: W) -> LogfmtBuilder<W> {
        LogfmtBuilder {
            io,
            prefix: None,
            skip_fields: HashSet::new(),
        }
    }
}

/// A constructor for a [`Logfmt`] drain.
pub struct LogfmtBuilder<W: io::Write> {
    io: W,
    prefix: Option<fn(&mut dyn io::Write, &Record) -> slog::Result>,
    skip_fields: HashSet<Key>,
}

impl<W: io::Write> LogfmtBuilder<W> {
    /// Constructs the drain.
    pub fn build(self) -> Logfmt<W> {
        Logfmt {
            io: RefCell::new(self.io),
            prefix: self.prefix,
            skip_fields: self.skip_fields,
        }
    }

    /// Set a function that prints a (not necessarily
    /// logfmt-formatted) prefix to the output stream.
    pub fn set_prefix(mut self, prefix: fn(&mut dyn io::Write, &Record) -> slog::Result) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// A list of fields that should not be printed with the `Logfmt` formatter.
    ///
    /// These could be emitted with the `set_prefix` prefixer, or
    /// could just be skipped altogether for different reasons.
    pub fn skip_fields(mut self, keys: impl IntoIterator<Item = Key>) -> Self {
        self.skip_fields = keys.into_iter().collect();
        self
    }
}

struct LogfmtSerializer<'a, W: io::Write> {
    io: &'a mut W,
    first: bool,
    skip_fields: &'a HashSet<Key>,
}

impl<'a, W: io::Write> LogfmtSerializer<'a, W> {
    fn next_field(&mut self) -> Result<(), io::Error> {
        if self.first {
            self.first = false;
        } else {
            write!(self.io, " ")?;
        }
        Ok(())
    }
}

macro_rules! w(
    ($s:expr, $k:expr, $v:expr) => {{
        if $s.skip_fields.contains(&$k) {
            return Ok(())
        }
        $s.next_field()?;
        write!($s.io, "{}={:?}", $k, $v)?;
        Ok(())
    }};
);

impl<'a, W> slog::Serializer for LogfmtSerializer<'a, W>
where
    W: io::Write,
{
    fn emit_usize(&mut self, key: &'static str, val: usize) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_isize(&mut self, key: &'static str, val: isize) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_bool(&mut self, key: &'static str, val: bool) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_char(&mut self, key: &'static str, val: char) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u8(&mut self, key: &'static str, val: u8) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i8(&mut self, key: &'static str, val: i8) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u16(&mut self, key: &'static str, val: u16) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i16(&mut self, key: &'static str, val: i16) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u32(&mut self, key: &'static str, val: u32) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i32(&mut self, key: &'static str, val: i32) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_f32(&mut self, key: &'static str, val: f32) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u64(&mut self, key: &'static str, val: u64) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i64(&mut self, key: &'static str, val: i64) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_f64(&mut self, key: &'static str, val: f64) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u128(&mut self, key: &'static str, val: u128) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i128(&mut self, key: &'static str, val: i128) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_str(&mut self, key: &'static str, val: &str) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_unit(&mut self, key: &'static str) -> Result<(), Error> {
        w!(self, key, ())
    }

    fn emit_none(&mut self, key: &'static str) -> Result<(), Error> {
        let o: Option<()> = None;
        w!(self, key, o)
    }

    fn emit_arguments<'b>(&mut self, key: &'static str, val: &Arguments<'b>) -> Result<(), Error> {
        let val = format!("{}", val);
        w!(self, key, val)
    }
}

impl<W> slog::Drain for Logfmt<W>
where
    W: io::Write,
{
    type Ok = ();
    type Err = io::Error;

    fn log<'a>(
        &self,
        record: &Record<'a>,
        logger_values: &OwnedKVList,
    ) -> Result<Self::Ok, Self::Err> {
        let mut io = self.io.borrow_mut();

        if let Some(prefix) = self.prefix {
            prefix(&mut *io, record)?;
        }

        let mut serializer = LogfmtSerializer {
            io: &mut *io,
            first: true,
            skip_fields: &self.skip_fields,
        };
        logger_values.serialize(record, &mut serializer)?;
        record.msg().serialize(record, "msg", &mut serializer)?;
        record.kv().serialize(record, &mut serializer)?;

        io.write_all(b"\n")?;
        io.flush()?;

        Ok(())
    }
}
