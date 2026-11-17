//! Command-line tools for Intent layouts.
//!
//! Everything lives behind [`run`], which takes its arguments and output
//! streams as parameters so the whole tool can be tested without spawning a
//! process.

pub mod coverage;

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use intent_core::layout::{Compiled, Layout};
use intent_core::{Action, Chord, Command, Effect, EffortModel, Engine, Hand, Plane};

/// Exit code: everything went fine.
pub const OK: u8 = 0;
/// Exit code: the layout or the request has a problem.
pub const FAILED: u8 = 1;
/// Exit code: the tool was called wrongly or a file could not be read.
pub const USAGE: u8 = 2;

#[derive(Debug, Parser)]
#[command(name = "intent", version, about = "Tools for Intent chord layouts")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Validate a layout file and report every problem in it
    Check {
        /// Path to the layout file
        layout: PathBuf,
        /// Treat warnings as errors
        #[arg(long)]
        strict: bool,
    },
    /// Summarise how a layout uses the chord space
    Stats {
        /// Path to the layout file
        layout: PathBuf,
        /// How many of the hardest concepts to list
        #[arg(long, default_value_t = 10)]
        top: usize,
    },
    /// Apply chords such as I-M or -T and show the resulting buffer
    Sim {
        /// Path to the layout file
        layout: PathBuf,
        /// Chords in PRMIT-TIMRP notation
        #[arg(required = true, allow_hyphen_values = true)]
        chords: Vec<String>,
    },
    /// Show the strokes needed to enter tokens, words and commands
    How {
        /// Path to the layout file
        layout: PathBuf,
        /// A token (FIX), a word to spell (login), a command (<send>),
        /// or =TEXT to force spelling
        #[arg(required = true, allow_hyphen_values = true)]
        items: Vec<String>,
    },
    /// Show what a user would enter for each line of a text file
    Encode {
        /// Path to the layout file
        layout: PathBuf,
        /// A text file with one sentence per line
        corpus: PathBuf,
        /// Print JSON lines with the decoder messages for each sentence
        #[arg(long)]
        messages: bool,
    },
    /// Measure how much of a text corpus the layout covers with concepts
    Coverage {
        /// Path to the layout file
        layout: PathBuf,
        /// A text file: one utterance per line, or `word count` lines with --frequency
        corpus: PathBuf,
        /// Treat each line as a word and how often it occurs
        #[arg(long)]
        frequency: bool,
        /// How many of the most frequent spelled words to list
        #[arg(long, default_value_t = 30)]
        top: usize,
    },
}

/// Runs the tool and returns its exit code.
pub fn run<I, T>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(error) => {
            // Help and version are requested output, not failures.
            if error.use_stderr() {
                let _ = write!(err, "{}", error.render());
                return USAGE;
            }
            let _ = write!(out, "{}", error.render());
            return OK;
        }
    };
    let result = match &cli.command {
        Cmd::Check { layout, strict } => check(layout, *strict, out, err),
        Cmd::Stats { layout, top } => stats(layout, *top, out, err),
        Cmd::Sim { layout, chords } => sim(layout, chords, out, err),
        Cmd::How { layout, items } => how(layout, items, out, err),
        Cmd::Coverage {
            layout,
            corpus,
            frequency,
            top,
        } => coverage_command(layout, corpus, *frequency, *top, out, err),
        Cmd::Encode {
            layout,
            corpus,
            messages,
        } => encode_command(layout, corpus, *messages, out, err),
    };
    result.unwrap_or_else(|error| {
        let _ = writeln!(err, "error: {error}");
        USAGE
    })
}

/// Reads and compiles a layout. On failure the problem has already been
/// written to `err` and the exit code to use is returned.
fn load(path: &Path, err: &mut dyn Write) -> io::Result<Result<Compiled, u8>> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            writeln!(err, "error: cannot read {}: {error}", path.display())?;
            return Ok(Err(USAGE));
        }
    };
    match Layout::from_toml(&source) {
        Ok(compiled) => Ok(Ok(compiled)),
        Err(error) => {
            writeln!(err, "{error}")?;
            writeln!(err, "{}: the layout has errors", path.display())?;
            Ok(Err(FAILED))
        }
    }
}

