use slog::{debug, o, Drain, Error, Logger, Serializer, KV};
use slog_logfmt::Logfmt;
use std::fmt::Arguments;
use std::io;
use std::io::stdout;

#[test]
fn write_stuff() {
    let drain = Logfmt::new(stdout()).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let logger = Logger::root(drain, o!("logger" => "tests"));
    debug!(logger, #"tag", "hi there"; "foo" => "bar'baz\"");
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
    let drain = Logfmt::new(stdout())
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
}
