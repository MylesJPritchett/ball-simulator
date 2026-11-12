//! Video metadata written next to the `.mp4` as `meta.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::scenario::{ParamValue, Scenario};
use crate::sim::{SimRecord, Simulation};

pub const GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize)]
pub struct VideoMeta {
    pub title: String,
    pub description: String,
    pub hashtags: Vec<String>,
    /// Scenario id (file stem); with `seed` and `generator_version` this
    /// recreates the Video exactly.
    pub scenario: String,
    pub seed: u64,
    pub generator_version: String,
    /// Each Scenario range (by field path) and the value this Seed resolved it to.
    pub parameters: BTreeMap<String, ParamValue>,
}

impl VideoMeta {
    /// Fills the Scenario's `[meta]` templates with this Seed's values and
    /// merges its hashtags with [`SHARED_HASHTAGS`].
    pub fn new(scenario: &Scenario, record: &SimRecord) -> Self {
        let resolved = Simulation::new(scenario, record.seed).scenario().clone();
        let vars = template_vars(resolved.values(), record);
        let fill = |template: &str| {
            interpolate(template, &vars).expect("templates were checked when the Scenario loaded")
        };
        let spec = &scenario.meta;
        let title = fill(spec.title.as_deref().unwrap_or(DEFAULT_TITLE));
        let description = spec
            .description
            .as_deref()
            .map_or_else(|| title.clone(), fill);
        let tags: Vec<String> = spec.hashtags.iter().map(|t| fill(t)).collect();
        VideoMeta {
            title,
            description,
            hashtags: merge_hashtags(&tags, SHARED_HASHTAGS),
            scenario: scenario.id.clone(),
            seed: record.seed,
            generator_version: GENERATOR_VERSION.to_string(),
            parameters: record
                .parameters
                .iter()
                .map(|p| (p.field.clone(), p.value))
                .collect(),
        }
    }
}

/// Hashtags added to every Video, after the Scenario's own.
pub const SHARED_HASHTAGS: &[&str] = &[
    "satisfying",
    "oddlysatisfying",
    "physics",
    "simulation",
    "asmr",
    "shorts",
];

const DEFAULT_TITLE: &str = "{hook_title}";

/// Metadata templates (`[meta]` in a Scenario file, optional). Each may use
/// `{placeholders}`: any value in the Scenario file by its path (`ball.speed`,
/// `population_cap`, `rules[0].percent`, `hook_title`, ...), resolved for the
/// Seed, plus `seed`, `palette`, `instrument`, `scale`, `boundary_hits`
/// (every Boundary Hit on screen, including those during the Play-out),
/// `balls` (final ball count), `rings_settled` (Rings that settled) and
/// `duration` (Video length in whole seconds, including the Play-out and the
/// hold).
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetaSpec {
    /// Defaults to the Hook Title.
    pub title: Option<String>,
    /// Defaults to the title.
    pub description: Option<String>,
    /// Scenario hashtags, with or without `#`; merged with [`SHARED_HASHTAGS`].
    #[serde(default)]
    pub hashtags: Vec<String>,
}

/// Placeholders that come from the run rather than the Scenario file.
const RECORD_VARS: &[&str] = &[
    "seed",
    "palette",
    "instrument",
    "scale",
    "boundary_hits",
    "balls",
    "rings_settled",
    "duration",
];

fn template_vars(
    mut vars: BTreeMap<String, String>,
    record: &SimRecord,
) -> BTreeMap<String, String> {
    let values = [
        record.seed.to_string(),
        record.palette.clone(),
        record.sound.instrument.name().to_string(),
        record.sound.scale.name().to_string(),
        record.boundary_hits.len().to_string(),
        record.final_balls.len().to_string(),
        record.settles.len().to_string(),
        (record.ending.cut_time().round() as u64).to_string(),
    ];
    vars.extend(RECORD_VARS.iter().map(|k| k.to_string()).zip(values));
    vars
}

/// Checks every `[meta]` template only uses known placeholders.
pub(crate) fn check_templates(scenario: &Scenario) -> Result<(), String> {
    let mut vars = scenario.values();
    vars.extend(RECORD_VARS.iter().map(|k| (k.to_string(), String::new())));
    let spec = &scenario.meta;
    let named = [
        ("meta.title", &spec.title),
        ("meta.description", &spec.description),
    ];
    for (field, template) in named {
        if let Some(t) = template {
            interpolate(t, &vars).map_err(|e| format!("{field}: {e}"))?;
        }
    }
    for (i, t) in spec.hashtags.iter().enumerate() {
        interpolate(t, &vars).map_err(|e| format!("meta.hashtags[{i}]: {e}"))?;
    }
    Ok(())
}

/// Numbers as shown in metadata: at most one decimal, none if whole.
pub fn format_number(v: f64) -> String {
    let rounded = (v * 10.0).round() / 10.0;
    if rounded.fract() == 0.0 {
        format!("{rounded:.0}")
    } else {
        format!("{rounded:.1}")
    }
}

/// Fills `{name}` placeholders in a metadata template from `vars`; `{{` and
/// `}}` are literal braces. Unknown names and unclosed braces are errors.
pub fn interpolate(template: &str, vars: &BTreeMap<String, String>) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(i) = rest.find(['{', '}']) {
        out.push_str(&rest[..i]);
        let brace = &rest[i..i + 1];
        let after = &rest[i + 1..];
        if after.starts_with(brace) {
            out.push_str(brace);
            rest = &after[1..];
        } else if brace == "}" {
            return Err(format!(
                "unmatched '}}' in \"{template}\" (write '}}}}' for a brace)"
            ));
        } else {
            let Some(end) = after.find('}') else {
                return Err(format!("unclosed '{{' in \"{template}\""));
            };
            let name = &after[..end];
            let Some(value) = vars.get(name) else {
                let known: Vec<&str> = vars.keys().map(String::as_str).collect();
                return Err(format!(
                    "unknown placeholder {{{name}}} in \"{template}\" (known: {})",
                    known.join(", ")
                ));
            };
            out.push_str(value);
            rest = &after[end + 1..];
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// Hashtags as `#tag`: the Scenario's own first, then the shared pool, each
/// once (compared case-insensitively). Whitespace is removed; empty tags dropped.
pub fn merge_hashtags(scenario_tags: &[String], shared: &[&str]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    scenario_tags
        .iter()
        .map(String::as_str)
        .chain(shared.iter().copied())
        .filter_map(|tag| {
            let tag: String = tag
                .trim_start()
                .trim_start_matches('#')
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            (!tag.is_empty() && seen.insert(tag.to_lowercase())).then(|| format!("#{tag}"))
        })
        .collect()
}
