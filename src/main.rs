use anyhow::Error;
use clap::Parser;
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

// ---------------------------------------------------------------------------------------------------------------------
// Opt
// ---------------------------------------------------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Opt {
    pub files: Vec<PathBuf>,

    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    #[arg(long, default_value = "^--")]
    pub indicator: String,
}

// ---------------------------------------------------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------------------------------------------------

pub fn main() -> Result<(), Error> {
    let opt = Opt::parse();

    let mut text = String::new();

    if opt.files.is_empty() {
        for line in io::stdin().lines() {
            let line = line?;
            text.push_str(&line);
            text.push('\n');
        }
    } else {
        for file in &opt.files {
            text.push_str(&fs::read_to_string(file)?);
        }
    }

    let extracts = sourcemap_resolver::extract(&text);

    let mut resolves = VecDeque::new();

    for extract in extracts {
        if let Ok(resolve) =
            sourcemap_resolver::resolve(&extract.path, extract.line, extract.column)
        {
            resolves.push_back((extract.range, resolve));
        }
    }

    let mut beg = 0;
    let mut end = 0;

    let mut output = if let Some(x) = &opt.output {
        Box::new(File::create(x)?) as Box<dyn Write>
    } else {
        Box::new(io::stdout()) as Box<dyn Write>
    };

    while end != text.len() {
        if let Some(x) = text[end..].find('\n') {
            end += x + 1;
        } else {
            end = text.len()
        }

        write!(output, "{}", &text[beg..end])?;

        let insert = resolves
            .front()
            .map(|x| beg as u32 <= x.0.end && x.0.end < end as u32)
            .unwrap_or(false);
        if insert {
            let head = resolves.pop_front().unwrap();
            let column = (head.0.start as usize).saturating_sub(beg);
            writeln!(
                output,
                "{}{} {}:{}:{}",
                " ".repeat(column),
                opt.indicator,
                head.1.path.to_string_lossy(),
                head.1.line,
                head.1.column
            )?;
        }

        beg = end;
    }

    Ok(())
}
