use std::io::{self, BufWriter, Read, Write};

use crate::color;
use crate::{MAX_SOURCE_BYTES, Options};

pub(crate) fn run(
    mut input: impl Read,
    output: impl Write,
    options: &Options,
) -> Result<(), String> {
    let mut output = BufWriter::new(output);
    let mut source = Vec::new();
    let mut buffer = [0; 8192];
    let mut checked_prefix = None;

    let result = loop {
        let remaining = MAX_SOURCE_BYTES - source.len();
        let read_limit = buffer.len().min(remaining + 1);
        let count = match input.read(&mut buffer[..read_limit]) {
            Ok(count) => count,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => break Err(format!("reading diagram: {err}")),
        };
        if count > remaining {
            break Err(format!("diagram exceeds {} KiB", MAX_SOURCE_BYTES / 1024));
        }

        let at_eof = count == 0;
        let end = if at_eof {
            if let Some((len, result)) = checked_prefix.take()
                && len == source.len()
            {
                break result;
            }
            source.len()
        } else {
            let batch = &buffer[..count];
            let start = source.len();
            source.extend_from_slice(batch);
            let Some(last_newline) = batch.iter().rposition(|&byte| byte == b'\n') else {
                continue;
            };
            start + last_newline + 1
        };

        let diagram = std::str::from_utf8(&source[..end])
            .map_err(|err| format!("expected UTF-8: {err}"))
            .and_then(|source| crate::render::Diagram::parse(source, options)?.text());
        let result = match diagram {
            Ok(diagram) => {
                Event::Frame(&diagram).emit(&mut output)?;
                Ok(())
            }
            Err(err) => Err(err),
        };

        if at_eof {
            break result;
        }
        checked_prefix = Some((end, result));
    };

    finish(&mut output, result)
}

pub(crate) fn gallery(
    mut sections: impl Iterator<Item = Result<String, String>>,
    output: impl Write,
) -> Result<(), String> {
    let mut output = BufWriter::new(output);
    let mut gallery = String::new();
    let result = sections.try_for_each(|section| {
        let section = section?;
        if !gallery.is_empty() {
            gallery.push_str("\n\n");
        }
        gallery.push_str(&section);
        Event::Frame(&gallery).emit(&mut output)
    });
    finish(&mut output, result)
}

fn finish(output: &mut impl Write, result: Result<(), String>) -> Result<(), String> {
    match &result {
        Ok(()) => Event::Done,
        Err(message) => Event::Error(message),
    }
    .emit(output)?;
    result
}

enum Event<'a> {
    Frame(&'a str),
    Done,
    Error(&'a str),
}

#[derive(serde::Serialize)]
struct FrameStyle {
    row: usize,
    #[serde(flatten)]
    span: color::Span,
}

impl Event<'_> {
    fn emit(self, output: &mut impl Write) -> Result<(), String> {
        self.write_line(output)
            .and_then(|()| output.flush())
            .map_err(|err| format!("writing stream event: {err}"))
    }

    fn write_line(self, output: &mut impl Write) -> io::Result<()> {
        match self {
            Self::Frame(diagram) => {
                output.write_all(b"{\"type\":\"frame\",\"lines\":[")?;
                let mut styles = Vec::new();
                for (row, line) in diagram.lines().enumerate() {
                    if row != 0 {
                        output.write_all(b",")?;
                    }
                    if line.contains('\x1b') {
                        let (text, spans) = color::decode_line(line).map_err(io::Error::other)?;
                        serde_json::to_writer(&mut *output, &text)?;
                        styles.extend(spans.into_iter().map(|span| FrameStyle { row, span }));
                    } else {
                        serde_json::to_writer(&mut *output, line)?;
                    }
                }
                output.write_all(b"]")?;
                if !styles.is_empty() {
                    output.write_all(b",\"styles\":")?;
                    serde_json::to_writer(&mut *output, &styles)?;
                }
                output.write_all(b"}\n")
            }
            Self::Done => output.write_all(b"{\"type\":\"done\"}\n"),
            Self::Error(message) => {
                output.write_all(b"{\"type\":\"error\",\"message\":")?;
                serde_json::to_writer(&mut *output, message)?;
                output.write_all(b"}\n")
            }
        }
    }
}
