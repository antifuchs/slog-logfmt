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
//! DEBG | #tag	hi there	logger="tests" foo="bar\'baz\""
//! ```
//!

use slog::{o, Error, Key, OwnedKVList, Record, Value, KV};
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
    prefix: fn(&mut dyn io::Write, &Record) -> slog::Result,
    skip_fields: HashSet<Key>,
    print_level: bool,
    print_msg: bool,
    print_tag: bool,
}

impl<W: io::Write> Logfmt<W> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(io: W) -> LogfmtBuilder<W> {
        LogfmtBuilder {
            io,
            prefix: None,
            skip_fields: HashSet::new(),
            print_level: false,
            print_msg: false,
            print_tag: false,
        }
    }
}

/// A constructor for a [`Logfmt`] drain.
pub struct LogfmtBuilder<W: io::Write> {
    io: W,
    prefix: Option<fn(&mut dyn io::Write, &Record) -> slog::Result>,
    skip_fields: HashSet<Key>,
    print_msg: bool,
    print_level: bool,
    print_tag: bool,
}

impl<W: io::Write> LogfmtBuilder<W> {
    /// Constructs the drain.
    pub fn build(self) -> Logfmt<W> {
        Logfmt {
            io: RefCell::new(self.io),
            prefix: self.prefix.unwrap_or(default_prefix),
            skip_fields: self.skip_fields,
            print_msg: self.print_msg,
            print_level: self.print_level,
            print_tag: self.print_tag,
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

    /// Choose whether to print the log message.
    ///
    /// The default prefix already prints it, so the default is to skip.
    pub fn print_msg(mut self, print: bool) -> Self {
        self.print_msg = print;
        self
    }

    /// Choose whether to print the log level.
    ///
    /// The default prefix already prints it, so the default is to skip.
    pub fn print_level(mut self, print: bool) -> Self {
        self.print_level = print;
        self
    }

    /// Choose whether to print the log level.
    ///
    /// The default prefix already prints it, so the default is to skip.
    pub fn print_tag(mut self, print: bool) -> Self {
        self.print_tag = print;
        self
    }
}

fn default_prefix(io: &mut dyn io::Write, rec: &Record) -> slog::Result {
    let tag_prefix = if rec.tag() == "" { "" } else { "#" };
    let tag_suffix = if rec.tag() == "" { "" } else { "\t" };
    write!(
        io,
        "{level} | {tag_prefix}{tag}{tag_suffix}{msg}\t",
        tag_prefix = tag_prefix,
        tag = rec.tag(),
        tag_suffix = tag_suffix,
        level = rec.level().as_short_str(),
        msg = rec.msg()
    )?;
    Ok(())
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
        // TODO: `Debug` is kinda right, but excessive. Try to not quote strings when we can.
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
        let prefix = self.prefix;
        prefix(&mut *io, record)?;

        let mut serializer = LogfmtSerializer {
            io: &mut *io,
            first: true,
            skip_fields: &self.skip_fields,
        };
        if self.print_level {
            let lvl = o!("level" => record.level().as_short_str());
            lvl.serialize(record, &mut serializer)?;
        }
        if self.print_msg {
            record.msg().serialize(record, "msg", &mut serializer)?;
        }
        if self.print_tag {
            let tag = o!("level" => record.tag());
            tag.serialize(record, &mut serializer)?;
        }
        logger_values.serialize(record, &mut serializer)?;
        record.kv().serialize(record, &mut serializer)?;

        io.write_all(b"\n")?;
        io.flush()?;

        Ok(())
    }
}
