//! `retone --eval-styles`: sample texts through every preset style with the real prompt builder
//! and provider, printed side by side, so a style prompt change can be measured.
//!
//! A developer tool, run by hand: it prints the sample texts and the answers to stdout. The app
//! never calls it, so the no-history rule for user text is not touched.
//!
//! ```text
//! retone --eval-styles --model qwen2.5:3b [--endpoint http://127.0.0.1:8080/v1] [--text FILE]
//! retone --eval-styles --provider pi --model ollama/qwen2.5:3b [--style summarize]
//! retone --eval-styles --model NAME --runs 3 --json out.json   # used by scripts/model-bench.sh
//! ```

#![forbid(unsafe_code)]

use crate::provider::{Cancel, Env, Outcome, Provider, local};
use crate::styles::{PRESETS, Style, TONES};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// Concise must cut at least this much, or it did not do its one job.
const CONCISE_MAX_RATIO: f64 = 0.7;
/// Fix corrects errors only: past this word distance from the text, it rewrote it.
const FIX_MAX_CHANGE: f64 = 0.35;
/// Summarize writes "at most a third of the words"; this leaves room for longer names and numbers.
const SUMMARY_MAX_RATIO: f64 = 0.5;
/// Expand must add wording, but at most twice the length (the user's limit for this action).
const EXPAND_RATIOS: std::ops::RangeInclusive<f64> = 1.2..=2.0;
/// Two answers closer than this read as the same rewrite.
const MIN_PAIR_DISTANCE: f64 = 0.1;
/// The tones together must spread at least this far: some pairs may fairly read alike (a
/// Professional and a Technical email), but not the whole set.
const MIN_MEAN_DISTANCE: f64 = 0.3;
/// Details of the style examples in `styles.rs`. In an answer to a text without them, the model
/// copied them from the example: an invented fact.
const EXAMPLE_DETAILS: [&str; 9] =
    ["safari", "login", "yesterday", "sam", "thursday", "monday", "wednesday", "changelog", "complaining"];

const SAMPLES: [(&str, &str); 4] = [
    (
        "chat",
        "hey so i looked at the deploy thing and basically it broke because the config file had the wrong port i \
         think, i fixed it now but we should probably add a check so this doesnt happen again next time",
    ),
    (
        "email",
        "Hi Anna, I just wanted to reach out and let you know that unfortunately I won't be able to make it to the \
         meeting on Thursday because something came up with another client that I really need to deal with, so \
         would it maybe be possible to move it to Friday afternoon or sometime early next week if that works for you?",
    ),
    (
        "bug",
        "the app is kind of slow when you open a big file, like it freezes for a few seconds and then it works \
         again, i think its because it loads everything at once instead of bit by bit",
    ),
    (
        "notes",
        "ok so notes from todays call. we agreed to ship version 2.4 on march 3 if qa signs off by friday. anna will \
         finish the export bug fix, it affects about 40 customers who use csv files with more than 10000 rows. tom \
         said the new pricing page needs another review because legal hasnt seen the last 2 changes yet. we also \
         talked about moving the weekly sync from tuesday to wednesday at 10am but nobody decided. next call is in 2 \
         weeks, please send your updates before then",
    ),
];

/// Runs the eval when the arguments ask for it; returns the exit code, or None for a normal start.
pub fn run_from_args() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.iter().any(|arg| arg == "--eval-styles") {
        return None;
    }
    Some(match run(&args) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(error) => {
            eprintln!("retone --eval-styles: {error}");
            2
        }
    })
}