fn check(path: &Path, strict: bool, out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let compiled = match load(path, err)? {
        Ok(compiled) => compiled,
        Err(code) => return Ok(code),
    };
    for warning in &compiled.warnings {
        writeln!(err, "{warning}")?;
    }
    let meta = compiled.layout.meta();
    if strict && !compiled.warnings.is_empty() {
        writeln!(
            err,
            "{}: {} warning(s), and --strict treats warnings as errors",
            path.display(),
            compiled.warnings.len()
        )?;
        return Ok(FAILED);
    }
    writeln!(
        out,
        "{} {}: ok, {} of {} chords assigned",
        meta.name,
        meta.version,
        compiled.layout.entries().len(),
        Chord::COUNT
    )?;
    Ok(OK)
}

fn stats(path: &Path, top: usize, out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let layout = match load(path, err)? {
        Ok(compiled) => compiled.layout,
        Err(code) => return Ok(code),
    };
    let model = EffortModel::default();
    let meta = layout.meta();
    writeln!(out, "{} {}", meta.name, meta.version)?;
    if !meta.description.is_empty() {
        writeln!(out, "{}", meta.description)?;
    }

    writeln!(out)?;
    writeln!(out, "{:<12} {:>5} {:>9}", "plane", "used", "capacity")?;
    for plane in Plane::ALL {
        writeln!(
            out,
            "{:<12} {:>5} {:>9}",
            plane.name(),
            layout.plane_usage(plane),
            plane.capacity()
        )?;
    }
    writeln!(
        out,
        "{:<12} {:>5} {:>9}",
        "total",
        layout.entries().len(),
        Chord::COUNT
    )?;

    let mut concepts: Vec<(f64, Chord, &str)> = Vec::new();
    let mut letters: Vec<f64> = Vec::new();
    let mut texts = 0_usize;
    let mut commands = 0_usize;
    for entry in layout.entries() {
        let effort = model.chord(entry.chord);
        match &entry.action {
            Action::Concept(concept) => concepts.push((effort, entry.chord, &concept.token)),
            Action::Text(_) => {
                texts += 1;
                if matches!(entry.chord.plane(), Some(Plane::Spell(_))) {
                    letters.push(effort);
                }
            }
            Action::Command(_) => commands += 1,
        }
    }

    writeln!(out)?;
    writeln!(
        out,
        "{} concepts, {texts} text chords, {commands} command chords",
        concepts.len()
    )?;
    let concept_efforts: Vec<f64> = concepts.iter().map(|(effort, ..)| *effort).collect();
    for (label, efforts) in [("concepts", &concept_efforts), ("letters", &letters)] {
        if let Some(summary) = Summary::of(efforts) {
            writeln!(
                out,
                "effort of {label}: min {:.1}, mean {:.1}, max {:.1}",
                summary.min, summary.mean, summary.max
            )?;
        }
    }

    if top > 0 && !concepts.is_empty() {
        concepts.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        writeln!(out)?;
        writeln!(out, "hardest concepts:")?;
        for (effort, chord, token) in concepts.iter().take(top) {
            writeln!(out, "  {effort:>4.1}  {:<12} {token}", chord.to_string())?;
        }
    }
    Ok(OK)
}

struct Summary {
    min: f64,
    mean: f64,
    max: f64,
}

impl Summary {
    fn of(values: &[f64]) -> Option<Summary> {
        let first = *values.first()?;
        let (min, max, sum) = values
            .iter()
            .fold((first, first, 0.0), |(min, max, sum), value| {
                (min.min(*value), max.max(*value), sum + value)
            });
        #[allow(clippy::cast_precision_loss)] // counts here are far below 2^52
        let mean = sum / values.len() as f64;
        Some(Summary { min, mean, max })
    }
}

