#![allow(clippy::tabs_in_doc_comments)]
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
use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::Arguments;
use std::io;

/// A decision on whether to print a key/value pair.
pub enum Redaction {
    /// Print the value as-is.
    Plain,

    /// Do not print the entry at all.
    Skip,

    /// Redact the value with the given function.
    Redact(fn(&'_ dyn Value) -> Arguments),
}

struct Options {
    prefix: fn(&mut dyn io::Write, &Record) -> slog::Result,
    print_level: bool,
    print_msg: bool,
    print_tag: bool,
    force_quotes: bool,
    redactor: fn(&Key) -> Redaction,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            prefix: default_prefix,
            print_level: false,
            print_msg: false,
            print_tag: false,
            force_quotes: false,
            redactor: |_| Redaction::Plain,
        }
    }
}

/// A drain & formatter for [logfmt](https://brandur.org/logfmt)-formatted messages.
///
/// # Format
/// The default format looks like the somewhat-more-human-readable
/// format in https://brandur.org/logfmt#human. You can customize it
/// with the [`LogfmtBuilder`] method `set_prefix`.
pub struct Logfmt<W: io::Write> {
    io: RefCell<W>,
    options: Options,
}

impl<W: io::Write> Logfmt<W> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(io: W) -> LogfmtBuilder<W> {
        LogfmtBuilder {
            io,
            options: Default::default(),
        }
    }
}

/// A constructor for a [`Logfmt`] drain.
pub struct LogfmtBuilder<W: io::Write> {
    io: W,
    options: Options,
}

impl<W: io::Write> LogfmtBuilder<W> {
    /// Constructs the drain.
    pub fn build(self) -> Logfmt<W> {
        Logfmt {
            io: RefCell::new(self.io),
            options: self.options,
        }
    }

    /// Set a function that prints a (not necessarily
    /// logfmt-formatted) prefix to the output stream.
    pub fn set_prefix(mut self, prefix: fn(&mut dyn io::Write, &Record) -> slog::Result) -> Self {
        self.options.prefix = prefix;
        self
    }

    /// Sets the logger up to print no prefix, effectively starting the line entirely
    /// logfmt field-formatted.
    pub fn no_prefix(mut self) -> Self {
        self.options.prefix = |_, _| Ok(());
        self
    }

    /// Sets a function that makes decisions on whether to log a field.
    ///
    /// This function must return a [`Redaction`] result, which has
    /// two variants at the moment: `Redact::Skip` to not log the
    /// field, and `Redact::Plain` to log the field value in plain
    /// text.
    pub fn redact(mut self, redact: fn(&Key) -> Redaction) -> Self {
        self.options.redactor = redact;
        self
    }

    /// Choose whether to print the log message.
    ///
    /// The default prefix already prints it, so the default is to skip.
    pub fn print_msg(mut self, print: bool) -> Self {
        self.options.print_msg = print;
        self
    }

    /// Choose whether to print the log level.
    ///
    /// The default prefix already prints it, so the default is to skip.
    pub fn print_level(mut self, print: bool) -> Self {
        self.options.print_level = print;
        self
    }

    /// Choose whether to print the log level.
    ///
    /// The default prefix already prints it, so the default is to skip.
    pub fn print_tag(mut self, print: bool) -> Self {
        self.options.print_tag = print;
        self
    }

    /// Force quoting field values even if they don't contain quotable characters.
    ///
    /// Setting this option will surround values with quotes like `foo="bar"`.
    pub fn force_quotes(mut self) -> Self {
        self.options.force_quotes = true;
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
    force_quotes: bool,
    redactor: fn(&Key) -> Redaction,
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
        use Redaction::*;

        let redact = $s.redactor;
        let val = $v;
        match redact(&$k) {
            Skip => {return Ok(());}
            Plain => {
                $s.next_field()?;
                write!($s.io, "{}={}", $k, val)?;
                Ok(())
            },
            Redact(redactor) => {
                $s.next_field()?;
                let val = format!("{}", redactor(&val));
                write!($s.io, "{}={}", $k, optionally_quote(&val, $s.force_quotes))?;
                Ok(())
            }
        }
    }};
);

fn can_skip_quoting(ch: char) -> bool {
    ('a'..='z').contains(&ch)
        || ('A'..='Z').contains(&ch)
        || ('0'..='9').contains(&ch)
        || ch == '-'
        || ch == '.'
        || ch == '_'
        || ch == '/'
        || ch == '@'
        || ch == '^'
        || ch == '+'
}

fn optionally_quote(input: &str, force: bool) -> Cow<str> {
    if !force && input.chars().all(can_skip_quoting) {
        input.into()
    } else {
        format!("\"{}\"", input.escape_debug()).into()
    }
}

impl<'a, W> slog::Serializer for LogfmtSerializer<'a, W>
where
    W: io::Write,
{
    fn emit_usize(&mut self, key: slog::Key, val: usize) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_isize(&mut self, key: slog::Key, val: isize) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_bool(&mut self, key: slog::Key, val: bool) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_char(&mut self, key: slog::Key, val: char) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u8(&mut self, key: slog::Key, val: u8) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i8(&mut self, key: slog::Key, val: i8) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u16(&mut self, key: slog::Key, val: u16) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i16(&mut self, key: slog::Key, val: i16) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u32(&mut self, key: slog::Key, val: u32) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i32(&mut self, key: slog::Key, val: i32) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_f32(&mut self, key: slog::Key, val: f32) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u64(&mut self, key: slog::Key, val: u64) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i64(&mut self, key: slog::Key, val: i64) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_f64(&mut self, key: slog::Key, val: f64) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_u128(&mut self, key: slog::Key, val: u128) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_i128(&mut self, key: slog::Key, val: i128) -> Result<(), Error> {
        w!(self, key, val)
    }

    fn emit_str(&mut self, key: slog::Key, val: &str) -> Result<(), Error> {
        let val = optionally_quote(val, self.force_quotes);
        w!(self, key, &*val)
    }

    fn emit_unit(&mut self, key: slog::Key) -> Result<(), Error> {
        w!(self, key, "()")
    }

    fn emit_none(&mut self, key: slog::Key) -> Result<(), Error> {
        w!(self, key, "None")
    }

    fn emit_arguments<'b>(&mut self, key: slog::Key, val: &Arguments<'b>) -> Result<(), Error> {
        let val = format!("{}", val);
        let val = optionally_quote(&val, self.force_quotes);
        w!(self, key, &*val)
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
        let prefix = self.options.prefix;
        prefix(&mut *io, record)?;

        let mut serializer = LogfmtSerializer {
            io: &mut *io,
            first: true,
            force_quotes: self.options.force_quotes,
            redactor: self.options.redactor,
        };
        if self.options.print_level {
            let lvl = o!("level" => record.level().as_short_str());
            lvl.serialize(record, &mut serializer)?;
        }
        if self.options.print_msg {
            record.msg().serialize(
                record,
                #[allow(clippy::useless_conversion)] // necessary for dynamic-keys
                "msg".into(),
                &mut serializer,
            )?;
        }
        if self.options.print_tag {
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