pub fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|arg| arg == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn run(args: &[String]) -> Result<bool, String> {
    let model = flag(args, "--model").ok_or("--model is required")?.to_string();
    let provider = match flag(args, "--provider").unwrap_or("local") {
        "local" => {
            // An explicit address on the command line counts as the opt-in for a network server.
            let endpoint = local::validate_endpoint(flag(args, "--endpoint").unwrap_or(local::OLLAMA), true)?;
            Provider::Local { endpoint, model }
        }
        "pi" => Provider::Pi { model },
        "opencode" => Provider::OpenCode { model },
        "claude" => Provider::ClaudeCode { model },
        "codex" => Provider::Codex { model },
        other => return Err(format!("unknown provider {other}")),
    };
    let mut env = Env::system().ok_or("HOME is not set")?;
    env.local_api_key = rune_core::env::var("RETONE_API_KEY").map(local::ApiKey);
    let own_text = match flag(args, "--text") {
        Some(path) => Some(std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?),
        None => None,
    };
    let styles: Vec<&Style> = match flag(args, "--style") {
        Some(id) => vec![PRESETS.iter().find(|style| style.id() == id).ok_or(format!("unknown style {id}"))?],
        None => PRESETS.iter().collect(),
    };
    let samples: Vec<(&str, &str)> = match &own_text {
        Some(text) => vec![("text", text.trim())],
        None => SAMPLES.to_vec(),
    };

    let runs: usize = match flag(args, "--runs") {
        Some(runs) => runs.parse().ok().filter(|&runs| runs > 0).ok_or("--runs takes a number from 1")?,
        None => 1,
    };
    let json_path = flag(args, "--json");
    let rewrite = |style: &Style, text: &str| -> Timed {
        let start = rune_core::time::now();
        let first = Cell::new(None);
        let on_partial = |_: &str| {
            if first.get().is_none() {
                first.set(Some(start.elapsed()));
            }
        };
        let Outcome { answer, .. } = provider.rewrite(style, text, &Cancel::default(), &env, &on_partial);
        let total = start.elapsed();
        // A server that does not stream sends the whole answer at once.
        let first = first.get().unwrap_or(total);
        Timed { answer: answer.unwrap_or_else(|error| format!("ERROR: {error}")), first, total }
    };
    if json_path.is_some() {
        // The first request can load the model; that time is not the model's speed.
        rewrite(&Style::Clear, samples[0].1);
    }

    println!("# Style eval: {} · {}\n", provider.kind().name(), provider.model());
    let mut all_passed = true;
    let mut bench = Bench::default();
    for run in 1..=runs {
        for &(name, text) in &samples {
            let timed: Vec<(Style, Timed)> =
                styles.iter().map(|&style| (style.clone(), rewrite(style, text))).collect();
            let answers: Vec<(Style, String)> =
                timed.iter().map(|(style, timed)| (style.clone(), timed.answer.clone())).collect();
            let report = Report::new(text, &answers);
            let title = if runs > 1 { format!("{name}, run {run}") } else { name.to_string() };
            print!("{}", report.markdown(&title, text));
            all_passed &= report.failures.is_empty();
            bench.add(text, &timed, &report);
        }
    }
    println!("{}", if all_passed { "PASS" } else { "FAIL" });
    if let Some(path) = json_path {
        let summary = bench.summary(provider.model(), runs);
        std::fs::write(path, format!("{summary:#}\n")).map_err(|e| format!("{path}: {e}"))?;
    }
    Ok(all_passed)
}

/// One answer and how long it took: to the first streamed text, and to the end.
struct Timed {
    answer: String,
    first: Duration,
    total: Duration,
}

/// Counts for `--json`, the numbers `scripts/model-bench.sh` compares models by. A cell is one
/// style on one sample in one run.
#[derive(Default)]
struct Bench {
    cells: usize,
    passed: usize,
    errors: usize,
    invented: usize,
    /// Cells that lost a number the text has; Summarize may fairly drop details, so it is skipped.
    dropped: usize,
    kept_checked: usize,
    tone_spread_failures: usize,
    styles: BTreeMap<String, StyleCounts>,
    first: Vec<f64>,
    total: Vec<f64>,
}

#[derive(Default)]
struct StyleCounts {
    cells: usize,
    passed: usize,
    /// Answer length against the text, for answers without an error.
    ratios: Vec<f64>,
}

impl Bench {
    fn add(&mut self, original: &str, timed: &[(Style, Timed)], report: &Report) {
        self.tone_spread_failures += report.spread_failures;
        for ((style, timed), (_, answer, ratio, changed)) in timed.iter().zip(&report.rows) {
            let failures = answer_failures(style, original, answer, *ratio, *changed);
            let counts = self.styles.entry(style.id().to_string()).or_default();
            counts.cells += 1;
            self.cells += 1;
            // A timeout is a real wait for the user, so failed answers count in the times too.
            self.first.push(timed.first.as_secs_f64());
            self.total.push(timed.total.as_secs_f64());
            if answer.starts_with("ERROR:") {
                self.errors += 1;
                continue;
            }
            counts.ratios.push(*ratio);
            if failures.is_empty() {
                counts.passed += 1;
                self.passed += 1;
            }
            if !new_numbers(original, answer).is_empty() || !example_details(original, answer).is_empty() {
                self.invented += 1;
            }
            if *style != Style::Summarize {
                self.kept_checked += 1;
                // A number of the text that the answer does not have.
                if !new_numbers(answer, original).is_empty() {
                    self.dropped += 1;
                }
            }
        }
    }

