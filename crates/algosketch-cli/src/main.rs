use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use algosketch_core::parser::{LanguageParser, PythonParser};
use algosketch_core::renderer::PseudoRenderer;
use algosketch_core::{PseudoError, SourceLang};
use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "algosketch",
    version,
    about = "Turn real source code into language-neutral pseudocode.",
    long_about = None,
)]
struct Cli {
    /// Path to source file, or "-" for stdin.
    input: String,

    /// Source language (auto-detected from file extension if omitted).
    #[arg(short = 'l', long = "source-lang", value_enum)]
    source_lang: Option<CliLang>,

    /// Output format: md | text.
    #[arg(long = "format", value_enum, default_value_t = OutFormat::Md)]
    format: OutFormat,

    /// Write output to FILE instead of stdout.
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Indent width.
    #[arg(long = "indent", default_value_t = 2)]
    indent: usize,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliLang {
    Python,
    Java,
    Cpp,
}

impl From<CliLang> for SourceLang {
    fn from(value: CliLang) -> Self {
        match value {
            CliLang::Python => SourceLang::Python,
            CliLang::Java => SourceLang::Java,
            CliLang::Cpp => SourceLang::Cpp,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum OutFormat {
    Md,
    Text,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(exit_code_for(&e))
        }
    }
}

fn run(cli: Cli) -> Result<(), PseudoError> {
    let source_lang = resolve_source_lang(&cli)?;
    let source = read_source(&cli.input)?;

    let module = match source_lang {
        SourceLang::Python => PythonParser::new().parse(&source)?,
        SourceLang::Java | SourceLang::Cpp => {
            return Err(PseudoError::UnsupportedLanguage(
                source_lang.as_str().to_string(),
            ));
        }
    };

    let pseudocode = PseudoRenderer {
        indent_width: cli.indent,
    }
    .render_module(&module);

    let output = if cli.format == OutFormat::Md {
        format!("### Pseudocode\n\n```text\n{pseudocode}```\n")
    } else {
        pseudocode
    };

    if let Some(path) = cli.output {
        fs::write(path, output)?;
    } else {
        write_to_stdout(&output)?;
    }

    Ok(())
}

fn write_to_stdout(text: &str) -> Result<(), PseudoError> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    match handle.write_all(text.as_bytes()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(PseudoError::Io(e)),
    }
}

fn resolve_source_lang(cli: &Cli) -> Result<SourceLang, PseudoError> {
    if let Some(lang) = cli.source_lang {
        return Ok(lang.into());
    }

    if cli.input == "-" {
        return Err(PseudoError::UnknownLanguage);
    }

    let ext = Path::new(&cli.input)
        .extension()
        .and_then(|s| s.to_str())
        .ok_or(PseudoError::UnknownLanguage)?;

    SourceLang::from_extension(ext).ok_or(PseudoError::UnknownLanguage)
}

fn read_source(input: &str) -> Result<String, PseudoError> {
    if input == "-" {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        Ok(source)
    } else {
        Ok(fs::read_to_string(input)?)
    }
}

fn exit_code_for(e: &PseudoError) -> u8 {
    match e {
        PseudoError::UnsupportedLanguage(_) | PseudoError::UnknownLanguage => 1,
        PseudoError::Io(_) => 1,
        PseudoError::Parse { .. } => 2,
        PseudoError::Internal(_) => 3,
    }
}
