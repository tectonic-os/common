//! What no flag gave. Every command takes flags for everything it needs; this
//! fills what they left empty when there is someone to ask, and names the flag
//! when there is not.

use crate::ui::{self, Answer, Choice};
use std::cell::Cell;
use std::io::{IsTerminal, Write};

/// A question drawn on its own may end in a colon; one with a default, a
/// range or an answer written after it must not carry two.
fn stem(question: &str) -> &str {
    question.trim_end_matches(':')
}

/// A file of answers, one per line, which a run answers from without asking.
/// What the transcript goldens drive the binary with.
const SCRIPT: &str = "TECT_ANSWERS";

/// The sentinel cannot equal valid prompt text. It carries cancellation through
/// string errors without masking an operation error.
const CANCELLED: &str = "\0cancelled";

pub fn cancelled(message: &str) -> bool {
    message == CANCELLED
}

pub struct Prompt {
    ask: bool,
    /// Whether an answer may be asked for with a widget, which redirected
    /// output rules out.
    draw: bool,
    /// The answers a scripted run reads, and how far through them it is.
    script: Option<(Vec<String>, Cell<usize>)>,
}

impl Prompt {
    pub fn new(no_tui: bool) -> Self {
        if let Some(path) = std::env::var_os(SCRIPT) {
            // An unreadable script still runs with no answers, so the first
            // question fails, and the user is told why it had none.
            let text = std::fs::read_to_string(&path).unwrap_or_else(|err| {
                eprintln!("{SCRIPT}: {}: {err}", std::path::Path::new(&path).display());
                String::new()
            });
            return Self::scripted(text.lines().map(str::to_string).collect());
        }
        let ask = !no_tui && std::io::stdin().is_terminal();
        Self {
            ask,
            draw: ask && std::io::stdout().is_terminal(),
            script: None,
        }
    }

    /// Nobody to ask: every missing value fails naming its flag.
    pub fn silent() -> Self {
        Self {
            ask: false,
            draw: false,
            script: None,
        }
    }

    /// The answers in order, whatever the terminal is. A question they run out
    /// for fails, so an unexpected one is a failure. Waiting would hang a
    /// scripted run.
    pub fn scripted(answers: Vec<String>) -> Self {
        Self {
            ask: true,
            draw: false,
            script: Some((answers, Cell::new(0))),
        }
    }

    /// Whether there is anyone to ask, which is what a line standing with a
    /// question has to know.
    pub fn asks(&self) -> bool {
        self.ask
    }

    /// Whether an answer may be drawn, which is what a command with nothing to
    /// do but ask has to know: it opens a picker, or it refuses.
    pub fn draws(&self) -> bool {
        self.draw
    }

    /// `shown` is everything that stands before the answer, and `question` is
    /// what a run with no answer left for it names. One blank line follows,
    /// which is what separates a question from what comes after it.
    fn read(&self, question: &str, shown: &str) -> Result<String, String> {
        let answer = self.answer(question, shown)?;
        println!();
        Ok(answer)
    }

    fn answer(&self, question: &str, shown: &str) -> Result<String, String> {
        if let Some((answers, at)) = &self.script {
            let answer = answers
                .get(at.get())
                .ok_or_else(|| format!("nothing left in {SCRIPT} to answer `{question}` with"))?;
            at.set(at.get() + 1);
            println!("{shown}{answer}");
            return Ok(answer.trim().to_string());
        }
        print!("{shown}");
        std::io::stdout().flush().map_err(|err| err.to_string())?;
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .map_err(|err| err.to_string())?;
        Ok(answer.trim().to_string())
    }

    /// What the flag gave, else the answer, else the default, else a failure
    /// naming `flag`.
    pub fn text(
        &self,
        given: Option<String>,
        question: &str,
        flag: &str,
        default: Option<&str>,
    ) -> Result<String, String> {
        self.asked(given, question, flag, None, default)
    }

    /// Both free-text questions. `prefix` is `None` where the answer is typed
    /// after the question on one line, and what it is typed after otherwise.
    /// A default is shown in the question and answered by typing nothing.
    fn asked(
        &self,
        given: Option<String>,
        question: &str,
        flag: &str,
        prefix: Option<&str>,
        default: Option<&str>,
    ) -> Result<String, String> {
        match self.asked_answer(given, question, flag, prefix, default)? {
            Some(answer) => Ok(answer),
            None => Err(CANCELLED.to_string()),
        }
    }

