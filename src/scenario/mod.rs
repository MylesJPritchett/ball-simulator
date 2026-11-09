//! Scenario files: parsing and validation.
//!
//! A Scenario is a TOML file. Rules are listed as `[[rules]]` tables with a
//! `kind` key; all other keys in the table are that Rule's parameters and are
//! interpreted by the Rule itself (see [`crate::rules`]).

use std::path::Path;

use serde::Deserialize;

/// Errors from loading or validating a Scenario.
#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    #[error("could not read Scenario file {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid Scenario '{id}': {message}")]
    Parse { id: String, message: String },
    #[error("invalid Scenario '{id}': {message}")]
    Invalid { id: String, message: String },
}

/// A parsed, validated Scenario with concrete values.
#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    /// Identifier used in output folder names; the file stem for Scenario files.
    pub id: String,
    pub name: String,
    /// Shown above the Arena and used as the Video title. Defaults to `name`.
    pub hook_title: String,
    /// Hard time cap in seconds; always an End Condition.
    pub time_cap: f64,
    pub arena: ArenaSpec,
    pub ball: BallSpec,
    pub rules: Vec<RuleSpec>,
    pub end_condition: Option<RuleSpec>,
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
    arena: ArenaSpec,
    ball: BallSpec,
    #[serde(default)]
    rules: Vec<RuleSpec>,
    end_condition: Option<RuleSpec>,
}

impl Scenario {
    pub fn from_toml_str(id: &str, toml_src: &str) -> Result<Self, ScenarioError> {
        let file: ScenarioFile = toml::from_str(toml_src).map_err(|e| ScenarioError::Parse {
            id: id.to_string(),
            message: e.to_string(),
        })?;
        let scenario = Scenario {
            id: id.to_string(),
            hook_title: file.hook_title.unwrap_or_else(|| file.name.clone()),
            name: file.name,
            time_cap: file.time_cap,
            arena: file.arena,
            ball: file.ball,
            rules: file.rules,
            end_condition: file.end_condition,
        };
        scenario.validate()?;
        Ok(scenario)
    }

    pub fn from_file(path: &Path) -> Result<Self, ScenarioError> {
        let src = std::fs::read_to_string(path).map_err(|source| ScenarioError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let id = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "scenario".to_string());
        Self::from_toml_str(&id, &src)
    }

    fn invalid(&self, message: impl Into<String>) -> ScenarioError {
        ScenarioError::Invalid {
            id: self.id.clone(),
            message: message.into(),
        }
    }

    fn validate(&self) -> Result<(), ScenarioError> {
        if !is_positive(self.time_cap) {
            return Err(self.invalid("time_cap must be greater than 0"));
        }
        if !is_positive(self.arena.radius) {
            return Err(self.invalid("arena.radius must be greater than 0"));
        }
        if !(self.ball.radius > 0.0 && self.ball.radius < self.arena.radius) {
            return Err(
                self.invalid("ball.radius must be greater than 0 and smaller than arena.radius")
            );
        }
        // Building Rules and End Conditions validates their kinds and parameters.
        crate::rules::build_all(self).map_err(|m| self.invalid(m))?;
        crate::sim::end_condition::build(self).map_err(|m| self.invalid(m))?;
        Ok(())
    }
}

/// True for finite-or-infinite values above zero; false for zero, negatives and NaN.
fn is_positive(v: f64) -> bool {
    v > 0.0
}