    fn summary(&self, model: &str, runs: usize) -> serde_json::Value {
        let share = |part: usize, whole: usize| part as f64 / whole.max(1) as f64;
        let styles: serde_json::Map<String, serde_json::Value> = self
            .styles
            .iter()
            .map(|(id, counts)| {
                let pass_rate = share(counts.passed, counts.cells);
                let value = serde_json::json!({ "pass_rate": pass_rate, "length_ratio": median(&counts.ratios) });
                (id.clone(), value)
            })
            .collect();
        serde_json::json!({
            "model": model,
            "runs": runs,
            "cells": self.cells,
            "pass_rate": share(self.passed, self.cells),
            "errors": self.errors,
            "invented_rate": share(self.invented, self.cells - self.errors),
            "dropped_rate": share(self.dropped, self.kept_checked),
            "tone_spread_failures": self.tone_spread_failures,
            "first_token_s": median(&self.first),
            "total_s": median(&self.total),
            "total_p90_s": percentile(&self.total, 0.9),
            "styles": styles,
        })
    }
}

fn median(values: &[f64]) -> f64 {
    percentile(values, 0.5)
}

/// The nearest-rank percentile; 0 for no values.
fn percentile(values: &[f64], p: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (p * sorted.len() as f64).ceil() as usize;
    sorted.get(rank.saturating_sub(1)).copied().unwrap_or(0.0)
}

struct Report<'a> {
    rows: Vec<(&'a Style, &'a str, f64, f64)>,
    failures: Vec<String>,
    /// How many of `failures` are about the tones reading alike, not about one answer.
    spread_failures: usize,
}

/// What is wrong with one answer on its own.
fn answer_failures(style: &Style, original: &str, answer: &str, ratio: f64, changed: f64) -> Vec<String> {
    if answer.starts_with("ERROR:") {
        return vec![format!("{} failed", style.name())];
    }
    let mut failures = Vec::new();
    // Fix may fairly return a text that has no errors unchanged.
    if changed == 0.0 && *style != Style::Fix {
        failures.push(format!("{} returned the text unchanged", style.name()));
    }
    failures.extend(job_failure(style, original, answer, ratio, changed));
    let added = new_numbers(original, answer);
    if !added.is_empty() {
        failures.push(format!("{} added numbers the text does not have: {}", style.name(), added.join(", ")));
    }
    let copied = example_details(original, answer);
    if !copied.is_empty() {
        failures.push(format!("{} copied details from the example: {}", style.name(), copied.join(", ")));
    }
    failures
}

/// Words of [`EXAMPLE_DETAILS`] in the answer that the text does not have.
fn example_details(original: &str, answer: &str) -> Vec<&'static str> {
    let (original, answer) = (words(original), words(answer));
    EXAMPLE_DETAILS.into_iter().filter(|word| answer.contains(*word) && !original.contains(*word)).collect()
}

