use slog::{debug, o, Drain, Error, Logger, Serializer, KV};
use slog_logfmt::Logfmt;
use std::fmt::Arguments;
use std::io;
use std::io::Cursor;
use std::str::from_utf8;
use std::sync::{Arc, Mutex};

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

#[test]
fn write_stuff() {
    let output = LogCapture::default();
    let drain = Logfmt::new(output.clone()).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!("logger" => "tests"));
    debug!(logger, #"testing_tag", "hi there"; "foo" => "bar'baz\"");

    drop(logger);
    assert_eq!(
        output.snapshot_str(),
        "DEBG | #testing_tag\thi there\tlogger=tests foo=\"bar\\\'baz\\\"\"\n"
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
        "DEBG | #testing_tag\thi there\tlogger=\"tests\" foo=\"bar\\\'baz\\\"\"\n"
    );
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
        .skip_fields(vec!["foo"])
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!("logger" => "tests"));
    debug!(logger, #"tag", "hi there"; "foo" => "9029292");

    drop(logger);
    assert_eq!(output.snapshot_str(), "[9029292] logger=tests\n");
}
