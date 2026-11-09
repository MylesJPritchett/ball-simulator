//! Parameter ranges. Any numeric value in a Scenario file may be written as
//! `{ min = a, max = b }` instead of a number; it is resolved from the Seed when
//! a simulation starts. Integer bounds resolve to an integer in `[min, max]`;
//! if either bound is a float the value is a float in `[min, max)`.
//!
//! Draw order is document order with keys sorted (TOML tables are sorted maps),
//! one draw per range. Adding, removing or renaming a ranged field changes the
//! draws that follow it.

use serde::Serialize;
use toml::{Table, Value};

use crate::sim::rng::Rng;

/// One ranged parameter and the value the Seed resolved it to.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedParam {
    /// Path in the Scenario file, e.g. `ball.speed` or `rules[1].percent`.
    pub field: String,
    pub value: ParamValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ParamValue {
    Int(i64),
    Float(f64),
}

impl ParamValue {
    pub fn as_f64(self) -> f64 {
        match self {
            ParamValue::Int(v) => v as f64,
            ParamValue::Float(v) => v,
        }
    }

    fn into_toml(self) -> Value {
        match self {
            ParamValue::Int(v) => Value::Integer(v),
            ParamValue::Float(v) => Value::Float(v),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Range {
    Int(i64, i64),
    Float(f64, f64),
}

/// Which concrete value every range takes.
pub(super) enum Pick<'a> {
    /// Every range at its lower bound (used to validate).
    Min,
    /// Every range at its upper bound (used to validate).
    Max,
    /// Drawn from the Seed's RNG; each draw is reported in `out`.
    Seeded {
        rng: &'a mut Rng,
        out: &'a mut Vec<ResolvedParam>,
    },
}

/// Replaces every range in `table` with a concrete value.
pub(super) fn resolve(table: &Table, pick: &mut Pick) -> Result<Table, String> {
    resolve_table("", table, pick)
}

fn resolve_table(prefix: &str, table: &Table, pick: &mut Pick) -> Result<Table, String> {
    table
        .iter()
        .map(|(key, value)| {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            Ok((key.clone(), resolve_value(&path, value, pick)?))
        })
        .collect()
}

fn resolve_value(path: &str, value: &Value, pick: &mut Pick) -> Result<Value, String> {
    match value {
        Value::Table(table) => match as_range(path, table)? {
            Some(range) => Ok(pick_value(path, range, pick).into_toml()),
            None => Ok(Value::Table(resolve_table(path, table, pick)?)),
        },
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| resolve_value(&format!("{path}[{i}]"), item, pick))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        other => Ok(other.clone()),
    }
}

/// A table with exactly the keys `min` and `max` is a range.
fn as_range(path: &str, table: &Table) -> Result<Option<Range>, String> {
    let (Some(min), Some(max)) = (table.get("min"), table.get("max")) else {
        return Ok(None);
    };
    if table.len() != 2 {
        return Ok(None);
    }
    let range = match (min, max) {
        (Value::Integer(lo), Value::Integer(hi)) => Range::Int(*lo, *hi),
        (lo, hi) => match (number(lo), number(hi)) {
            (Some(lo), Some(hi)) => Range::Float(lo, hi),
            _ => return Err(format!("{path}: range min and max must both be numbers")),
        },
    };
    let inverted = match range {
        Range::Int(lo, hi) => lo > hi,
        Range::Float(lo, hi) => lo.is_nan() || hi.is_nan() || lo > hi,
    };
    if inverted {
        return Err(format!(
            "{path}: inverted range, min ({min}) is greater than max ({max})"
        ));
    }
    Ok(Some(range))
}

fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Integer(i) => Some(*i as f64),
        Value::Float(f) => Some(*f),
        _ => None,
    }
}

fn pick_value(path: &str, range: Range, pick: &mut Pick) -> ParamValue {
    match (pick, range) {
        (Pick::Min, Range::Int(lo, _)) => ParamValue::Int(lo),
        (Pick::Min, Range::Float(lo, _)) => ParamValue::Float(lo),
        (Pick::Max, Range::Int(_, hi)) => ParamValue::Int(hi),
        (Pick::Max, Range::Float(_, hi)) => ParamValue::Float(hi),
        (Pick::Seeded { rng, out }, range) => {
            let value = match range {
                Range::Int(lo, hi) => {
                    let span = (hi as i128 - lo as i128 + 1) as u128;
                    ParamValue::Int((lo as i128 + (rng.next_u64() as u128 % span) as i128) as i64)
                }
                Range::Float(lo, hi) => ParamValue::Float(rng.range_f64(lo, hi)),
            };
            out.push(ResolvedParam {
                field: path.to_string(),
                value,
            });
            value
        }
    }
}
