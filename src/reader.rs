// // Copyright 2026 Merck KGaA, Darmstadt, Germany and/or its affiliates.
// // All rights reserved

//! # Reader
//!
//! Interactive terminal input with `rustyline`-powered line editing.
//!
//! This module provides two abstractions:
//!
//! | Type | Role |
//! |---|---|
//! | [`InputReader`] | Trait — testable seam for any code that needs a line of user input |
//! | [`StdinReader`] | Struct — production implementation backed by `rustyline` |
//!
//! ## Features
//!
//! - **ANSI-coloured prompts** — callers pass a `color_print`-formatted string;
//!   colour codes are rendered in the terminal but stripped before being passed
//!   to `rustyline` so cursor positioning is never corrupted.
//! - **Filename tab-completion** — pressing `<Tab>` completes filesystem paths,
//!   useful when the user is asked to enter a CSV file path.
//! - **History hinting** — previous entries are shown as dim ghost text.
//! - **Bracket validation** — mismatched brackets are flagged before the line
//!   is submitted.
//! - **Graceful Ctrl-C / Ctrl-D** — both signals call [`std::process::exit(0)`]
//!   for a clean shutdown from any call site.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::reader::{StdinReader, InputReader};
//!
//! let mut reader = StdinReader;
//! let path = reader.read_line(
//!     color_print::cformat!("<white>Enter CSV path: </>")
//! );
//! println!("You entered: {path}");
//! ```
//!
//! ## Testing
//!
//! Because all call sites accept `&mut impl InputReader`, tests substitute a
//! scripted fake without driving a real terminal:
//!
//! ```rust,ignore
//! struct ScriptedReader { lines: Vec<String> }
//!
//! impl InputReader for ScriptedReader {
//!     fn read_line(&mut self, _prompt: String) -> String {
//!         self.lines.pop().unwrap_or_default()
//!     }
//! }
//! ```

use color_print::cformat;
use regex::Regex;
use rustyline::completion::FilenameCompleter;
use rustyline::highlight::Highlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{CompletionType, Config, Editor};
use rustyline_derive::{Completer, Helper, Hinter, Validator};
use std::borrow::Cow;

/// Strips ANSI SGR escape sequences (e.g. `\x1b[32m`) from a string.
///
/// `rustyline` measures prompt width by counting raw bytes in the string
/// passed to [`Editor::readline`]. If ANSI colour codes are included,
/// `rustyline` miscounts the visible width and cursor positioning breaks.
///
/// This function produces a plain-text copy safe to pass as the `rustyline`
/// prompt, while the original coloured string is retained for display via the
/// [`Highlighter`] implementation on [`ReadHelper`].
///
/// # Arguments
/// * `s` — A string that may contain ANSI SGR sequences such as those
///          produced by `color_print::cformat!`.
///
/// # Returns
/// A new [`String`] with all sequences matching `\x1b\[[0-9;]*m` removed.
fn strip_ansi(s: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

/// `rustyline` helper that bundles tab-completion, history hinting, bracket
/// validation, and ANSI-coloured prompt highlighting into a single type.
///
/// This is a private implementation detail of [`StdinReader`]. It is
/// constructed fresh for every call to [`StdinReader::read_line`] so that the
/// coloured prompt string can vary between calls.
///
/// The four `rustyline_derive` traits ([`Helper`], [`Completer`], [`Hinter`],
/// [`Validator`]) are derived automatically and delegate to the named fields.
#[derive(Helper, Completer, Hinter, Validator)]
struct ReadHelper {
    /// Provides filesystem path completion on `<Tab>`.
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    /// Shows the most recent matching history entry as dim ghost text.
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    /// Flags mismatched brackets before the line is submitted.
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    /// The ANSI-coloured prompt string rendered by [`Highlighter::highlight_prompt`].
    colored_prompt: String,
}

impl ReadHelper {
    /// Creates a new [`ReadHelper`] for the given coloured prompt string.
    ///
    /// # Arguments
    /// * `colored_prompt` — An ANSI-coloured prompt as produced by
    ///                       `color_print::cformat!`. Stored and returned
    ///                       verbatim by [`Highlighter::highlight_prompt`].
    fn new(colored_prompt: String) -> Self {
        Self {
            completer: FilenameCompleter::new(),
            hinter: HistoryHinter {},
            validator: MatchingBracketValidator::new(),
            colored_prompt,
        }
    }
}

impl Highlighter for ReadHelper {
    /// Returns the ANSI-coloured prompt so the terminal renders colours while
    /// `rustyline` uses the plain-text version for width measurement.
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        _prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Borrowed(&self.colored_prompt)
    }

    /// Wraps the history hint text in a `<dim>` colour tag so it is visually
    /// distinct from user input.
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(cformat!("<dim>{}</>", hint))
    }
}