    fn asked_answer(
        &self,
        given: Option<String>,
        question: &str,
        flag: &str,
        prefix: Option<&str>,
        default: Option<&str>,
    ) -> Result<Option<String>, String> {
        if let Some(value) = given.filter(|value| !value.is_empty()) {
            return Ok(Some(value));
        }
        let missing = || match default {
            Some(default) => Ok(default.to_string()),
            None => Err(format!(
                "give {flag}, since nothing can be asked here: {}",
                stem(question)
            )),
        };
        if !self.ask {
            return missing().map(Some);
        }
        if self.draw {
            return Self::written_answer(question, prefix.unwrap_or(""), default, missing);
        }
        let labelled = match default {
            Some(default) => format!("{} [{default}]", stem(question)),
            None => question.to_string(),
        };
        let shown = match prefix {
            None => format!("{}: ", stem(&labelled)),
            Some(prefix) => format!("{labelled}\n{prefix}"),
        };
        match self.read(&labelled, &shown)? {
            answer if answer.is_empty() => missing().map(Some),
            answer => Ok(Some(answer)),
        }
    }

    /// The drawn half of both free-text questions. The default is the widget's
    /// to show, so the question is passed bare. Nothing is echoed for an answer
    /// that ends in the refusal.
    fn written_answer(
        question: &str,
        prefix: &str,
        default: Option<&str>,
        missing: impl Fn() -> Result<String, String>,
    ) -> Result<Option<String>, String> {
        let Some(typed) = ui::line(stem(question), prefix, default)? else {
            return Ok(None);
        };
        let answer = match typed.is_empty() {
            true => missing()?,
            false => typed,
        };
        println!("{}: {prefix}{answer}\n", stem(question));
        Ok(Some(answer))
    }

    /// The same, asked over two lines: the question on its own, the answer
    /// typed after `prefix`, so what the answer belongs to stays visible.
    pub fn line(
        &self,
        given: Option<String>,
        question: &str,
        flag: &str,
        prefix: &str,
        default: Option<&str>,
    ) -> Result<String, String> {
        self.asked(given, question, flag, Some(prefix), default)
    }

    /// A step with no flag of its own: the flag that answers it is the answer,
    /// so with nobody to ask the answer is no. `yes` and `no` are what the two
    /// answers are called, since not every one of them is a refusal.
    pub fn confirm(&self, question: &str, yes: &str, no: &str) -> Result<bool, String> {
        if !self.ask {
            return Ok(false);
        }
        self.confirm_current(question, yes, no, true)
    }

    /// A confirmation editing an existing answer, which opens on and defaults
    /// to that answer. Always opening on yes would discard it.
    pub fn confirm_current(
        &self,
        question: &str,
        yes: &str,
        no: &str,
        current: bool,
    ) -> Result<bool, String> {
        self.confirm_answer(question, yes, no, current)?
            .ok_or_else(|| CANCELLED.to_string())
    }

    fn confirm_answer(
        &self,
        question: &str,
        yes: &str,
        no: &str,
        current: bool,
    ) -> Result<Option<bool>, String> {
        if !self.ask {
            return Ok(Some(current));
        }
        if self.draw {
            let chosen = ui::confirm_current(question, yes, no, current)?;
            if let Some(chosen) = chosen {
                println!("{}: {}\n", stem(question), if chosen { yes } else { no });
            }
            return Ok(chosen);
        }
        let question = format!("{} ({yes}/{no})", stem(question));
        let answer = self.read(&question, &format!("{question}\n"))?;
        Ok(Some(confirmation(&answer, no, current)))
    }

    /// One of a set, or none of them: an inline select list where the output is
    /// a terminal, a numbered list to answer by number or by name where it is
    /// not.
    pub fn choose(&self, question: &str, options: &[Choice]) -> Result<Option<usize>, String> {
        self.pick_one(question, options, None)
    }

    /// The same, editing an existing answer: it opens on that answer, marks it
    /// in the numbered list, and takes an empty answer as keeping it.
    pub fn choose_current(
        &self,
        question: &str,
        options: &[Choice],
        current: usize,
    ) -> Result<Option<usize>, String> {
        self.pick_one(question, options, Some(current))
    }