fn sim(path: &Path, chords: &[String], out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let layout = match load(path, err)? {
        Ok(compiled) => compiled.layout,
        Err(code) => return Ok(code),
    };
    let mut parsed = Vec::with_capacity(chords.len());
    for text in chords {
        match text.parse::<Chord>() {
            Ok(chord) => parsed.push(chord),
            Err(error) => {
                writeln!(err, "error: \"{text}\" is not a chord: {error}")?;
                return Ok(USAGE);
            }
        }
    }
    let mut engine = Engine::new(layout);
    for chord in parsed {
        let does = describe(engine.layout(), chord);
        let effect = engine.stroke(chord);
        let line = format!(
            "{:<12} {}  {:<16} {}",
            chord.to_string(),
            chord.diagram(),
            does,
            effect_note(&effect)
        );
        writeln!(out, "{}", line.trim_end())?;
    }
    writeln!(out, "buffer: {}", engine.buffer().utterance)?;
    Ok(OK)
}

fn describe(layout: &Layout, chord: Chord) -> String {
    layout
        .lookup(chord)
        .map_or_else(|| "(nothing)".to_owned(), |entry| entry.action.to_string())
}

fn effect_note(effect: &Effect) -> String {
    match effect {
        Effect::Updated => String::new(),
        Effect::Ignored => "(nothing to do)".to_owned(),
        Effect::Send(utterance) => format!("sends: {utterance}"),
        Effect::Key(command) => format!("presses the {command} key"),
        Effect::Unmapped(_) => "(not in this layout)".to_owned(),
    }
}

fn how(path: &Path, items: &[String], out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let layout = match load(path, err)? {
        Ok(compiled) => compiled.layout,
        Err(code) => return Ok(code),
    };
    let strokes = match plan(&layout, items) {
        Ok(strokes) => strokes,
        Err(message) => {
            writeln!(err, "error: {message}")?;
            return Ok(FAILED);
        }
    };

    let model = EffortModel::default();
    let mut engine = Engine::new(layout);
    let mut total_effort = 0.0;
    let mut total_keys = 0;
    let mut sent = None;
    writeln!(
        out,
        "{:>3}  {:<12} {:<11}  {:<16} {:>6}",
        "#", "chord", "keys", "does", "effort"
    )?;
    for (number, chord) in strokes.iter().enumerate() {
        let effort = model.chord(*chord);
        total_effort += effort;
        total_keys += chord.len();
        writeln!(
            out,
            "{:>3}  {:<12} {}  {:<16} {effort:>6.1}",
            number + 1,
            chord.to_string(),
            chord.diagram(),
            describe(engine.layout(), *chord),
        )?;
        if let Effect::Send(utterance) = engine.stroke(*chord) {
            sent = Some(utterance);
        }
    }
    writeln!(
        out,
        "{} strokes, {total_keys} key presses, effort {total_effort:.1}",
        strokes.len()
    )?;
    match sent {
        Some(utterance) => writeln!(out, "sends: {utterance}")?,
        None => writeln!(out, "buffer: {}", engine.buffer().utterance)?,
    }
    Ok(OK)
}

fn encode_command(
    path: &Path,
    corpus: &Path,
    messages: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let layout = match load(path, err)? {
        Ok(compiled) => compiled.layout,
        Err(code) => return Ok(code),
    };
    let text = match fs::read_to_string(corpus) {
        Ok(text) => text,
        Err(error) => {
            writeln!(err, "error: cannot read {}: {error}", corpus.display())?;
            return Ok(USAGE);
        }
    };
    let matcher = coverage::Matcher::new(&layout);
    let options = intent_core::prompt::PromptOptions::default();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let utterance = coverage::encode(&matcher, line);
        if messages {
            let chat: Vec<serde_json::Value> = intent_core::prompt::messages(&utterance, &[], &options)
                .into_iter()
                .map(|message| serde_json::json!({ "role": message.role.name(), "content": message.content }))
                .collect();
            let record = serde_json::json!({ "text": line, "wire": utterance.to_string(), "messages": chat });
            writeln!(out, "{record}")?;
        } else {
            writeln!(out, "{utterance}")?;
        }
    }
    Ok(OK)
}

