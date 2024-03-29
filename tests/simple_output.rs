use core::fmt;
use slog::{debug, o, Drain, Error, Logger, Serializer, KV};
use slog_logfmt::{Logfmt, Redaction};
use std::fmt::Arguments;
use std::io;
use std::io::Cursor;
use std::str::from_utf8;
use std::sync::{Arc, Mutex};
use test_case::test_case;

#[derive(Clone, Default)]
struct LogCapture(Arc<Mutex<Cursor<Vec<u8>>>>);

impl LogCapture {
    fn snapshot_buf(&self) -> Vec<u8> {
        let guard = self.0.lock().unwrap();
        (*guard).get_ref().clone()
    }

    fn snapshot_str(&self) -> String {
        let buf = self.snapshot_buf();
        from_utf8(&buf).unwrap().to_string()
    }
}

impl io::Write for LogCapture {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        let mut guard = self.0.lock().unwrap();
        (*guard).write(buf)
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        let mut guard = self.0.lock().unwrap();
        (*guard).flush()
    }
}

struct DebugRepr(char, usize);

impl fmt::Debug for DebugRepr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..self.1 {
            write!(f, "{}", self.0)?;
        }
        Ok(())
    }
}

#[test]
fn write_stuff() {
    let output = LogCapture::default();
    let drain = Logfmt::new(output.clone()).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!("logger" => "tests"));
    debug!(logger, #"testing_tag", "hi there"; "backslashes" => ?DebugRepr('\\', 2), "single_quotes" => ?DebugRepr('\'', 2), "double_quotes" => ?DebugRepr('"', 3));

    drop(logger);

    assert_eq!(
        output.snapshot_str(),
        "DEBG | #testing_tag\thi there\tlogger=tests double_quotes=\"\\\"\\\"\\\"\" single_quotes=\"\\\'\\\'\" backslashes=\"\\\\\\\\\"\n"
    );
}

#[test]
fn force_quotes() {
    let output = LogCapture::default();
    let drain = Logfmt::new(output.clone()).force_quotes().build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!("logger" => "tests"));
    debug!(logger, #"testing_tag", "hi there"; "foo" => "bar'baz\"");

    drop(logger);

    assert_eq!(
        output.snapshot_str(),
        "DEBG | #testing_tag\thi there\tlogger=\"tests\" foo=\"bar\\'baz\\\"\"\n"
    );
}

#[test_case(r#"foo"#, r#"f="foo""#;
            "a plain string")]
#[test_case(r#"hi="there""#, r#"f="hi=\"there\"""#;
            "something that looks like a field")]
#[test_case(r#" "hi" "#, r#"f=" \"hi\" ""#;
            "spaces and quotes")]
#[test_case(r#"/foo/bar/baz"#, r#"f="/foo/bar/baz""#;
            "pathname")]
#[test_case(2_i128, r#"f="2""#)]
#[test_case(12_u64, r#"f="12""#)]
#[test_case(12_i64, r#"f="12""#)]
fn field_formatting_with_quoting(str_repr: impl slog::Value, expected: &str) {
    let output = LogCapture::default();
    let drain = Logfmt::new(output.clone())
        .force_quotes()
        .no_prefix()
        .print_level(false)
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!());

    debug!(logger, ""; "f" => str_repr);
    drop(logger);
    assert_eq!(output.snapshot_str().trim_end(), expected);
}

#[test_case(r#"foo"#, r#"f=foo"#;
            "a plain string")]
#[test_case(r#"hi="there""#, r#"f="hi=\"there\"""#;
            "something that looks like a field")]
#[test_case(r#" "hi" "#, r#"f=" \"hi\" ""#;
            "spaces and quotes")]
#[test_case(r#"/foo/bar/baz"#, r#"f=/foo/bar/baz"#;
            "pathname")]
#[test_case(2_i128, r#"f=2"#)]
#[test_case(12_u64, r#"f=12"#)]
#[test_case(12_i64, r#"f=12"#)]
fn field_formatting_without_quoting(str_repr: impl slog::Value, expected: &str) {
    let output = LogCapture::default();
    let drain = Logfmt::new(output.clone())
        .no_prefix()
        .print_level(false)
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!());

    debug!(logger, ""; "f" => str_repr);
    drop(logger);
    assert_eq!(output.snapshot_str().trim_end(), expected);
}

struct PrefixSerializer<W: io::Write> {
    io: W,
}

impl<W: io::Write> Serializer for PrefixSerializer<W> {
    fn emit_arguments<'a>(&mut self, _key: &'static str, val: &Arguments<'a>) -> Result<(), Error> {
        self.io.write_fmt(*val)?;
        Ok(())
    }
}

#[test]
fn prefixed_stuff() {
    let output = LogCapture::default();
    let drain = Logfmt::new(output.clone())
        .set_prefix(move |mut io, rec| {
            write!(&mut io, "[")?;
            {
                let mut serializer = PrefixSerializer { io: &mut io };
                rec.kv().serialize(&rec, &mut serializer)?;
            }
            write!(&mut io, "] ")?;
            Ok(())
        })
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!("logger" => "tests"));
    debug!(logger, #"tag", "hi there"; "foo" => "9029292");

    drop(logger);
    assert_eq!(
        output.snapshot_str(),
        "[9029292] logger=tests foo=9029292\n"
    );
}

#[test]
fn redactions() {
    let output = LogCapture::default();
    let drain = Logfmt::new(output.clone())
        .redact(|&key| match key {
            "foo" => Redaction::Skip,
            "secret" => Redaction::Redact(|_val| format_args!("***")),
            _ => Redaction::Plain,
        })
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!("logger" => "tests"));
    debug!(logger, #"tag", "hi there"; "foo" => "9029292", "secret" => 900);

    drop(logger);
    assert_eq!(
        output.snapshot_str(),
        "DEBG | #tag\thi there\tlogger=tests secret=\"***\"\n"
    );
}
