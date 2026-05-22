use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use algosketch_core::ir::Item;
use algosketch_core::parser::{LanguageParser, PythonParser};
use algosketch_core::renderer::{ExplainRenderer, PseudoRenderer};
use algosketch_core::{NaturalLang, PseudoError, SourceLang};
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

    /// Disable pseudocode output.
    #[arg(long = "no-pseudo")]
    no_pseudo: bool,

    /// Disable explanation output.
    #[arg(long = "no-explain")]
    no_explain: bool,

    /// Natural language for explanations: zh | en | auto.
    #[arg(long = "lang", value_enum, default_value_t = NaturalLangArg::Auto)]
    lang: NaturalLangArg,
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

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum NaturalLangArg {
    Zh,
    En,
    Auto,
}

fn resolve_natural_lang(arg: NaturalLangArg) -> NaturalLang {
    match arg {
        NaturalLangArg::Zh => NaturalLang::Zh,
        NaturalLangArg::En => NaturalLang::En,
        NaturalLangArg::Auto => detect_locale(),
    }
}

fn detect_locale() -> NaturalLang {
    if let Ok(val) = std::env::var("PSEUDOCODE_LANG") {
        if val.starts_with("zh") {
            return NaturalLang::Zh;
        }
        if val.starts_with("en") {
            return NaturalLang::En;
        }
    }

    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            if val.starts_with("zh") || val.starts_with("zh_") {
                return NaturalLang::Zh;
            }
        }
    }

    NaturalLang::En
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

    let natural_lang = resolve_natural_lang(cli.lang);
    let pseudo_renderer = PseudoRenderer {
        indent_width: cli.indent,
    };
    let explain_renderer = ExplainRenderer::new(natural_lang);

    let show_pseudo = !cli.no_pseudo;
    let show_explain = !cli.no_explain;

    let mut sections = Vec::new();

    for item in &module.items {
        if let Item::Function(f) = item {
            let mut func_output = String::new();

            if cli.format == OutFormat::Md {
                func_output.push_str(&format!("## {}\n\n", f.name));
            }

            if show_pseudo {
                let pseudo = pseudo_renderer.render_function(f);
                if cli.format == OutFormat::Md {
                    if show_explain {
                        func_output.push_str("### Pseudocode\n\n");
                    }
                    func_output.push_str(&format!("```text\n{pseudo}```\n\n"));
                } else {
                    func_output.push_str(&pseudo);
                }
            }

            if show_explain {
                let explain = explain_renderer.render_function(f);
                if cli.format == OutFormat::Md {
                    let title = match natural_lang {
                        NaturalLang::Zh => "### 解释\n\n",
                        NaturalLang::En => "### Explanation\n\n",
                    };
                    func_output.push_str(title);
                }
                func_output.push_str(&explain);
                func_output.push('\n');
            }

            sections.push(func_output);
        }
    }

    let separator = if cli.format == OutFormat::Md {
        "\n\n"
    } else {
        "\n"
    };
    let output = sections
        .into_iter()
        .map(|s| s.trim_end_matches('\n').to_string())
        .collect::<Vec<_>>()
        .join(separator);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_explicit_natural_lang() {
        assert_eq!(resolve_natural_lang(NaturalLangArg::Zh), NaturalLang::Zh);
        assert_eq!(resolve_natural_lang(NaturalLangArg::En), NaturalLang::En);
    }
}