fn coverage_command(
    path: &Path,
    corpus: &Path,
    frequency: bool,
    top: usize,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let layout = match load(path, err)? {
        Ok(compiled) => compiled.layout,
        Err(code) => return Ok(code),
    };
    let text = match fs::read_to_string(corpus) {
        Ok(text) => text,
        Err(error) => {
            writeln!(err, "error: cannot read {}: {error}", corpus.display())?;
            return Ok(USAGE);
        }
    };
    let matcher = coverage::Matcher::new(&layout);
    let mut report = coverage::Report::default();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if frequency {
            let parsed = line
                .rsplit_once(char::is_whitespace)
                .and_then(|(word, count)| Some((word.trim(), count.parse::<u64>().ok()?)));
            let Some((word, count)) = parsed else {
                writeln!(
                    err,
                    "error: {} line {}: expected `word count`, found \"{line}\"",
                    corpus.display(),
                    number + 1
                )?;
                return Ok(USAGE);
            };
            report.add_frequency(&matcher, word, count);
        } else {
            report.add_utterance(&matcher, line);
        }
    }

    let words = report.words;
    let pct = |part| coverage::percent(part, words);
    writeln!(
        out,
        "{} {} on {}",
        layout.meta().name,
        layout.meta().version,
        corpus.display()
    )?;
    writeln!(out)?;
    writeln!(out, "{:<10} {:>12} {:>7}", "words", "count", "share")?;
    for (label, count) in [
        ("concept", report.concept),
        ("grammar", report.grammar),
        ("spelled", report.spelled),
    ] {
        writeln!(out, "{label:<10} {count:>12} {:>7}", pct(count))?;
    }
    writeln!(out, "{:<10} {words:>12}", "total")?;
    if report.unenterable > 0 {
        writeln!(
            out,
            "{} spelled words contain a character no chord types",
            report.unenterable
        )?;
    }
    writeln!(out)?;
    writeln!(
        out,
        "no spelling needed for {} of words ({} concept + grammar)",
        pct(report.concept + report.grammar),
        report.concept + report.grammar
    )?;
    if report.utterances > 0 {
        writeln!(
            out,
            "{} of {} utterances need no spelling at all ({})",
            report.utterances_without_spelling,
            report.utterances,
            coverage::percent(report.utterances_without_spelling, report.utterances)
        )?;
        #[allow(clippy::cast_precision_loss)] // corpus counts are far below 2^52
        let per_word = |total: u64| total as f64 / words.max(1) as f64;
        writeln!(
            out,
            "{:.2} strokes per word, against {:.2} keystrokes per word to type the same words",
            per_word(report.strokes),
            per_word(report.keystrokes)
        )?;
    }
    if top > 0 && !report.missing.is_empty() {
        writeln!(out)?;
        writeln!(out, "most frequent spelled words:")?;
        for (word, count) in report.top_missing(top) {
            writeln!(out, "  {count:>10} {:>6}  {word}", pct(count))?;
        }
    }
    Ok(OK)
}

/// Works out the strokes for a list of tokens, words and commands.
///
/// Words are spelled with alternating hands, and a space stroke is added
/// between two words in a row.
fn plan(layout: &Layout, items: &[String]) -> Result<Vec<Chord>, String> {
    let model = EffortModel::default();
    let easiest = |chords: Vec<Chord>| {
        chords
            .into_iter()
            .min_by(|a, b| model.chord(*a).total_cmp(&model.chord(*b)))
    };
    let mut strokes = Vec::new();
    let mut after_word = false;
    let mut next_hand = Hand::Left;

    for item in items {
        if let Some(name) = item
            .strip_prefix('<')
            .and_then(|rest| rest.strip_suffix('>'))
        {
            let command =
                Command::from_name(name).ok_or_else(|| format!("unknown command <{name}>"))?;
            let chord = easiest(layout.chords_for_command(command))
                .ok_or_else(|| format!("this layout has no chord for <{name}>"))?;
            strokes.push(chord);
            after_word = false;
            continue;
        }
        if let Some(chord) = layout.chord_for_token(item) {
            strokes.push(chord);
            after_word = false;
            continue;
        }
        let word = match item.strip_prefix('=') {
            Some(forced) => forced,
            None if looks_like_token(item) => {
                return Err(format!(
                    "{item} is not a token in this layout (write ={item} to spell it out)"
                ));
            }
            None => item,
        };
        if word.is_empty() {
            return Err("nothing to spell after '='".to_owned());
        }
        if after_word {
            let space = easiest(layout.chords_for_command(Command::Space)).ok_or(
                "this layout has no chord for <space>, so two words cannot follow each other",
            )?;
            strokes.push(space);
        }
        for character in word.chars() {
            let options = layout.chords_for_text(&character.to_string());
            let on_next_hand = options
                .iter()
                .copied()
                .find(|chord| spelling_hand(*chord) == Some(next_hand));
            let chord = on_next_hand
                .or_else(|| easiest(options))
                .ok_or_else(|| format!("this layout cannot type '{character}' (in \"{word}\")"))?;
            if let Some(hand) = spelling_hand(chord) {
                next_hand = hand.other();
            }
            strokes.push(chord);
        }
        after_word = true;
    }
    Ok(strokes)
}

