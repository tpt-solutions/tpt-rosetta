//! Recording and summarising benchmark trials.
//!
//! The primary metric is cost per successful task, reported alongside the
//! success rate and its Wilson 95% interval. Token counts are kept with an
//! `exact` flag, so estimates and provider counts are never mixed silently.

use std::collections::BTreeMap;

/// Header of the trial CSV format. Fields must not contain commas or quotes.
pub const CSV_HEADER: &str = "task_id,category,encoding,model,input_tokens,output_tokens,tokens_exact,latency_ms,success,violations,retries,cost_usd";

/// Category value used in summaries that are not split by category.
pub const ALL_CATEGORIES: &str = "all";

/// One model call for one task in one encoding.
#[derive(Debug, Clone, PartialEq)]
pub struct Trial {
    pub task_id: String,
    pub category: String,
    pub encoding: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// `true` when the token counts came from the provider or an exact tokenizer.
    pub tokens_exact: bool,
    pub latency_ms: u64,
    /// The output met every acceptance criterion.
    pub success: bool,
    /// Semantic constraint violations found by scoring.
    pub violations: u32,
    pub retries: u32,
    /// `None` when the provider did not report cost.
    pub cost_usd: Option<f64>,
}

/// Aggregate results for one (encoding, model, category) group.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub encoding: String,
    pub model: String,
    /// [`ALL_CATEGORIES`] unless the summary was split by category.
    pub category: String,
    pub trials: usize,
    pub successes: usize,
    pub success_rate: f64,
    /// Lower bound of the Wilson 95% interval on the success rate.
    pub wilson_low: f64,
    /// Upper bound of the Wilson 95% interval on the success rate.
    pub wilson_high: f64,
    /// Sum of costs, or `None` if any trial lacks a cost.
    pub total_cost_usd: Option<f64>,
    /// Total cost divided by successful trials. `None` if costs are missing or
    /// there were no successes.
    pub cost_per_success_usd: Option<f64>,
    pub mean_latency_ms: f64,
    pub total_violations: u64,
    pub total_retries: u64,
    /// `true` only when every trial's token counts are exact.
    pub tokens_exact: bool,
}

const Z_95: f64 = 1.959_963_984_540_054;

/// Groups trials by (encoding, model) and summarises each group. Groups are
/// returned in sorted order, so output is deterministic.
pub fn summarise(trials: &[Trial]) -> Vec<Summary> {
    summarise_with(trials, |t| {
        (
            t.encoding.clone(),
            t.model.clone(),
            ALL_CATEGORIES.to_string(),
        )
    })
}

/// Like [`summarise`], but also splits each group by task category.
pub fn summarise_by_category(trials: &[Trial]) -> Vec<Summary> {
    summarise_with(trials, |t| {
        (t.encoding.clone(), t.model.clone(), t.category.clone())
    })
}

fn summarise_with<F>(trials: &[Trial], key: F) -> Vec<Summary>
where
    F: Fn(&Trial) -> (String, String, String),
{
    let mut groups: BTreeMap<(String, String, String), Vec<&Trial>> = BTreeMap::new();
    for trial in trials {
        groups.entry(key(trial)).or_default().push(trial);
    }
    groups
        .into_iter()
        .map(|((encoding, model, category), group)| {
            summarise_group(encoding, model, category, &group)
        })
        .collect()
}

