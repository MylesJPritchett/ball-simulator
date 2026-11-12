//! Scenario files: parsing and validation.
//!
//! A Scenario is a TOML file. Rules are listed as `[[rules]]` tables with a
//! `kind` key; all other keys in the table are that Rule's parameters and are
//! interpreted by the Rule itself (see [`crate::rules`]).
//!
//! Any numeric value may be a range (see [`range`]); a parsed [`Scenario`] keeps
//! its ranges and [`Scenario::resolve`] turns it into concrete values for a Seed.

mod range;

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::look::LookSpec;
use crate::meta::MetaSpec;
use crate::sim::rng::Rng;
use range::Pick;
pub use range::{ParamValue, ResolvedParam};

/// Errors from loading or validating a Scenario.
#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    #[error("could not read Scenario file {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    /// `file` is the Scenario file path, or the id for inline Scenarios.
    #[error("invalid Scenario file {file}: {message}")]
    Parse { file: String, message: String },
    #[error("invalid Scenario file {file}: {message}")]
    Invalid { file: String, message: String },
}

/// A parsed, validated Scenario. If the file has ranges, the fields below
/// hold every range at its lower bound; use [`Scenario::resolve`] (which
/// [`crate::sim::run`] does) for a Seed's concrete values.
///
/// The file's keys deserialize straight into this struct (after ranges are
/// picked): a new Scenario key is one field here plus its validation.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Identifier used in output folder names; the file stem for Scenario files.
    #[serde(skip)]
    pub id: String,
    pub name: String,
    /// Shown above the Arena and used as the Video title. Defaults to `name`.
    #[serde(default)]
    pub hook_title: String,
    /// Hard time cap in seconds: a safety net, not an End Condition. A Seed
    /// that reaches it is rejected by the Quality Gate.
    pub time_cap: f64,
    /// Maximum ball count; the simulation never exceeds it.
    #[serde(default = "default_population_cap")]
    pub population_cap: usize,
    pub arena: ArenaSpec,
    pub ball: BallSpec,
    #[serde(default)]
    pub rules: Vec<RuleSpec>,
    /// The Scenario's End Condition, which must reach a Terminal State.
    pub end_condition: RuleSpec,
    /// Thresholds a Seed must meet to be rendered.
    #[serde(default)]
    pub quality_gate: QualityGateSpec,
    /// Pinned instrument / scale / pitch mode (`[sound]`).
    #[serde(default)]
    pub sound: crate::audio::music::SoundSpec,
    /// Palette, visual toggles and Counter (`[look]`, optional).
    #[serde(default)]
    pub look: LookSpec,
    /// Title, description and hashtag templates (`[meta]`, optional).
    #[serde(default)]
    pub meta: MetaSpec,
    /// The file's TOML: as written (ranges included) on a parsed Scenario,
    /// with every range resolved on a [`Scenario::resolve`]d one.
    #[serde(skip)]
    raw: toml::Table,
    /// For error messages: the file path, or the id for inline Scenarios.
    #[serde(skip)]
    file: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArenaSpec {
    /// Arena radius in simulation units. Render scales the Arena to fit the layout.
    pub radius: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BallSpec {
    pub radius: f64,
    /// Initial speed in simulation units per second.
    pub speed: f64,
    /// Speed limit in units per second, always enforced on every ball.
    #[serde(default = "default_max_speed")]
    pub max_speed: f64,
}

/// Max speed for Scenarios that don't set one.
pub const DEFAULT_MAX_SPEED: f64 = 5000.0;

fn default_max_speed() -> f64 {
    DEFAULT_MAX_SPEED
}

/// Quality Gate thresholds (`[quality_gate]`, optional). Durations are Video
/// length in seconds (the action plus the Ending's Play-out and hold). Each
/// defaults to no limit; an End Condition that never fires before the time cap
/// is always rejected.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct QualityGateSpec {
    pub min_duration: f64,
    pub max_duration: f64,
    pub min_boundary_hits: usize,
    /// Longest time in seconds from the Film first dropping below
    /// [`crate::sim::quality_gate::FINAL_STRETCH_BELOW`] to it being cleared
    /// (or to the End Condition, if it never is). `None` means no check.
    pub max_final_stretch: Option<f64>,
    /// Largest ball radius, as a fraction of the Arena radius, allowed when
    /// the run ends. `None` means no check.
    pub max_ball_radius_at_end: Option<f64>,
}