    /// `current` is the answer already held, which is what nobody to ask is
    /// answered with and what an empty answer keeps. A question opening on
    /// nothing has none, and an empty answer to that one is not an answer.
    fn pick_one(
        &self,
        question: &str,
        options: &[Choice],
        current: Option<usize>,
    ) -> Result<Option<usize>, String> {
        if !self.ask || options.is_empty() {
            return Ok(current);
        }
        if self.draw {
            let chosen = ui::select_current(question, options, current.unwrap_or(0))?;
            if let Some(index) = chosen {
                println!("{}: {}\n", stem(question), options[index].label);
            }
            return chosen.map(Some).ok_or_else(|| CANCELLED.to_string());
        }
        Self::numbered(options, current.as_slice());
        let question = format!("{} [{}, 0 for none]", stem(question), range(options));
        let answer = self.read(&question, &format!("{question}: "))?;
        one_answer(&answer, options, current)
    }

    /// Any of a set, or none of them: an inline multi-select where the output
    /// is a terminal, the numbered list taking several numbers or names on the
    /// one line where it is not. `on` is what is already true, which an empty
    /// answer takes and which nobody to ask is answered with.
    pub fn choose_many(
        &self,
        question: &str,
        options: &[Choice],
        on: &[usize],
    ) -> Result<Answer, String> {
        if !self.ask || options.is_empty() {
            return Ok(Answer::Chosen(on.to_vec()));
        }
        if self.draw {
            let answer = ui::multi(question, options, on)?;
            if matches!(answer, Answer::Cancelled) {
                return Err(CANCELLED.to_string());
            }
            println!("{}: {}\n", stem(question), said(&answer, options));
            return Ok(answer);
        }
        Self::numbered(options, on);
        let several = match options.len() {
            1 => "",
            _ => ", several",
        };
        let question = format!(
            "{} [{}{several}, 0 for none]",
            stem(question),
            range(options)
        );
        let answer = self.read(&question, &format!("{question}: "))?;
        many_answer(&answer, options, on)
    }

    /// `on` marks what is already true, and marks nothing for a question that
    /// opens with nothing chosen.
    fn numbered(options: &[Choice], on: &[usize]) {
        for (index, option) in options.iter().enumerate() {
            let mark = match (on.is_empty(), on.contains(&index)) {
                (true, _) => "",
                (false, true) => "[x] ",
                (false, false) => "[ ] ",
            };
            // Dim ink marks an unavailable option on a drawn list. A numbered
            // list has no ink, so it says so in words.
            let absent = match option.dim {
                true => "(unavailable) ",
                false => "",
            };
            let line = format!("{mark}{absent}{}  {}", option.label, option.detail);
            println!("  {}) {}", index + 1, line.trim_end());
        }
    }
}

fn confirmation(answer: &str, no: &str, current: bool) -> bool {
    if answer.is_empty() {
        return current;
    }
    let first = |word: &str| word.chars().next().map(|c| c.to_ascii_lowercase());
    first(answer) != first(no)
}

fn one_answer(
    answer: &str,
    options: &[Choice],
    current: Option<usize>,
) -> Result<Option<usize>, String> {
    if let (true, Some(current)) = (answer.is_empty(), current) {
        return Ok(Some(current));
    }
    let at = match answer.parse::<usize>() {
        Ok(0) => return Ok(None),
        Ok(number) if number <= options.len() => number - 1,
        _ => options
            .iter()
            .position(|option| option.label == answer)
            .ok_or_else(|| format!("`{answer}` is not one of them"))?,
    };
    choosable(answer, options, at).map(Some)
}

/// Refuses an answer that names an option the drawn list would not let the
/// user pick, and says why with the option's own detail.
fn choosable(answer: &str, options: &[Choice], at: usize) -> Result<usize, String> {
    let option = &options[at];
    match (option.available, option.detail.is_empty()) {
        (true, _) => Ok(at),
        (false, true) => Err(format!("`{answer}` cannot be chosen")),
        (false, false) => Err(format!("`{answer}` cannot be chosen: {}", option.detail)),
    }
}

fn many_answer(answer: &str, options: &[Choice], on: &[usize]) -> Result<Answer, String> {
    if answer.is_empty() {
        return Ok(Answer::Chosen(on.to_vec()));
    }
    let mut chosen: Vec<usize> = Vec::new();
    for word in answer.split([' ', ',']).filter(|word| !word.is_empty()) {
        let at = match word.parse::<usize>() {
            Ok(0) => return Ok(Answer::Chosen(Vec::new())),
            Ok(number) if number <= options.len() => number - 1,
            _ => options
                .iter()
                .position(|option| option.label == word)
                .ok_or_else(|| format!("`{word}` is not one of them"))?,
        };
        let at = choosable(word, options, at)?;
        if !chosen.contains(&at) {
            chosen.push(at);
        }
    }
    Ok(Answer::Chosen(chosen))
}

