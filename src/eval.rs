//! `retone --eval-styles`: sample texts through every preset style with the real prompt builder
//! and provider, printed side by side, so a style prompt change can be measured.
//!
//! A developer tool, run by hand: it prints the sample texts and the answers to stdout. The app
//! never calls it, so the no-history rule for user text is not touched.
//!
//! ```text
//! retone --eval-styles --model qwen2.5:3b [--endpoint http://127.0.0.1:8080/v1] [--text FILE]
//! retone --eval-styles --provider pi --model ollama/qwen2.5:3b
//! ```

#![forbid(unsafe_code)]

use crate::provider::{Cancel, Env, Outcome, Provider, local};
use crate::styles::{PRESETS, Style};
use std::collections::BTreeSet;

/// Concise must cut at least this much, or it did not do its one job.
const CONCISE_MAX_RATIO: f64 = 0.7;
/// Two answers closer than this read as the same rewrite.
const MIN_PAIR_DISTANCE: f64 = 0.1;
/// The styles together must spread at least this far: some pairs may fairly read alike (a
/// Professional and a Technical email), but not the whole set.
const MIN_MEAN_DISTANCE: f64 = 0.3;

const SAMPLES: [(&str, &str); 3] = [
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
        other => return Err(format!("unknown provider {other}")),
    };
    let mut env = Env::system().ok_or("HOME is not set")?;
    env.local_api_key = rune_core::env::var("RETONE_API_KEY").map(local::ApiKey);
    let own_text = match flag(args, "--text") {
        Some(path) => Some(std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?),
        None => None,
    };
    let samples: Vec<(&str, &str)> = match &own_text {
        Some(text) => vec![("text", text.trim())],
        None => SAMPLES.to_vec(),
    };

    println!("# Style eval: {} · {}\n", provider.kind().name(), provider.model());
    let mut all_passed = true;
    for (name, text) in samples {
        let answers: Vec<(Style, String)> = PRESETS
            .iter()
            .map(|style| {
                let Outcome { answer, .. } = provider.rewrite(style, text, &Cancel::default(), &env, &|_| {});
                (style.clone(), answer.unwrap_or_else(|error| format!("ERROR: {error}")))
            })
            .collect();
        let report = Report::new(text, &answers);
        print!("{}", report.markdown(name, text));
        all_passed &= report.failures.is_empty();
    }
    println!("{}", if all_passed { "PASS" } else { "FAIL" });
    Ok(all_passed)
}

struct Report<'a> {
    rows: Vec<(&'a Style, &'a str, f64, f64)>,
    failures: Vec<String>,
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
        let mut failures = Vec::new();
        for (style, answer, ratio, changed) in &rows {
            if answer.starts_with("ERROR:") {
                failures.push(format!("{} failed", style.name()));
            } else if *changed == 0.0 {
                failures.push(format!("{} returned the text unchanged", style.name()));
            }
            if **style == Style::Concise && *ratio > CONCISE_MAX_RATIO {
                failures.push(format!(
                    "Concise kept {:.0}% of the length (max {:.0}%)",
                    ratio * 100.0,
                    CONCISE_MAX_RATIO * 100.0
                ));
            }
        }
        let mut distances = Vec::new();
        for (i, a) in rows.iter().enumerate() {
            for b in &rows[i + 1..] {
                let distance = word_distance(a.1, b.1);
                if distance < MIN_PAIR_DISTANCE {
                    let (x, y) = (a.0.name(), b.0.name());
                    failures.push(format!("{x} and {y} are {distance:.2} apart (min {MIN_PAIR_DISTANCE})"));
                }
                distances.push(distance);
            }
        }
        let mean = distances.iter().sum::<f64>() / distances.len().max(1) as f64;
        if mean < MIN_MEAN_DISTANCE {
            failures.push(format!("the styles are {mean:.2} apart on average (min {MIN_MEAN_DISTANCE})"));
        }
        Report { rows, failures }
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

fn length_ratio(original: &str, answer: &str) -> f64 {
    answer.chars().count() as f64 / original.chars().count().max(1) as f64
}

/// 1 minus the share of distinct words two texts have in common (Jaccard distance): 0 for the
/// same words, 1 for none in common. Case and punctuation are ignored, so a style that only
/// fixes capitals scores near 0.
fn word_distance(a: &str, b: &str) -> f64 {
    let words = |text: &str| -> BTreeSet<String> {
        text.split(|c: char| !c.is_alphanumeric() && c != '\'')
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect()
    };
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
        assert!(failures.iter().any(|f| f.starts_with("the styles are 0.00 apart on average")), "{failures:?}");
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
