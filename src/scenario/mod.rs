//! Scenario files: parsing and validation.
//!
//! A Scenario is a TOML file. Rules are listed as `[[rules]]` tables with a
//! `kind` key; all other keys in the table are that Rule's parameters and are
//! interpreted by the Rule itself (see [`crate::rules`]).
//!
//! Any numeric value may be a range (see [`range`]); a parsed [`Scenario`] keeps
//! its ranges and [`Scenario::resolve`] turns it into concrete values for a Seed.

mod range;

use std::path::Path;

use serde::Deserialize;

use crate::look::LookSpec;
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
#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    /// Identifier used in output folder names; the file stem for Scenario files.
    pub id: String,
    pub name: String,
    /// Shown above the Arena and used as the Video title. Defaults to `name`.
    pub hook_title: String,
    /// Hard time cap in seconds; always an End Condition.
    pub time_cap: f64,
    /// Maximum ball count; the simulation never exceeds it.
    pub population_cap: usize,
    pub arena: ArenaSpec,
    pub ball: BallSpec,
    pub rules: Vec<RuleSpec>,
    /// Required in files, so always `Some` once parsed.
    pub end_condition: Option<RuleSpec>,
    /// Thresholds a Seed must meet to be rendered.
    pub quality_gate: QualityGateSpec,
    /// The file as written, ranges included.
    template: toml::Table,
    /// For error messages: the file path, or the id for inline Scenarios.
    file: String,
    /// Palette, visual toggles and Counter (`[look]`, optional).
    pub look: LookSpec,
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
/// length in seconds (the action plus the ending's hold). Each defaults to
/// no limit; an End Condition that never fires before the time cap is always
/// rejected.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct QualityGateSpec {
    pub min_duration: f64,
    pub max_duration: f64,
    pub min_boundary_hits: usize,
}

impl Default for QualityGateSpec {
    fn default() -> Self {
        QualityGateSpec {
            min_duration: 0.0,
            max_duration: f64::INFINITY,
            min_boundary_hits: 0,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScenarioFile {
    name: String,
    hook_title: Option<String>,
    time_cap: f64,
    #[serde(default = "default_population_cap")]
    population_cap: usize,
    arena: ArenaSpec,
    ball: BallSpec,
    #[serde(default)]
    rules: Vec<RuleSpec>,
    end_condition: Option<RuleSpec>,
    #[serde(default)]
    quality_gate: QualityGateSpec,
    #[serde(default)]
    look: LookSpec,
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
        let parse_err = |message: String| ScenarioError::Parse {
            file: file.to_string(),
            message,
        };
        let template: toml::Table =
            toml::from_str(toml_src).map_err(|e| parse_err(e.to_string()))?;
        // Validate with every range at each end, which also type-checks every field.
        let lowest = Self::concrete(id, file, &template, &mut Pick::Min)?;
        let highest = Self::concrete(id, file, &template, &mut Pick::Max)?;
        lowest.validate()?;
        highest.validate()?;
        if highest.ball.radius >= lowest.arena.radius {
            return Err(
                lowest.invalid("ball.radius must be smaller than arena.radius across their ranges")
            );
        }
        Ok(Scenario { template, ..lowest })
    }

    /// Builds a Scenario from `template` with every range picked by `pick`.
    fn concrete(
        id: &str,
        file: &str,
        template: &toml::Table,
        pick: &mut Pick,
    ) -> Result<Self, ScenarioError> {
        let parse_err = |message: String| ScenarioError::Parse {
            file: file.to_string(),
            message,
        };
        let table = range::resolve(template, pick).map_err(parse_err)?;
        let file_data: ScenarioFile =
            serde_path_to_error::deserialize(toml::Value::Table(table.clone()))
                .map_err(|e| parse_err(with_path(&e.path().to_string(), e.inner())))?;
        let Some(end_condition) = file_data.end_condition else {
            return Err(parse_err(
                "missing [end_condition] (use kind = \"time_cap\" to end at the time cap)".into(),
            ));
        };
        Ok(Scenario {
            id: id.to_string(),
            hook_title: file_data
                .hook_title
                .unwrap_or_else(|| file_data.name.clone()),
            name: file_data.name,
            time_cap: file_data.time_cap,
            population_cap: file_data.population_cap,
            arena: file_data.arena,
            ball: file_data.ball,
            rules: file_data.rules,
            end_condition: Some(end_condition),
            quality_gate: file_data.quality_gate,
            look: file_data.look,
            template: table,
            file: file.to_string(),
        })
    }

    /// Resolves every range from `rng`, one draw per range in document order.
    /// Returns the concrete Scenario and the value each range resolved to.
    /// A Scenario without ranges is returned unchanged and draws nothing.
    pub fn resolve(&self, rng: &mut Rng) -> (Scenario, Vec<ResolvedParam>) {
        let mut params = Vec::new();
        let resolved = Self::concrete(
            &self.id,
            &self.file,
            &self.template,
            &mut Pick::Seeded {
                rng,
                out: &mut params,
            },
        )
        .expect("ranges were validated at both ends");
        (resolved, params)
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
        // Building Rules and End Conditions validates their kinds and parameters.
        for (i, spec) in self.rules.iter().enumerate() {
            crate::rules::build(spec).map_err(|m| self.invalid(format!("rules[{i}]: {m}")))?;
        }
        crate::sim::end_condition::build(self)
            .map_err(|m| self.invalid(format!("end_condition: {m}")))?;
        Ok(())
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
