use color_print::cformat;
use regex::Regex;
use rustyline::completion::FilenameCompleter;
use rustyline::highlight::Highlighter;
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{CompletionType, Config, Editor};
use rustyline_derive::{Completer, Helper, Hinter, Validator};
use std::borrow::Cow;

fn strip_ansi(s: &str) -> String {
    let re = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

#[derive(Helper, Completer, Hinter, Validator)]
struct ReadHelper {
    #[rustyline(Completer)]
    completer: FilenameCompleter,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    colored_prompt: String,
}

impl ReadHelper {
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
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        _prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Borrowed(&self.colored_prompt)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(cformat!("<dim>{}</>", hint))
    }
}

pub struct StdinReader;

impl StdinReader {
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
