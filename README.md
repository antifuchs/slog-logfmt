# [logfmt](https://brandur.org/logfmt) formatter for [slog](https://github.com/slog-rs/slog/wiki/slog-v2)

This is a pretty straightforward [logfmt](https://brandur.org/logfmt)
formatter with a customizable prefix. The formatter exposed by the
crate is not `Send` or `Sync`, so you'll have to wrap it in
[`slog-async`](https://github.com/slog-rs/async) or similar.

## Status

This project is archived, as I didn't end up using it. For a modern
and useful logfmt library, check out [tracing-logfmt](https://docs.rs/tracing-logfmt/latest/tracing_logfmt/).