#[cfg(feature = "fuzzing")]
pub fn fuzz_answer(kind: u8, answers: &str) {
    let options = [
        Choice::new("one", "first"),
        Choice::new("two", "second"),
        Choice::new("three", "third"),
    ];
    for answer in answers.lines().map(str::trim) {
        match kind % 4 {
            0 => {
                let _ = answer.to_string();
            }
            1 => {
                let _ = confirmation(answer, "no", false);
            }
            2 => {
                let _ = one_answer(answer, &options, Some(1));
            }
            _ => {
                let _ = many_answer(answer, &options, &[1]);
            }
        }
    }
}

/// Past this many, an answer echoes as a count: a screen of labels reads back
/// as nothing at all.
const LISTED: usize = 8;

/// What a widget echoes after itself, so a terminal reads back the same answer
/// a script typed.
fn said(answer: &Answer, options: &[Choice]) -> String {
    let Answer::Chosen(chosen) = answer else {
        return "none".to_string();
    };
    match chosen.len() {
        0 => "none".to_string(),
        many if many > LISTED => format!("{many} chosen"),
        _ => chosen
            .iter()
            .map(|at| options[*at].label.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// What answers by number the question takes, which is one number when there is
/// one option.
fn range(options: &[Choice]) -> String {
    match options.len() {
        1 => "1".to_string(),
        n => format!("1-{n}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(count: usize) -> Vec<Choice> {
        (0..count)
            .map(|at| Choice::new(format!("rule {at}"), ""))
            .collect()
    }

    fn scripted(answers: &[&str]) -> Prompt {
        Prompt::scripted(answers.iter().map(|answer| answer.to_string()).collect())
    }

    #[test]
    fn a_silent_prompt_takes_a_given_answer() {
        assert_eq!(
            Prompt::silent().text(
                Some("given".into()),
                "Question?",
                "--answer",
                Some("default")
            ),
            Ok("given".into())
        );
    }

    #[test]
    fn a_silent_prompt_takes_its_default() {
        assert_eq!(
            Prompt::silent().text(None, "Question?", "--answer", Some("default")),
            Ok("default".into())
        );
    }

    #[test]
    fn a_silent_prompt_names_the_flag_for_an_answer_it_lacks() {
        let prompt = Prompt::silent();
        let error = prompt
            .line(None, "Question:", "--answer", "prefix: ", None)
            .unwrap_err();
        assert_eq!(
            error,
            "give --answer, since nothing can be asked here: Question"
        );
    }

    #[test]
    fn a_silent_prompt_neither_asks_nor_draws() {
        let prompt = Prompt::silent();
        assert!(!prompt.asks());
        assert!(!prompt.draws());
    }

    #[test]
    fn a_scripted_prompt_trims_an_answer() {
        assert_eq!(
            scripted(&["  answer  "]).text(None, "Question?", "--answer", None),
            Ok("answer".into())
        );
    }

    #[test]
    fn a_scripted_prompt_fails_when_the_script_ends() {
        assert_eq!(
            scripted(&[]).text(None, "Second?", "--second", None),
            Err("nothing left in TECT_ANSWERS to answer `Second?` with".into())
        );
    }

    #[test]
    fn a_scripted_prompt_asks_without_drawing() {
        let prompt = scripted(&[]);
        assert!(prompt.asks());
        assert!(!prompt.draws());
    }

    #[test]
    fn an_empty_scripted_answer_takes_the_default() {
        assert_eq!(
            scripted(&[""]).text(None, "Question?", "--answer", Some("default")),
            Ok("default".into())
        );
    }

    #[test]
    fn a_silent_confirmation_declines_a_new_answer() {
        assert_eq!(Prompt::silent().confirm("Change?", "Yes", "No"), Ok(false));
    }

    #[test]
    fn a_silent_confirmation_keeps_the_current_answer() {
        assert_eq!(
            Prompt::silent().confirm_current("Change?", "Yes", "No", true),
            Ok(true)
        );
    }

    #[test]
    fn scripted_confirmations_take_an_empty_answer_as_the_current_one() {
        assert_eq!(
            scripted(&[""]).confirm_current("Change?", "Yes", "No", false),
            Ok(false)
        );
    }

    #[test]
    fn scripted_confirmations_match_the_first_letter_of_the_no_answer() {
        assert_eq!(
            scripted(&["never"]).confirm("Change?", "Always", "Never"),
            Ok(false)
        );
        assert_eq!(
            scripted(&["anything else"]).confirm("Change?", "Always", "Never"),
            Ok(true)
        );
    }

    #[test]
    fn a_scripted_picker_accepts_a_number_a_label_and_zero() {
        let options = options(3);
        assert_eq!(scripted(&["2"]).choose("Choose?", &options), Ok(Some(1)));
        assert_eq!(
            scripted(&["rule 2"]).choose("Choose?", &options),
            Ok(Some(2))
        );
        assert_eq!(scripted(&["0"]).choose("Choose?", &options), Ok(None));
    }

    #[test]
    fn a_scripted_picker_keeps_a_current_answer_on_an_empty_line() {
        let options = options(3);
        assert_eq!(
            scripted(&[""]).choose_current("Choose?", &options, 2),
            Ok(Some(2))
        );
    }

    #[test]
    fn a_scripted_picker_refuses_an_answer_outside_its_options() {
        assert_eq!(
            scripted(&["4"]).choose("Choose?", &options(3)),
            Err("`4` is not one of them".into())
        );
    }

    #[test]
    fn a_scripted_answer_naming_an_unavailable_option_is_refused_with_its_reason() {
        let mut options = options(3);
        options[1] = Choice::new("two", "supports fedora").unavailable();
        assert_eq!(
            scripted(&["2"]).choose("Choose?", &options),
            Err("`2` cannot be chosen: supports fedora".into())
        );
        assert!(matches!(
            scripted(&["1 two"]).choose_many("Choose?", &options, &[]),
            Err(message) if message == "`two` cannot be chosen: supports fedora"
        ));
    }

    #[test]
    fn a_silent_picker_with_no_current_answer_chooses_nothing() {
        assert_eq!(Prompt::silent().choose("Choose?", &options(3)), Ok(None));
    }

    #[test]
    fn a_silent_picker_keeps_its_current_answer() {
        assert_eq!(
            Prompt::silent().choose_current("Choose?", &options(3), 1),
            Ok(Some(1))
        );
    }

    #[test]
    fn a_scripted_multi_picker_accepts_numbers_and_labels_once_each() {
        let options = ["first", "second", "third"]
            .map(|label| Choice::new(label, ""))
            .to_vec();
        let Answer::Chosen(chosen) = scripted(&["1, third 1"])
            .choose_many("Choose?", &options, &[])
            .unwrap()
        else {
            panic!("a scripted answer cannot cancel");
        };
        assert_eq!(chosen, vec![0, 2]);
    }

    #[test]
    fn zero_clears_a_scripted_multi_picker() {
        let Answer::Chosen(chosen) = scripted(&["0"])
            .choose_many("Choose?", &options(3), &[1])
            .unwrap()
        else {
            panic!("a scripted answer cannot cancel");
        };
        assert!(chosen.is_empty());
    }

    #[test]
    fn a_silent_multi_picker_keeps_its_current_answers() {
        let Answer::Chosen(chosen) = Prompt::silent()
            .choose_many("Choose?", &options(3), &[0, 2])
            .unwrap()
        else {
            panic!("a silent answer cannot cancel");
        };
        assert_eq!(chosen, vec![0, 2]);
    }

    #[test]
    fn the_cancelled_sentinel_is_not_a_user_facing_error() {
        assert!(cancelled(CANCELLED));
        assert!(!cancelled("cancelled"));
    }

    #[test]
    fn a_short_answer_reads_back_as_itself_and_a_long_one_as_a_count() {
        let options = options(20);
        let short = Answer::Chosen((0..LISTED).collect());
        assert!(said(&short, &options).starts_with("rule 0, rule 1,"));
        let long = Answer::Chosen((0..LISTED + 1).collect());
        assert_eq!(said(&long, &options), format!("{} chosen", LISTED + 1));
        assert_eq!(said(&Answer::Chosen(Vec::new()), &options), "none");
        assert_eq!(said(&Answer::Cancelled, &options), "none");
    }
}
