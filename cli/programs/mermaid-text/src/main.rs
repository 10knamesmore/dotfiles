mod border;
mod canvas;
mod charts;
mod color;
mod graphics;
mod image_output;
mod node_styles;
mod options;
mod render;
mod source;
mod stream;
mod text;

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use border::BorderCorners;
use clap::Parser;
use options::Options;
use render::Diagram;

const MAX_SOURCE_BYTES: usize = 256 * 1024;

#[derive(Parser)]
#[command(
    version,
    about,
    after_help = "Example:\n  mermaid-text --example flowchart --border-corners rounded"
)]
struct Args {
    /// Preferred layout width in terminal display columns (1–4096)
    #[arg(long, value_name = "COLUMNS", value_parser = clap::value_parser!(u16).range(1..=4096))]
    width: Option<u16>,

    /// Draw borders and arrows with ASCII; preserve label text
    #[arg(long)]
    ascii: bool,

    /// Disable colors declared in Mermaid source (also honors a nonempty NO_COLOR)
    #[arg(long)]
    no_color: bool,

    /// Rectangle corners: auto, square, rounded, or four one-column characters (TL TR BL BR)
    #[arg(
        long,
        default_value = "auto",
        env = "MERMAID_TEXT_BORDER_CORNERS",
        allow_hyphen_values = true,
        value_name = "STYLE"
    )]
    border_corners: BorderCorners,

    /// Emit newline-delimited JSON frame, done and error events while reading
    #[arg(long)]
    stream: bool,

    /// Draw a graphical diagram directly in a Kitty graphics terminal
    #[arg(long, group = "image", conflicts_with_all = ["png", "stream", "ascii"])]
    kitty_image: bool,

    /// Write one graphical diagram as PNG bytes to stdout
    #[arg(long, group = "image", conflicts_with_all = ["stream", "ascii"])]
    png: bool,

    /// Output width in pixels (height follows the aspect ratio when omitted)
    #[arg(long, requires = "image", value_name = "PX", value_parser = clap::value_parser!(u16).range(1..=8192))]
    image_width: Option<u16>,

    /// Output height in pixels (with width, fit inside an exact-size canvas)
    #[arg(long, requires = "image", value_name = "PX", value_parser = clap::value_parser!(u16).range(1..=8192))]
    image_height: Option<u16>,

    /// List recognized diagram headers, grouped by family
    #[arg(long, conflicts_with_all = ["stream", "file"])]
    list_types: bool,

    /// Render examples for the named diagram types, or all types when none are named
    #[arg(long, num_args = 0.., value_name = "TYPE", conflicts_with_all = ["file", "list_types"])]
    example: Option<Vec<String>>,

    /// Read a UTF-8 Mermaid source file (default or "-": stdin)
    file: Option<PathBuf>,
}

fn gallery_section(
    ty: &charts::Entry,
    position: usize,
    total: usize,
    options: &Options,
) -> Result<String, String> {
    let mut parsed = source::Source::parse(ty.example)?;
    let mut heading = format!("{position}/{total}  {}", ty.name);
    if let Some(title) = parsed.title.take() {
        heading.push_str("  ");
        heading.push_str(&title);
    }
    let heading = text::wrap(&heading, options.width).join("\n");
    let rule = if options.ascii { "=" } else { "━" }.repeat(options.width);
    let diagram = Diagram::new(parsed, options)?.text()?;
    Ok(format!(
        "{rule}\n{heading}\n{rule}\n\n{}",
        diagram.trim_end_matches('\n')
    ))
}

fn run(args: Args) -> Result<(), String> {
    let images = args.kitty_image || args.png;
    if args.list_types {
        return write_diagram(&charts::list(images));
    }
    let options = Options {
        width: usize::from(args.width.unwrap_or(80)),
        ascii: args.ascii,
        color: !args.no_color && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()),
        corners: args.border_corners,
        config: source::DiagramConfig::default(),
    };
    let terminal = if args.kitty_image
        || (args.png && args.image_width.is_none() && args.image_height.is_none())
    {
        image_output::Terminal::detect()
    } else {
        None
    };
    let size = image_output::image_size(args.image_width, args.image_height, terminal, args.width);
    let output = if args.kitty_image {
        Some(image_output::Output::terminal(terminal)?)
    } else if args.png {
        if io::stdout().is_terminal() {
            return Err(
                "redirect --png to a file or pipe, or use --kitty-image to display it".into(),
            );
        }
        Some(image_output::Output::Png)
    } else {
        None
    };
    let input: Box<dyn Read> = if let Some(names) = args.example {
        let mut selected = charts::select_examples(&names)?;
        if let Some(output) = &output {
            if names.is_empty() {
                selected.retain(|ty| ty.image.is_some());
            }
            if args.png && selected.len() != 1 {
                return Err(
                    "--png --example needs one diagram type (see --png --list-types)".into(),
                );
            }
            for ty in &selected {
                ty.image_renderer()?;
            }
            for ty in selected {
                output.write(
                    &Diagram::parse(ty.example, &options)?.png(size)?,
                    args.width,
                )?;
            }
            return Ok(());
        }
        match selected.as_slice() {
            [ty] => Box::new(ty.example.as_bytes()),
            _ => {
                let sections = selected.iter().enumerate().map(|(index, ty)| {
                    gallery_section(ty, index + 1, selected.len(), &options)
                        .map_err(|err| format!("example {}: {err}", ty.name))
                });
                if args.stream {
                    return stream::gallery(sections, io::stdout().lock());
                }
                return write_diagram(&sections.collect::<Result<Vec<_>, _>>()?.join("\n\n"));
            }
        }
    } else {
        match args.file.as_deref() {
            Some(path) if path != OsStr::new("-") => {
                Box::new(File::open(path).map_err(|err| format!("{}: {err}", path.display()))?)
            }
            _ => Box::new(io::stdin().lock()),
        }
    };
    if args.stream {
        return stream::run(input, io::stdout().lock(), &options);
    }
    let mut bytes = Vec::new();
    input
        .take(MAX_SOURCE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| format!("reading diagram: {err}"))?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err("diagram exceeds 256 KiB".into());
    }
    let source = std::str::from_utf8(&bytes).map_err(|err| format!("expected UTF-8: {err}"))?;
    let diagram = Diagram::parse(source, &options)?;
    match output {
        Some(output) => output.write(&diagram.png(size)?, args.width),
        None => write_diagram(&diagram.text()?),
    }
}

fn write_diagram(diagram: &str) -> Result<(), String> {
    match writeln!(io::stdout().lock(), "{}", diagram.trim_end_matches('\n')) {
        Err(err) if err.kind() != io::ErrorKind::BrokenPipe => {
            Err(format!("writing stdout: {err}"))
        }
        _ => Ok(()),
    }
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("mermaid-text: {err}");
            ExitCode::FAILURE
        }
    }
}