impl<'a> Report<'a> {
    fn new(original: &str, answers: &'a [(Style, String)]) -> Report<'a> {
        let rows: Vec<_> = answers
            .iter()
            .map(|(style, answer)| {
                let changed = word_distance(original, answer);
                (style, answer.as_str(), length_ratio(original, answer), changed)
            })
            .collect();
        let mut failures: Vec<String> = rows
            .iter()
            .flat_map(|(style, answer, ratio, changed)| answer_failures(style, original, answer, *ratio, *changed))
            .collect();
        let answer_failure_count = failures.len();
        // Only the tones must read apart: Fix is close to the text by design, and a summary may
        // fairly read like Concise.
        let tones: Vec<_> = rows.iter().filter(|row| PRESETS[..TONES].contains(row.0)).collect();
        let mut distances = Vec::new();
        for (i, a) in tones.iter().enumerate() {
            for b in &tones[i + 1..] {
                let distance = word_distance(a.1, b.1);
                if distance < MIN_PAIR_DISTANCE {
                    let (x, y) = (a.0.name(), b.0.name());
                    failures.push(format!("{x} and {y} are {distance:.2} apart (min {MIN_PAIR_DISTANCE})"));
                }
                distances.push(distance);
            }
        }
        let mean = distances.iter().sum::<f64>() / distances.len().max(1) as f64;
        if !distances.is_empty() && mean < MIN_MEAN_DISTANCE {
            failures.push(format!("the tones are {mean:.2} apart on average (min {MIN_MEAN_DISTANCE})"));
        }
        let spread_failures = failures.len() - answer_failure_count;
        Report { rows, failures, spread_failures }
    }

    fn markdown(&self, name: &str, original: &str) -> String {
        let mut out = format!(
            "## {name} ({} chars)\n\n> {original}\n\n| Style | Length | Changed | Rewrite |\n|---|---|---|---|\n",
            original.chars().count()
        );
        for (style, answer, ratio, changed) in &self.rows {
            out.push_str(&format!(
                "| {} | {ratio:.2}× | {changed:.2} | {} |\n",
                style.name(),
                answer.replace('\n', " ").replace('|', "\\|")
            ));
        }
        for failure in &self.failures {
            out.push_str(&format!("\n- FAIL: {failure}"));
        }
        out.push_str("\n\n");
        out
    }
}

/// The one job a style must do, measured against the text. None when it did it, or has no
/// measurable job.
fn job_failure(style: &Style, original: &str, answer: &str, ratio: f64, changed: f64) -> Option<String> {
    let percent = |ratio: f64| format!("{:.0}%", ratio * 100.0);
    match style {
        Style::Concise if ratio > CONCISE_MAX_RATIO => {
            Some(format!("Concise kept {} of the length (max {})", percent(ratio), percent(CONCISE_MAX_RATIO)))
        }
        Style::Fix if changed > FIX_MAX_CHANGE => {
            Some(format!("Fix changed {changed:.2} of the words (max {FIX_MAX_CHANGE}): it rewrote the text"))
        }
        Style::Simplify if sentence_words(answer) >= sentence_words(original) => {
            Some("Simplify did not shorten the sentences".to_string())
        }
        Style::Summarize if ratio > SUMMARY_MAX_RATIO => {
            Some(format!("Summarize kept {} of the length (max {})", percent(ratio), percent(SUMMARY_MAX_RATIO)))
        }
        Style::Expand if !EXPAND_RATIOS.contains(&ratio) => Some(format!(
            "Expand made {} of the length ({} to {})",
            percent(ratio),
            percent(*EXPAND_RATIOS.start()),
            percent(*EXPAND_RATIOS.end())
        )),
        Style::Bullets if !is_bullet_list(answer) => Some("Bullets did not answer with a \"- \" list".to_string()),
        _ => None,
    }
}

/// Words per sentence, on average.
fn sentence_words(text: &str) -> f64 {
    let sentences: Vec<&str> =
        text.split(['.', '!', '?', '\n']).filter(|sentence| sentence.split_whitespace().next().is_some()).collect();
    let words: usize = sentences.iter().map(|sentence| sentence.split_whitespace().count()).sum();
    words as f64 / sentences.len().max(1) as f64
}

/// At least two points, and every line a "- " point.
fn is_bullet_list(text: &str) -> bool {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    lines.len() >= 2 && lines.iter().all(|line| line.starts_with("- "))
}

/// Digit runs in the answer that the text does not have: a cheap sign of an invented fact.
/// Thousands commas are dropped first, so "10000" written as "10,000" is the same number, and so
/// are full hours, so "10am" written as "10:00 AM" is the same time.
fn new_numbers(original: &str, answer: &str) -> Vec<String> {
    let numbers = |text: &str| -> BTreeSet<String> {
        let digits: String = text.replace(":00", "").chars().filter(|&c| c != ',').collect();
        digits.split(|c: char| !c.is_ascii_digit()).filter(|run| !run.is_empty()).map(str::to_string).collect()
    };
    numbers(answer).difference(&numbers(original)).cloned().collect()
}

fn length_ratio(original: &str, answer: &str) -> f64 {
    answer.chars().count() as f64 / original.chars().count().max(1) as f64
}

/// The distinct words of a text, in lower case, without punctuation.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// 1 minus the share of distinct words two texts have in common (Jaccard distance): 0 for the
/// same words, 1 for none in common. Case and punctuation are ignored, so a style that only
/// fixes capitals scores near 0.
fn word_distance(a: &str, b: &str) -> f64 {
    let (a, b) = (words(a), words(b));
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    1.0 - a.intersection(&b).count() as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_distance_ignores_case_and_punctuation_and_counts_shared_words() {
        assert_eq!(word_distance("Hey, I fixed it.", "hey i fixed it"), 0.0);
        // {the, port, broke} vs {the, port, failed}: 2 shared of 4 distinct.
        assert!((word_distance("the port broke", "The port failed!") - 0.5).abs() < 1e-9);
        assert_eq!(word_distance("alpha", "beta"), 1.0);
        assert_eq!(word_distance("doesn't", "doesnt"), 1.0, "apostrophes stay inside words");
    }

    #[test]
    fn near_identical_answers_and_a_narrow_set_fail() {
        let original = "the port was wrong so the deploy broke";
        let same = "The port was wrong, so the deploy broke.".to_string();
        let answers: Vec<(Style, String)> = PRESETS.iter().map(|s| (s.clone(), same.clone())).collect();
        let failures = Report::new(original, &answers).failures;
        assert!(failures.iter().any(|f| f.starts_with("Clear and Technical are 0.00")), "{failures:?}");
        assert!(failures.iter().any(|f| f.starts_with("the tones are 0.00 apart on average")), "{failures:?}");
    }

    #[test]
    fn each_action_is_held_to_its_own_job_and_not_to_the_tone_spread() {
        let original = "the deploy broke becuase the config had the wrong port, we should add a check for it";
        let check = |style: Style, answer: &str| Report::new(original, &[(style, answer.to_string())]).failures;
        let longer = |times: usize| vec![original; times].join(" ");

        assert!(
            check(Style::Fix, "The deploy broke because the config had the wrong port. We should add a check for it.")
                .is_empty()
        );
        assert!(
            !check(Style::Fix, "A wrong port in the config caused the outage; a validation step would prevent it.")
                .is_empty()
        );
        assert!(check(Style::Summarize, "Wrong port broke deploy; add check.").is_empty());
        assert!(
            !check(Style::Summarize, "The deploy broke because the config had the wrong port, so add a check.")
                .is_empty()
        );
        assert!(check(Style::Expand, &longer(1).replace("we should", "so I think that we should really")).is_empty());
        assert!(!check(Style::Expand, &longer(3)).is_empty(), "past twice the length");
        assert!(
            check(Style::Bullets, "- The deploy broke: wrong port in the config.\n- Add a check for it.").is_empty()
        );
        assert!(
            !check(Style::Bullets, "- The deploy broke because the config had the wrong port; add a check.").is_empty()
        );
        assert!(!check(Style::Simplify, original).is_empty(), "unchanged, and no shorter sentences");

        let invented = check(Style::Clear, "The deploy broke because port 8080 was wrong in the config. Add a check.");
        assert_eq!(invented, ["Clear added numbers the text does not have: 8080"]);
        let copied = check(Style::Clear, "The deploy broke on the login page since yesterday. Add a check.");
        assert_eq!(copied, ["Clear copied details from the example: login, yesterday"]);
        let monday = "the deploy broke on monday because the config had the wrong port";
        assert!(example_details(monday, "On Monday, the deploy broke: wrong port.").is_empty(), "in the text");
        let rows = "more than 10000 rows broke the deploy";
        assert!(new_numbers(rows, "More than 10,000 rows broke the deploy.").is_empty());
        assert_eq!(new_numbers(rows, "More than 10, 000 rows broke the deploy."), ["000", "10"], "a split number");
        assert!(new_numbers("the sync at 10am", "The sync at 10:00 AM.").is_empty());
        assert_eq!(new_numbers("the sync at 10am", "The sync at 10:30 AM."), ["30"]);
        // The eval cannot tell a clean text from a missed error, so an unchanged Fix passes.
        assert!(check(Style::Fix, original).is_empty());

        // Fix and Clear may read alike: only the tones are checked apart.
        let same = "The deploy broke because the config had the wrong port. We should add a check for it.";
        let answers = [(Style::Clear, same.to_string()), (Style::Fix, same.to_string())];
        assert!(Report::new(original, &answers).failures.is_empty());
    }

    #[test]
    fn the_report_fails_a_weak_concise_and_styles_that_read_the_same() {
        let original = "we should probably add a check so this does not happen again next time";
        let answers = vec![
            (Style::Clear, "We should add a check so this does not happen again next time.".to_string()),
            (Style::Concise, "We should add a check so this does not happen again.".to_string()),
            (Style::Professional, "I recommend that we introduce a validation step to prevent a recurrence.".into()),
            (Style::Technical, "ERROR: The server answered 500".to_string()),
        ];
        let failures = Report::new(original, &answers).failures;
        assert!(failures.iter().any(|f| f.starts_with("Concise kept")), "{failures:?}");
        assert!(
            !failures.iter().any(|f| f.starts_with("Clear and Concise are")),
            "0.15 apart is allowed: {failures:?}"
        );
        assert!(failures.contains(&"Technical failed".to_string()), "{failures:?}");
        assert!(!failures.iter().any(|f| f.starts_with("Clear and Professional")), "{failures:?}");
    }
}