/// The hand whose fingers type a spelling chord.
fn spelling_hand(chord: Chord) -> Option<Hand> {
    match chord.plane() {
        Some(Plane::Spell(hand) | Plane::SpellShift(hand)) => Some(hand),
        _ => None,
    }
}

/// Whether text is shaped like a concept token: `FIX`, `WITH_TESTS`.
fn looks_like_token(text: &str) -> bool {
    text.len() > 1
        && text.starts_with(|c: char| c.is_ascii_uppercase())
        && text
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const V0: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../layouts/intent-v0.toml");

    struct Outcome {
        code: u8,
        out: String,
        err: String,
    }

    fn intent(args: &[&str]) -> Outcome {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run(
            std::iter::once("intent").chain(args.iter().copied()),
            &mut out,
            &mut err,
        );
        Outcome {
            code,
            out: String::from_utf8(out).expect("utf-8 output"),
            err: String::from_utf8(err).expect("utf-8 errors"),
        }
    }

    /// Writes a layout to a temporary file and returns its path.
    fn layout_file(name: &str, body: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "intent-cli-test-{}-{name}.toml",
            std::process::id()
        ));
        let source = format!("[layout]\nname = \"{name}\"\nversion = \"0.0.1\"\n{body}");
        fs::write(&path, source).expect("temp file is writable");
        path
    }

    fn path_text(path: &Path) -> &str {
        path.to_str().expect("utf-8 path")
    }

    #[test]
    fn check_accepts_the_shipped_layout_even_when_strict() {
        let outcome = intent(&["check", V0, "--strict"]);
        assert_eq!(outcome.err, "");
        assert_eq!(
            outcome.out,
            "intent-v0 0.2.0: ok, 426 of 1023 chords assigned\n"
        );
        assert_eq!(outcome.code, OK);
    }

    #[test]
    fn check_reports_every_error_and_fails() {
        let path = layout_file(
            "broken",
            "[thumbs]\nleft = \"explode\"\n[[family]]\nid = \"a\"\nplane = \"core\"\nleft = \"I\"\nmembers = [\"X\", \"X\"]\n",
        );
        let outcome = intent(&["check", path_text(&path)]);
        assert_eq!(outcome.code, FAILED);
        assert_eq!(outcome.out, "");
        assert!(
            outcome
                .err
                .contains("error: thumbs.left: unknown command \"explode\"")
        );
        assert!(outcome.err.contains("token X is listed twice"));
        assert!(outcome.err.contains("the layout has errors"));
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn check_passes_with_warnings_unless_strict() {
        let path = layout_file("warns", "[spelling]\nkeys = \"abc\"\n");
        let relaxed = intent(&["check", path_text(&path)]);
        assert_eq!(relaxed.code, OK);
        assert!(
            relaxed
                .err
                .starts_with("warning: spelling.keys has no chord for")
        );
        assert!(relaxed.out.starts_with("warns 0.0.1: ok, 12 of 1023"));

        let strict = intent(&["check", path_text(&path), "--strict"]);
        assert_eq!(strict.code, FAILED);
        assert!(
            strict
                .err
                .contains("1 warning(s), and --strict treats warnings as errors")
        );
        assert_eq!(strict.out, "");
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn missing_file_is_a_usage_error() {
        for command in ["check", "stats"] {
            let outcome = intent(&[command, "no/such/layout.toml"]);
            assert_eq!(outcome.code, USAGE);
            assert!(
                outcome
                    .err
                    .starts_with("error: cannot read no/such/layout.toml")
            );
        }
        assert_eq!(intent(&["sim", "no/such/layout.toml", "I-I"]).code, USAGE);
        assert_eq!(intent(&["how", "no/such/layout.toml", "FIX"]).code, USAGE);
    }

    #[test]
    fn help_and_version_succeed_on_stdout() {
        let help = intent(&["--help"]);
        assert_eq!(help.code, OK);
        assert!(help.out.contains("Tools for Intent chord layouts"));
        assert_eq!(help.err, "");
        let version = intent(&["--version"]);
        assert_eq!(version.code, OK);
        assert!(version.out.starts_with("intent "));
    }

    #[test]
    fn bad_arguments_are_usage_errors_on_stderr() {
        let outcome = intent(&["frobnicate"]);
        assert_eq!(outcome.code, USAGE);
        assert_eq!(outcome.out, "");
        assert!(outcome.err.contains("frobnicate"));
        assert_eq!(intent(&["sim", V0]).code, USAGE);
    }

    #[test]
    fn stats_summarises_planes_kinds_and_effort() {
        let outcome = intent(&["stats", V0, "--top", "2"]);
        assert_eq!(outcome.code, OK, "{}", outcome.err);
        let out = outcome.out;
        assert!(out.starts_with("intent-v0 0.2.0\nSeed layout"));
        assert!(out.contains("thumbs           3         3\n"), "{out}");
        assert!(out.contains("core           150       225\n"), "{out}");
        assert!(out.contains("user             0       225\n"), "{out}");
        assert!(out.contains("total          426      1023\n"), "{out}");
        assert!(
            out.contains("245 concepts, 162 text chords, 19 command chords\n"),
            "{out}"
        );
        assert!(
            out.contains("effort of letters: min 1.0, mean 3.9, max 7.1\n"),
            "{out}"
        );
        assert!(out.contains("effort of concepts: min 2.3,"), "{out}");
        let hardest: Vec<&str> = out
            .lines()
            .skip_while(|line| *line != "hardest concepts:")
            .skip(1)
            .collect();
        assert_eq!(hardest.len(), 2, "{out}");
        assert!(hardest[0].ends_with("CONFUSED"), "{out}");
    }

    #[test]
    fn stats_without_top_or_concepts() {
        let none = intent(&["stats", V0, "--top", "0"]);
        assert!(!none.out.contains("hardest concepts"));
        let path = layout_file("bare", "");
        let bare = intent(&["stats", path_text(&path)]);
        assert_eq!(bare.code, OK);
        assert!(
            bare.out
                .contains("0 concepts, 0 text chords, 0 command chords")
        );
        assert!(!bare.out.contains("effort of"));
        assert!(!bare.out.contains("hardest concepts"));
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn stats_fails_on_a_broken_layout() {
        let path = layout_file("bad-stats", "[thumbs]\nleft = \"explode\"\n");
        assert_eq!(intent(&["stats", path_text(&path)]).code, FAILED);
        assert_eq!(intent(&["sim", path_text(&path), "I-I"]).code, FAILED);
        assert_eq!(intent(&["how", path_text(&path), "FIX"]).code, FAILED);
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn sim_applies_chords_including_ones_that_start_with_a_hyphen() {
        let outcome = intent(&["sim", V0, "I-M", "M-I", "-I", "-T", "PRMIT-TIMRP", "T-"]);
        assert_eq!(outcome.code, OK, "{}", outcome.err);
        let lines: Vec<&str> = outcome.out.lines().collect();
        assert_eq!(lines.len(), 7);
        assert!(lines[0].starts_with("I-M          ...x. ..x..  FIX"));
        assert!(lines[1].contains("THIS"));
        assert!(lines[2].contains("\"e\""));
        assert!(lines[3].contains("<space>"));
        assert!(lines[4].ends_with("(not in this layout)"));
        assert!(lines[5].contains("<undo>"));
        assert_eq!(lines[6], "buffer: FIX THIS \"e\"");
    }

    #[test]
    fn sim_shows_sends_keys_and_no_ops() {
        let outcome = intent(&["sim", V0, "T-", "I-I", "MIT-I", "T-T"]);
        assert_eq!(outcome.code, OK, "{}", outcome.err);
        let lines: Vec<&str> = outcome.out.lines().collect();
        assert!(lines[0].ends_with("(nothing to do)"));
        assert!(lines[2].ends_with("presses the left key"));
        assert!(lines[3].ends_with("sends: MAKE"));
        assert_eq!(lines[4], "buffer: ");
    }

    #[test]
    fn sim_rejects_things_that_are_not_chords() {
        let outcome = intent(&["sim", V0, "I-I", "hello"]);
        assert_eq!(outcome.code, USAGE);
        assert_eq!(outcome.out, "");
        assert!(
            outcome
                .err
                .starts_with("error: \"hello\" is not a chord: missing '-'")
        );
    }

    #[test]
    fn how_plans_tokens_words_and_commands() {
        let outcome = intent(&["how", V0, "FIX", "BUG", "IN", "login", "<send>"]);
        assert_eq!(outcome.code, OK, "{}", outcome.err);
        let lines: Vec<&str> = outcome.out.lines().collect();
        // header + 3 tokens + 5 letters + send + totals + result
        assert_eq!(lines.len(), 12, "{}", outcome.out);
        assert!(lines[1].contains("I-M") && lines[1].contains("FIX"));
        assert!(lines[4].contains("\"l\""));
        assert!(lines[9].contains("T-T") && lines[9].contains("<send>"));
        assert!(lines[10].starts_with("9 strokes, "));
        assert_eq!(lines[11], "sends: FIX BUG IN \"login\"");
    }

    #[test]
    fn how_alternates_hands_and_adds_spaces_between_words() {
        let outcome = intent(&["how", V0, "ee", "e"]);
        assert_eq!(outcome.code, OK, "{}", outcome.err);
        let lines: Vec<&str> = outcome.out.lines().collect();
        assert!(lines[1].contains(" I-  "), "{}", outcome.out);
        assert!(lines[2].contains(" -I  "), "{}", outcome.out);
        assert!(lines[3].contains("<space>"));
        assert!(lines[4].contains(" I-  "), "{}", outcome.out);
        assert_eq!(lines[5], "4 strokes, 4 key presses, effort 3.8");
        assert_eq!(lines[6], "buffer: \"ee\" \"e\"");
    }

    #[test]
    fn how_spells_capitals_and_forced_text() {
        let outcome = intent(&["how", V0, "=FIX", "Ab"]);
        assert_eq!(outcome.code, OK, "{}", outcome.err);
        assert!(
            outcome.out.ends_with("buffer: \"FIX\" \"Ab\"\n"),
            "{}",
            outcome.out
        );
    }

    #[test]
    fn how_explains_what_it_cannot_do() {
        let unknown = intent(&["how", V0, "FROBNICATE"]);
        assert_eq!(unknown.code, FAILED);
        assert_eq!(
            unknown.err,
            "error: FROBNICATE is not a token in this layout (write =FROBNICATE to spell it out)\n"
        );
        assert!(
            intent(&["how", V0, "<fly>"])
                .err
                .contains("unknown command <fly>")
        );
        assert!(
            intent(&["how", V0, "naïve"])
                .err
                .contains("cannot type 'ï' (in \"naïve\")")
        );
        assert!(intent(&["how", V0, "="]).err.contains("nothing to spell"));

        let path = layout_file(
            "letters-only",
            "[spelling]\nkeys = \"eaoistnrdumlchgpfvbwykqjxz\"\n",
        );
        let no_space = intent(&["how", path_text(&path), "a", "b"]);
        assert_eq!(no_space.code, FAILED);
        assert!(no_space.err.contains("no chord for <space>"));
        let no_send = intent(&["how", path_text(&path), "<send>"]);
        assert!(no_send.err.contains("this layout has no chord for <send>"));
        fs::remove_file(path).expect("cleanup");
    }

    #[test]
    fn coverage_reports_running_text() {
        let corpus = std::env::temp_dir().join(format!("intent-cov-{}.txt", std::process::id()));
        fs::write(
            &corpus,
            "Fix the bug in login.

Thanks!
",
        )
        .expect("temp file");
        let outcome = intent(&["coverage", V0, path_text(&corpus), "--top", "1"]);
        assert_eq!(outcome.code, OK, "{}", outcome.err);
        let out = outcome.out;
        assert!(out.contains("concept               4   66.7%"), "{out}");
        assert!(out.contains("grammar               1   16.7%"), "{out}");
        assert!(out.contains("spelled               1   16.7%"), "{out}");
        assert!(
            out.contains("1 of 2 utterances need no spelling at all (50.0%)"),
            "{out}"
        );
        assert!(out.contains("1.83 strokes per word"), "{out}");
        assert!(
            out.ends_with(
                "most frequent spelled words:
           1  16.7%  login
"
            ),
            "{out}"
        );
        fs::remove_file(corpus).expect("cleanup");
    }

    #[test]
    fn encode_prints_one_utterance_per_line_or_json_with_messages() {
        let corpus = std::env::temp_dir().join(format!("intent-enc-{}.txt", std::process::id()));
        fs::write(
            &corpus,
            "I fixed the bug.

Thanks!
",
        )
        .expect("temp file");
        let plain = intent(&["encode", V0, path_text(&corpus)]);
        assert_eq!(plain.code, OK, "{}", plain.err);
        assert_eq!(
            plain.out,
            "I FIX BUG
THANKS
"
        );

        let json = intent(&["encode", V0, path_text(&corpus), "--messages"]);
        assert_eq!(json.code, OK, "{}", json.err);
        let first: serde_json::Value =
            serde_json::from_str(json.out.lines().next().expect("a line")).expect("json");
        assert_eq!(first["text"], "I fixed the bug.");
        assert_eq!(first["wire"], "I FIX BUG");
        let messages = first["messages"].as_array().expect("messages");
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages.last().expect("last")["content"], "I FIX BUG");
        fs::remove_file(corpus).expect("cleanup");

        assert_eq!(intent(&["encode", V0, "no/such/file.txt"]).code, USAGE);
    }

    #[test]
    fn coverage_reads_frequency_lists() {
        let corpus = std::env::temp_dir().join(format!("intent-freq-{}.txt", std::process::id()));
        fs::write(
            &corpus,
            "you 300
the 100
pizza 100
",
        )
        .expect("temp file");
        let outcome = intent(&["coverage", V0, path_text(&corpus), "--frequency"]);
        assert_eq!(outcome.code, OK, "{}", outcome.err);
        assert!(
            outcome.out.contains("concept             300   60.0%"),
            "{}",
            outcome.out
        );
        assert!(!outcome.out.contains("utterances"), "{}", outcome.out);

        fs::write(
            &corpus,
            "you 300
broken line
",
        )
        .expect("temp file");
        let bad = intent(&["coverage", V0, path_text(&corpus), "--frequency"]);
        assert_eq!(bad.code, USAGE);
        assert!(
            bad.err.contains("line 2: expected `word count`"),
            "{}",
            bad.err
        );
        fs::remove_file(corpus).expect("cleanup");

        let missing = intent(&["coverage", V0, "no/such/corpus.txt"]);
        assert_eq!(missing.code, USAGE);
        assert!(
            missing
                .err
                .starts_with("error: cannot read no/such/corpus.txt")
        );
    }

    #[test]
    fn single_capital_letters_are_spelled_not_treated_as_unknown_tokens() {
        assert!(!looks_like_token("A"));
        assert!(looks_like_token("AB"));
        assert!(looks_like_token("WITH_TESTS"));
        assert!(!looks_like_token("Ab"));
        assert!(!looks_like_token("login"));
        assert!(!looks_like_token(""));
    }
}