fn summarise_group(encoding: String, model: String, category: String, group: &[&Trial]) -> Summary {
    let trials = group.len();
    let successes = group.iter().filter(|t| t.success).count();
    let (wilson_low, wilson_high) = wilson_interval(successes, trials);
    let success_rate = if trials == 0 {
        0.0
    } else {
        successes as f64 / trials as f64
    };

    let total_cost_usd: Option<f64> = group.iter().map(|t| t.cost_usd).sum();
    let cost_per_success_usd = match (total_cost_usd, successes) {
        (Some(total), n) if n > 0 => Some(total / n as f64),
        _ => None,
    };

    let total_latency: u64 = group.iter().map(|t| t.latency_ms).sum();
    let mean_latency_ms = if trials == 0 {
        0.0
    } else {
        total_latency as f64 / trials as f64
    };

    Summary {
        encoding,
        model,
        category,
        trials,
        successes,
        success_rate,
        wilson_low,
        wilson_high,
        total_cost_usd,
        cost_per_success_usd,
        mean_latency_ms,
        total_violations: group.iter().map(|t| u64::from(t.violations)).sum(),
        total_retries: group.iter().map(|t| u64::from(t.retries)).sum(),
        tokens_exact: group.iter().all(|t| t.tokens_exact),
    }
}

/// Wilson score interval for a binomial proportion at 95% confidence.
///
/// Returns `(0.0, 1.0)` when there are no trials.
pub fn wilson_interval(successes: usize, trials: usize) -> (f64, f64) {
    if trials == 0 {
        return (0.0, 1.0);
    }
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = Z_95 * Z_95;
    let denominator = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denominator;
    let margin = Z_95 * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denominator;
    ((centre - margin).max(0.0), (centre + margin).min(1.0))
}

