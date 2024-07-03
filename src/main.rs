use anyhow::Error;
use clap::Parser;
use sourcemap_resolver::{ExtractResult, Extractor};
use std::borrow::Borrow;
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

fn write(
    output: &mut Box<dyn Write>,
    text: &str,
    extract: &Option<ExtractResult>,
    opt: &Opt,
) -> Result<(), Error> {
    writeln!(output, "{}", &text)?;
    if let Some(extract) = extract {
        if let Ok(resolve) =
            sourcemap_resolver::resolve(&extract.path, extract.line, extract.column)
        {
            writeln!(
                output,
                "{}{} {}:{}:{}",
                " ".repeat(extract.range.start as usize),
                opt.indicator,
                resolve.path.to_string_lossy(),
                resolve.line,
                resolve.column
            )?;
        }
    }
    Ok(())
}

fn execute<I>(lines: I, opt: &Opt) -> Result<(), Error>
where
    I: IntoIterator,
    I::Item: Borrow<str>,
{
    let mut output = if let Some(x) = &opt.output {
        Box::new(File::create(x)?) as Box<dyn Write>
    } else {
        Box::new(io::stdout()) as Box<dyn Write>
    };

    let mut extractor = Extractor::new();
    for line in lines {
        if let Some((text, extract)) = extractor.push_line(line.borrow()) {
            write(&mut output, &text, &extract, opt)?;
        }
    }

    for (text, extract) in extractor.end() {
        write(&mut output, &text, &extract, opt)?;
    }

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let opt = Opt::parse();

    if opt.files.is_empty() {
        execute(io::stdin().lines().map(|x| x.unwrap()), &opt)?;
    } else {
        let mut text = String::new();
        for file in &opt.files {
            text.push_str(&fs::read_to_string(file)?);
        }
        execute(text.lines(), &opt)?;
    }

    Ok(())
}