impl Default for QualityGateSpec {
    fn default() -> Self {
        QualityGateSpec {
            min_duration: 0.0,
            max_duration: f64::INFINITY,
            min_boundary_hits: 0,
            max_final_stretch: None,
            max_ball_radius_at_end: None,
        }
    }
}

/// A `kind` plus free-form parameters. Used for both Rules and End Conditions.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RuleSpec {
    pub kind: String,
    #[serde(flatten)]
    pub params: toml::Table,
}

impl Scenario {
    pub fn from_toml_str(id: &str, toml_src: &str) -> Result<Self, ScenarioError> {
        Self::parse(id, id, toml_src)
    }

    pub fn from_file(path: &Path) -> Result<Self, ScenarioError> {
        let file = path.display().to_string();
        let src = std::fs::read_to_string(path).map_err(|source| ScenarioError::Io {
            path: file.clone(),
            source,
        })?;
        let id = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "scenario".to_string());
        Self::parse(&id, &file, &src)
    }

    fn parse(id: &str, file: &str, toml_src: &str) -> Result<Self, ScenarioError> {
        let raw: toml::Table =
            toml::from_str(toml_src).map_err(|e| parse_error(file, e.to_string()))?;
        // Validate with every range at each end, which also type-checks every field.
        let lowest = Self::concrete(id, file, &raw, &mut Pick::Min)?;
        let highest = Self::concrete(id, file, &raw, &mut Pick::Max)?;
        lowest.validate()?;
        highest.validate()?;
        if highest.ball.radius >= lowest.arena.radius {
            return Err(
                lowest.invalid("ball.radius must be smaller than arena.radius across their ranges")
            );
        }
        Ok(Scenario { raw, ..lowest })
    }

    /// Builds a Scenario from `raw` with every range picked by `pick`.
    fn concrete(
        id: &str,
        file: &str,
        raw: &toml::Table,
        pick: &mut Pick,
    ) -> Result<Self, ScenarioError> {
        let table = range::resolve(raw, pick).map_err(|m| parse_error(file, m))?;
        let mut scenario: Scenario =
            serde_path_to_error::deserialize(toml::Value::Table(table.clone())).map_err(|e| {
                // Missing fields are reported last, after every other problem.
                let message = if e
                    .inner()
                    .to_string()
                    .contains("missing field `end_condition`")
                {
                    "missing [end_condition] (it must reach a Terminal State, \
                     e.g. kind = \"arena_filled\")"
                        .to_string()
                } else {
                    with_path(&e.path().to_string(), e.inner())
                };
                parse_error(file, message)
            })?;
        if scenario.hook_title.is_empty() {
            scenario.hook_title = scenario.name.clone();
        }
        scenario.id = id.to_string();
        scenario.file = file.to_string();
        scenario.raw = table;
        Ok(scenario)
    }

    /// Resolves every range from `rng`, one draw per range in document order.
    /// Returns the concrete Scenario and the value each range resolved to.
    /// A Scenario without ranges is returned unchanged and draws nothing.
    pub fn resolve(&self, rng: &mut Rng) -> (Scenario, Vec<ResolvedParam>) {
        let mut params = Vec::new();
        let resolved = Self::concrete(
            &self.id,
            &self.file,
            &self.raw,
            &mut Pick::Seeded {
                rng,
                out: &mut params,
            },
        )
        .expect("ranges were validated at both ends");
        (resolved, params)
    }

    /// Every scalar value in the file by its path (`ball.speed`,
    /// `rules[1].percent`), plus `hook_title`; on a resolved Scenario these are
    /// the Seed's values. Metadata templates are filled in from these.
    pub fn values(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        flatten("", &toml::Value::Table(self.raw.clone()), &mut out);
        out.retain(|k, _| !k.starts_with("meta."));
        out.insert("hook_title".into(), self.hook_title.clone());
        out
    }

    fn invalid(&self, message: impl Into<String>) -> ScenarioError {
        ScenarioError::Invalid {
            file: self.file.clone(),
            message: message.into(),
        }
    }

    fn validate(&self) -> Result<(), ScenarioError> {
        if !is_positive(self.time_cap) {
            return Err(self.invalid("time_cap must be greater than 0"));
        }
        if self.population_cap == 0 {
            return Err(self.invalid("population_cap must be at least 1"));
        }
        if !is_positive(self.arena.radius) {
            return Err(self.invalid("arena.radius must be greater than 0"));
        }
        if !(self.ball.radius > 0.0 && self.ball.radius < self.arena.radius) {
            return Err(
                self.invalid("ball.radius must be greater than 0 and smaller than arena.radius")
            );
        }
        self.look.validate().map_err(|m| self.invalid(m))?;
        crate::meta::check_templates(self).map_err(|m| self.invalid(m))?;
        if !(self.ball.max_speed.is_finite() && self.ball.max_speed > 0.0) {
            return Err(self.invalid("ball.max_speed must be a finite number greater than 0"));
        }
        let gate = &self.quality_gate;
        if !(gate.min_duration >= 0.0 && gate.max_duration > 0.0) {
            return Err(self.invalid(
                "quality_gate.min_duration must be at least 0 and max_duration greater than 0",
            ));
        }
        if gate.min_duration > gate.max_duration {
            return Err(
                self.invalid("quality_gate.min_duration must not exceed quality_gate.max_duration")
            );
        }
        for (key, max) in [
            ("max_final_stretch", gate.max_final_stretch),
            ("max_ball_radius_at_end", gate.max_ball_radius_at_end),
        ] {
            if max.is_some_and(|max| max.is_nan() || max <= 0.0) {
                return Err(self.invalid(format!("quality_gate.{key} must be greater than 0")));
            }
        }
        let films = self.rules.iter().filter(|r| r.kind == crate::rules::FILM);
        if films.count() > 1 {
            return Err(self.invalid(format!(
                "rules: at most one '{}' Rule per Scenario",
                crate::rules::FILM
            )));
        }
        // Building Rules and End Conditions validates their kinds and parameters.
        let sets_max_speed = self
            .raw
            .get("ball")
            .and_then(|ball| ball.get("max_speed"))
            .is_some();
        for (i, spec) in self.rules.iter().enumerate() {
            let rule =
                crate::rules::build(spec).map_err(|m| self.invalid(format!("rules[{i}]: {m}")))?;
            if rule.needs_max_speed() && !sets_max_speed {
                return Err(self.invalid(format!(
                    "rules[{i}]: Rule '{}' needs ball.max_speed",
                    spec.kind
                )));
            }
        }
        crate::sim::end_condition::build(self)
            .map_err(|m| self.invalid(format!("end_condition: {m}")))?;
        Ok(())
    }
}

fn flatten(path: &str, value: &toml::Value, out: &mut BTreeMap<String, String>) {
    let text = match value {
        toml::Value::Table(table) => {
            for (key, v) in table {
                let path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                flatten(&path, v, out);
            }
            return;
        }
        toml::Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                flatten(&format!("{path}[{i}]"), v, out);
            }
            return;
        }
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => crate::meta::format_number(*f),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(d) => d.to_string(),
    };
    out.insert(path.to_string(), text);
}

fn parse_error(file: &str, message: impl Into<String>) -> ScenarioError {
    ScenarioError::Parse {
        file: file.to_string(),
        message: message.into(),
    }
}

/// Prefixes a deserialization error with the field it happened at.
pub(crate) fn with_path(path: &str, err: impl std::fmt::Display) -> String {
    if path == "." || path.is_empty() {
        err.to_string()
    } else {
        format!("{path}: {err}")
    }
}

/// Default Population Cap when a Scenario does not set one.
pub const DEFAULT_POPULATION_CAP: usize = 2000;

fn default_population_cap() -> usize {
    DEFAULT_POPULATION_CAP
}

/// True for finite-or-infinite values above zero; false for zero, negatives and NaN.
fn is_positive(v: f64) -> bool {
    v > 0.0
}