/// Parses trials from CSV with the header [`CSV_HEADER`].
///
/// An empty `cost_usd` means the cost is unknown.
pub fn parse_csv(text: &str) -> Result<Vec<Trial>, String> {
    let mut lines = text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty());
    match lines.next() {
        Some((_, header)) if header.trim() == CSV_HEADER => {}
        _ => return Err(format!("expected CSV header: {CSV_HEADER}")),
    }

    let width = CSV_HEADER.split(',').count();
    let mut out = Vec::new();
    for (index, line) in lines {
        let number = index + 1;
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() != width {
            return Err(format!(
                "line {number}: expected {width} fields, found {}",
                fields.len()
            ));
        }
        let parse_u64 = |value: &str, name: &str| -> Result<u64, String> {
            value
                .parse()
                .map_err(|_| format!("line {number}: {name} must be an unsigned integer"))
        };
        let parse_bool = |value: &str, name: &str| -> Result<bool, String> {
            match value {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(format!("line {number}: {name} must be true or false")),
            }
        };
        let cost_usd = if fields[11].is_empty() {
            None
        } else {
            let cost: f64 = fields[11]
                .parse()
                .map_err(|_| format!("line {number}: cost_usd must be a number"))?;
            if !cost.is_finite() || cost < 0.0 {
                return Err(format!(
                    "line {number}: cost_usd must be a non-negative number"
                ));
            }
            Some(cost)
        };
        out.push(Trial {
            task_id: fields[0].to_string(),
            category: fields[1].to_string(),
            encoding: fields[2].to_string(),
            model: fields[3].to_string(),
            input_tokens: parse_u64(fields[4], "input_tokens")?,
            output_tokens: parse_u64(fields[5], "output_tokens")?,
            tokens_exact: parse_bool(fields[6], "tokens_exact")?,
            latency_ms: parse_u64(fields[7], "latency_ms")?,
            success: parse_bool(fields[8], "success")?,
            violations: parse_u64(fields[9], "violations")? as u32,
            retries: parse_u64(fields[10], "retries")? as u32,
            cost_usd,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trial(encoding: &str, success: bool, cost: Option<f64>) -> Trial {
        Trial {
            task_id: "t".into(),
            category: "bug-fixing".into(),
            encoding: encoding.into(),
            model: "m".into(),
            input_tokens: 100,
            output_tokens: 50,
            tokens_exact: true,
            latency_ms: 200,
            success,
            violations: 0,
            retries: 0,
            cost_usd: cost,
        }
    }

    #[test]
    fn cost_per_success_divides_by_successes_not_trials() {
        let trials = vec![
            trial("english", true, Some(0.01)),
            trial("english", false, Some(0.01)),
        ];
        let summary = &summarise(&trials)[0];
        assert_eq!(summary.total_cost_usd, Some(0.02));
        assert_eq!(summary.cost_per_success_usd, Some(0.02));
        assert_eq!(summary.successes, 1);
    }

    #[test]
    fn cost_per_success_is_none_without_successes() {
        let trials = vec![trial("english", false, Some(0.01))];
        assert_eq!(summarise(&trials)[0].cost_per_success_usd, None);
    }

    #[test]
    fn missing_cost_makes_total_unknown() {
        let trials = vec![trial("dsl", true, Some(0.01)), trial("dsl", true, None)];
        let summary = &summarise(&trials)[0];
        assert_eq!(summary.total_cost_usd, None);
        assert_eq!(summary.cost_per_success_usd, None);
    }

    #[test]
    fn groups_are_separated_by_encoding() {
        let trials = vec![trial("english", true, None), trial("dsl", false, None)];
        let summaries = summarise(&trials);
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].encoding, "dsl");
        assert_eq!(summaries[0].category, ALL_CATEGORIES);
    }

    #[test]
    fn category_split_separates_groups_by_category() {
        let mut other = trial("dsl", true, None);
        other.category = "planning-analysis".into();
        let trials = vec![trial("dsl", true, None), other];
        assert_eq!(summarise(&trials).len(), 1);
        let split = summarise_by_category(&trials);
        assert_eq!(split.len(), 2);
        assert_eq!(split[0].category, "bug-fixing");
        assert_eq!(split[1].category, "planning-analysis");
    }

    #[test]
    fn exactness_requires_every_trial_to_be_exact() {
        let mut estimated = trial("dsl", true, None);
        estimated.tokens_exact = false;
        let trials = vec![trial("dsl", true, None), estimated];
        assert!(!summarise(&trials)[0].tokens_exact);
    }

    #[test]
    fn wilson_interval_brackets_the_rate() {
        let (low, high) = wilson_interval(8, 10);
        assert!(low < 0.8 && 0.8 < high);
        assert!(low > 0.0 && high < 1.0);
    }

    #[test]
    fn wilson_interval_handles_extremes_and_empty_groups() {
        let (low, high) = wilson_interval(0, 10);
        assert_eq!(low, 0.0);
        assert!(high > 0.0 && high < 0.5);
        assert_eq!(wilson_interval(0, 0), (0.0, 1.0));
    }

    #[test]
    fn csv_round_trip_parses_fields() {
        let csv = format!(
            "{CSV_HEADER}\n\
             dev-01,bug-fixing,english,m1,120,40,true,900,true,0,1,0.0021\n\
             dev-01,bug-fixing,dsl,m1,80,30,false,700,false,2,0,\n"
        );
        let trials = parse_csv(&csv).unwrap();
        assert_eq!(trials.len(), 2);
        assert_eq!(trials[0].category, "bug-fixing");
        assert_eq!(trials[0].cost_usd, Some(0.0021));
        assert_eq!(trials[1].cost_usd, None);
        assert!(!trials[1].tokens_exact);
        assert_eq!(trials[1].violations, 2);
    }

    #[test]
    fn csv_rejects_bad_header_and_values() {
        assert!(parse_csv("wrong,header\n").is_err());
        let bad_bool = format!("{CSV_HEADER}\nt,c,e,m,1,1,yes,1,true,0,0,\n");
        assert!(parse_csv(&bad_bool).is_err());
        let negative_cost = format!("{CSV_HEADER}\nt,c,e,m,1,1,true,1,true,0,0,-1\n");
        assert!(parse_csv(&negative_cost).is_err());
    }

    #[test]
    fn csv_rejects_old_header_without_category() {
        let old = "task_id,encoding,model,input_tokens,output_tokens,tokens_exact,latency_ms,success,violations,retries,cost_usd\n";
        assert!(parse_csv(old).is_err());
    }
}