/// Production [`InputReader`] that reads a line from stdin using a
/// `rustyline` [`Editor`].
///
/// Features tab-completion, history hinting, bracket validation, and coloured
/// prompts. See the [module-level documentation](self) for a full feature list.
///
/// # Example
///
/// ```rust,ignore
/// let mut reader = StdinReader;
/// let input = reader.read_line(color_print::cformat!("<white>Path: </>" ));
/// ```
pub struct StdinReader;

impl StdinReader {
    /// Displays `colored_prompt` and reads a single trimmed line from stdin.
    ///
    /// Internally creates a short-lived `rustyline` [`Editor`] configured with
    /// [`ReadHelper`] (tab-completion, history hinting, bracket validation, and
    /// coloured prompt rendering).
    ///
    /// The coloured prompt is stripped of ANSI codes via [`strip_ansi`] before
    /// being passed to `rustyline` to keep cursor positioning correct; the
    /// original string is handed to [`ReadHelper`] for display.
    ///
    /// # Arguments
    /// * `colored_prompt` — An ANSI-formatted prompt string. May be produced
    ///                       by `color_print::cformat!`.
    ///
    /// # Returns
    /// The line entered by the user with leading and trailing whitespace
    /// trimmed. Returns an empty [`String`] on unexpected I/O errors.
    ///
    /// # Process exit
    /// Calls [`std::process::exit(0)`] on `Ctrl-C` (`Interrupted`) or
    /// `Ctrl-D` (`Eof`) so the application shuts down cleanly regardless of
    /// the call-stack depth.
    pub fn read_line(&mut self, colored_prompt: String) -> String {
        let plain_prompt = strip_ansi(&colored_prompt);

        let config = Config::builder()
            .completion_type(CompletionType::List)
            .build();

        let mut rl: Editor<ReadHelper, _> = Editor::with_config(config)
            .expect("Failed to create line editor");
        rl.set_helper(Some(ReadHelper::new(colored_prompt)));

        match rl.readline(&plain_prompt) {
            Ok(line) => line.trim().to_string(),
            Err(rustyline::error::ReadlineError::Interrupted) => {
                std::process::exit(0);
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Input error: {e}");
                String::new()
            }
        }
    }
}

/// Trait for reading a single line of user input.
///
/// Abstracting over this trait allows any code that needs interactive input to
/// be tested without a real terminal by substituting a scripted fake.
///
/// # Implementors
///
/// | Type | Purpose |
/// |---|---|
/// | [`StdinReader`] | Production — `rustyline`-backed terminal input |
/// | `ScriptedReader` (tests) | Returns pre-canned strings from a `Vec` |
///
/// # Example
///
/// ```rust,ignore
/// fn ask(reader: &mut impl InputReader) -> String {
///     reader.read_line("Enter value: ".to_string())
/// }
/// ```
pub trait InputReader {
    /// Displays `prompt` and returns the next line of input.
    ///
    /// Implementations should trim leading and trailing whitespace from the
    /// returned string. The prompt string may contain ANSI colour codes;
    /// implementations that do not support colour should strip them (see
    /// [`strip_ansi`]).
    ///
    /// # Arguments
    /// * `prompt` — The prompt string to display to the user.
    ///
    /// # Returns
    /// The trimmed line of input as a [`String`].
    fn read_line(&mut self, prompt: String) -> String;
}

impl InputReader for StdinReader {
    fn read_line(&mut self, prompt: String) -> String {
        self.read_line(prompt)
    }
}
