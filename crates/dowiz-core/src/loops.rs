//! BP-20 — Orchestration state-machine + machine-executable preconditions.
//!
//! Replaces the *prose* `preconditions:` in `loops/*.yaml` with references to
//! the BP-08 `admit()` intake gate, adds a *programmatic* DRAFT→CERTIFIED gate
//! (a DRAFT loop cannot be dispatched by an ungated file-edit), and replaces
//! the `echo OK` placeholder `check_contracts` with a real structural check.
//!
//! Out of scope: the M1–M11 rubric text is untouched — only its *enforcement*
//! is made deterministic.
//!
//! no_std: the loop cards are parsed by a hand-rolled YAML *subset* parser
//! (see [`parse_loop_card`]) — no `serde`/`serde_yaml`. The subset covers
//! exactly what `loops/*.yaml` uses: top-level `key: value` scalars, trailing
//! `#` comments, `"…"`-quoted strings, int/float/bool scalars, `[a, b]` flow
//! lists, and the nested `certification:` block (`fields:`/`rules:` as
//! `- key: value` map lists). Unknown keys are ignored (serde ignore-unknown
//! parity).

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use crate::intake::{admit, BinOp, EtalonSpec, FieldSpec, IntakeError, RuleSpec};

/// Compiled lifecycle status of a loop card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopStatus {
    /// Not yet gated — must not dispatch without the CERTIFIED gate.
    #[default]
    Draft,
    /// Passed the programmatic certification gate — dispatchable.
    Certified,
}

impl LoopStatus {
    pub fn parse(s: &str) -> LoopStatus {
        match s.trim().to_uppercase().as_str() {
            "CERTIFIED" => LoopStatus::Certified,
            _ => LoopStatus::Draft,
        }
    }
}

/// One machine-checkable precondition field (fed to `admit()`).
#[derive(Debug, Clone, Default)]
pub struct CertField {
    pub name: String,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub required: bool,
    pub pinned: Option<i64>,
}

/// One cross-field rule (Tier-B arc-consistency), indices into `fields`.
#[derive(Debug, Clone)]
pub struct CertRule {
    pub a: usize,
    pub b: usize,
    pub op: String,
}

fn parse_op(op: &str) -> Option<BinOp> {
    match op.trim().to_uppercase().as_str() {
        "EQ" | "==" => Some(BinOp::Eq),
        "NE" | "!=" => Some(BinOp::Ne),
        "LT" | "<" => Some(BinOp::Lt),
        "LE" | "<=" => Some(BinOp::Le),
        "GT" | ">" => Some(BinOp::Gt),
        "GE" | ">=" => Some(BinOp::Ge),
        _ => None,
    }
}

/// The `certification:` block — machine-executable preconditions for the card.
#[derive(Debug, Clone, Default)]
pub struct Certification {
    pub fields: Vec<CertField>,
    pub rules: Vec<CertRule>,
}

impl Certification {
    /// Compile into an `EtalonSpec` consumable by `admit()`.
    fn to_spec(&self) -> Result<EtalonSpec, LoopError> {
        let mut fields: Vec<FieldSpec> = Vec::with_capacity(self.fields.len());
        for cf in &self.fields {
            let mut fs = FieldSpec::new(cf.name.clone());
            fs.min = cf.min;
            fs.max = cf.max;
            fs.required = cf.required;
            fs.pinned = cf.pinned;
            fields.push(fs);
        }
        let mut rules: Vec<RuleSpec> = Vec::with_capacity(self.rules.len());
        for cr in &self.rules {
            let op = parse_op(&cr.op).ok_or_else(|| {
                LoopError::ContractViolation(format!("certification rule op '{}' unknown", cr.op))
            })?;
            if cr.a >= fields.len() || cr.b >= fields.len() {
                return Err(LoopError::ContractViolation(format!(
                    "certification rule indexes out of range: a={} b={} n={}",
                    cr.a,
                    cr.b,
                    fields.len()
                )));
            }
            rules.push(RuleSpec {
                a: cr.a,
                b: cr.b,
                op,
            });
        }
        Ok(EtalonSpec {
            fields,
            rules,
            verify: "loop precondition admission".to_string(),
            verify_fn: None,
            nonlinear: false,
        })
    }
}

/// A parsed loop card (only the fields the orchestrator needs are typed; the
/// rest of the rubric is ignored — serde ignore-unknown parity).
#[derive(Debug, Clone, Default)]
pub struct LoopCard {
    pub id: String,
    pub version: f64,
    pub status: String,
    pub trigger: String,
    pub preconditions: Vec<String>,
    pub certification: Option<Certification>,
}

impl LoopCard {
    /// Parse a YAML loop card from a string (hand-rolled subset, no serde).
    pub fn from_yaml(src: &str) -> Result<Self, LoopError> {
        parse_loop_card(src)
    }

    pub fn status(&self) -> LoopStatus {
        LoopStatus::parse(&self.status)
    }
}

/// Why a loop was refused dispatch.
#[derive(Debug, Clone, PartialEq)]
pub enum LoopError {
    /// The certification preconditions are ill-posed (UNSAT / under-determined
    /// / non-reproducible). The loop is NOT dispatched.
    IllPosed(IntakeError),
    /// Status is DRAFT — the DRAFT→CERTIFIED gate refuses dispatch.
    NotCertified,
    /// A structural contract (id/version/trigger/status) is violated.
    ContractViolation(String),
    /// YAML parse failure.
    Parse(String),
}

/// Ticket returned on a successful, gated dispatch.
#[derive(Debug, Clone, PartialEq)]
pub struct DispatchTicket {
    pub id: String,
    pub version: f64,
}

/// The orchestration state-machine. `dispatch` is the single entry point that
/// enforces the three RED→GREEN gates.
pub struct Orchestrator;

impl Orchestrator {
    /// Decide whether a loop may be dispatched.
    ///
    /// Order of gates (fail-closed, stop-at-first):
    ///   1. If a `certification` block exists, compile it and run `admit()`.
    ///      Any `IntakeError` → `IllPosed` (no dispatch).
    ///   2. `status` must be `CERTIFIED`; a `DRAFT` card is refused (`NotCertified`).
    ///   3. `check_contracts()` — real structural validation (replaces `echo OK`).
    pub fn dispatch(card: &LoopCard) -> Result<DispatchTicket, LoopError> {
        // Gate 1: machine-executable preconditions via BP-08 admit().
        if let Some(cert) = &card.certification {
            let spec = cert.to_spec()?;
            admit(&spec).map_err(LoopError::IllPosed)?;
        }
        // Gate 2: DRAFT → CERTIFIED programmatic gate.
        if card.status() != LoopStatus::Certified {
            return Err(LoopError::NotCertified);
        }
        // Gate 3: real contract check (no placeholder).
        Self::check_contracts(card)?;
        Ok(DispatchTicket {
            id: card.id.clone(),
            version: card.version,
        })
    }

    /// Real structural validation of the card (replaces `echo OK`).
    fn check_contracts(card: &LoopCard) -> Result<(), LoopError> {
        if card.id.trim().is_empty() {
            return Err(LoopError::ContractViolation(
                "loop id must be non-empty".into(),
            ));
        }
        if card.version <= 0.0 {
            return Err(LoopError::ContractViolation(format!(
                "loop '{}' version must be > 0",
                card.id
            )));
        }
        if card.trigger.trim().is_empty() {
            return Err(LoopError::ContractViolation(format!(
                "loop '{}' trigger must be non-empty",
                card.id
            )));
        }
        if LoopStatus::parse(&card.status) == LoopStatus::Draft && !card.preconditions.is_empty() {
            // A draft that still carries prose preconditions but no certification
            // block is allowed to exist, but if it has a certification block that
            // we already validated above; here we only flag a malformed status.
        }
        Ok(())
    }
}

// ── Hand-rolled YAML subset parser (replaces serde_yaml::from_str) ──────────

/// Strip a trailing `#` comment (not inside a `"…"` quoted span).
fn strip_comment(s: &str) -> &str {
    let mut in_quote = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_quote = !in_quote,
            '#' if !in_quote => return &s[..i],
            _ => {}
        }
    }
    s
}

/// Unquote a scalar: trim + comment-strip + strip one pair of `"…"` quotes.
fn scalar_str(s: &str) -> String {
    let s = strip_comment(s).trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Split `line` into `(key, rest)` on the first `:`. Returns `None` when the
/// line has no colon or an empty key.
fn split_kv(line: &str) -> Option<(&str, &str)> {
    let colon = line.find(':')?;
    let key = line[..colon].trim();
    if key.is_empty() {
        return None;
    }
    Some((key, line[colon + 1..].trim_start()))
}

/// Parse an inline flow list `[a, b, c]` (single line).
fn flow_list(s: &str) -> Vec<String> {
    let s = s.trim();
    let inner = s
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .unwrap_or("");
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner.split(',').map(scalar_str).collect()
}

/// Indentation (leading spaces) of a line.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Advance `idx` past blank lines, returning the indent of the next content
/// line (or `None` at EOF).
fn peek_indent(lines: &[&str], idx: &mut usize) -> Option<usize> {
    while *idx < lines.len() {
        if lines[*idx].trim().is_empty() {
            *idx += 1;
        } else {
            return Some(indent_of(lines[*idx]));
        }
    }
    None
}

/// Parse a block list of scalars (`- "item"` lines) that follows a `key:` line.
/// `*idx` is positioned at the line AFTER the `key:` line.
fn block_scalar_list(lines: &[&str], idx: &mut usize) -> Vec<String> {
    let Some(item_indent) = peek_indent(lines, idx) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    while *idx < lines.len() {
        let l = lines[*idx];
        if l.trim().is_empty() {
            *idx += 1;
            continue;
        }
        let indent = indent_of(l);
        if indent < item_indent {
            break;
        }
        if indent == item_indent {
            if let Some(item) = l.trim_start().strip_prefix('-') {
                out.push(scalar_str(item));
                *idx += 1;
                continue;
            }
        }
        break;
    }
    out
}

/// Parse a list of `- key: value` maps (with deeper-indent continuations) that
/// follows a `fields:`/`rules:` line. `*idx` is positioned AFTER the key line.
fn block_map_list(lines: &[&str], idx: &mut usize) -> Result<Vec<BTreeMap<String, String>>, LoopError> {
    let Some(item_indent) = peek_indent(lines, idx) else {
        return Ok(Vec::new());
    };
    let mut items: Vec<BTreeMap<String, String>> = Vec::new();
    let mut cur: Option<BTreeMap<String, String>> = None;
    while *idx < lines.len() {
        let l = lines[*idx];
        if l.trim().is_empty() {
            *idx += 1;
            continue;
        }
        let indent = indent_of(l);
        if indent < item_indent {
            break;
        }
        let t = l.trim_start();
        if indent == item_indent {
            if let Some(item) = t.strip_prefix('-') {
                if let Some(m) = cur.take() {
                    items.push(m);
                }
                let mut m = BTreeMap::new();
                if let Some((k, v)) = split_kv(item) {
                    m.insert(k.to_string(), scalar_str(v));
                }
                cur = Some(m);
            }
            *idx += 1;
            continue;
        }
        // Deeper indent → continuation of the current map item.
        if let Some((k, v)) = split_kv(t) {
            if let Some(m) = cur.as_mut() {
                m.insert(k.to_string(), scalar_str(v));
            }
        }
        *idx += 1;
    }
    if let Some(m) = cur.take() {
        items.push(m);
    }
    Ok(items)
}

fn parse_i64(s: &str) -> Result<i64, LoopError> {
    s.trim()
        .parse::<i64>()
        .map_err(|_| LoopError::Parse(format!("bad integer: {s}")))
}

fn parse_usize(s: &str) -> Result<usize, LoopError> {
    s.trim()
        .parse::<usize>()
        .map_err(|_| LoopError::Parse(format!("bad index: {s}")))
}

fn parse_bool(s: &str) -> Result<bool, LoopError> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(LoopError::Parse(format!("bad bool: {other}"))),
    }
}

/// Parse the `certification:` block. `*idx` is at the `certification:` line;
/// on return `*idx` is at the first line past the block.
fn parse_certification(lines: &[&str], idx: &mut usize) -> Result<Certification, LoopError> {
    *idx += 1; // past `certification:`
    let Some(block_indent) = peek_indent(lines, idx) else {
        return Ok(Certification::default());
    };
    let mut cert = Certification::default();
    while *idx < lines.len() {
        let l = lines[*idx];
        if l.trim().is_empty() {
            *idx += 1;
            continue;
        }
        let indent = indent_of(l);
        if indent < block_indent {
            break; // back out of the certification block
        }
        let Some((key, _rest)) = split_kv(l.trim_start()) else {
            *idx += 1;
            continue;
        };
        match key {
            "fields" => {
                *idx += 1;
                let raw = block_map_list(lines, idx)?;
                cert.fields = raw
                    .into_iter()
                    .map(|m| {
                        let name = m.get("name").cloned().unwrap_or_default();
                        let min = match m.get("min") {
                            Some(v) => Some(parse_i64(v)?),
                            None => None,
                        };
                        let max = match m.get("max") {
                            Some(v) => Some(parse_i64(v)?),
                            None => None,
                        };
                        let required = match m.get("required") {
                            Some(v) => parse_bool(v)?,
                            None => false,
                        };
                        let pinned = match m.get("pinned") {
                            Some(v) => Some(parse_i64(v)?),
                            None => None,
                        };
                        Ok(CertField {
                            name,
                            min,
                            max,
                            required,
                            pinned,
                        })
                    })
                    .collect::<Result<Vec<_>, LoopError>>()?;
            }
            "rules" => {
                *idx += 1;
                let raw = block_map_list(lines, idx)?;
                cert.rules = raw
                    .into_iter()
                    .map(|m| {
                        let a = match m.get("a") {
                            Some(v) => parse_usize(v)?,
                            None => return Err(LoopError::Parse("rule missing 'a'".into())),
                        };
                        let b = match m.get("b") {
                            Some(v) => parse_usize(v)?,
                            None => return Err(LoopError::Parse("rule missing 'b'".into())),
                        };
                        let op = m.get("op").cloned().unwrap_or_default();
                        Ok(CertRule { a, b, op })
                    })
                    .collect::<Result<Vec<_>, LoopError>>()?;
            }
            _ => {
                *idx += 1;
            }
        }
    }
    Ok(cert)
}

/// Parse a full loop-card YAML document.
fn parse_loop_card(src: &str) -> Result<LoopCard, LoopError> {
    let lines: Vec<&str> = src.lines().collect();
    let mut card = LoopCard::default();
    let mut idx = 0usize;
    while idx < lines.len() {
        let line = lines[idx];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            idx += 1;
            continue;
        }
        // Only indent-0 keys are top-level fields.
        if indent_of(line) > 0 {
            idx += 1;
            continue;
        }
        let Some((key, rest)) = split_kv(line) else {
            idx += 1;
            continue;
        };
        match key {
            "id" => card.id = scalar_str(rest),
            "version" => {
                let v = scalar_str(rest);
                card.version = v
                    .parse::<f64>()
                    .map_err(|_| LoopError::Parse(format!("bad version: {v}")))?;
            }
            "status" => card.status = scalar_str(rest),
            "trigger" => card.trigger = scalar_str(rest),
            "preconditions" => {
                if rest.trim().is_empty() {
                    idx += 1;
                    card.preconditions = block_scalar_list(&lines, &mut idx);
                    continue; // block_scalar_list already advanced idx
                } else {
                    card.preconditions = flow_list(rest);
                }
            }
            "certification" => {
                card.certification = Some(parse_certification(&lines, &mut idx)?);
                continue; // parse_certification already advanced idx
            }
            _ => {}
        }
        idx += 1;
    }
    Ok(card)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cert_yaml(cert_body: &str, status: &str) -> String {
        format!(
            "id: test-loop\nversion: 1.0\nstatus: {status}\ntrigger: /test\ncertification:\n{cert_body}"
        )
    }

    // RED→GREEN: ill-posed certification (UNSAT: min > max) → reject, no dispatch.
    #[test]
    fn ill_posed_precondition_rejects_dispatch() {
        let yaml = cert_yaml(
            "  fields:\n    - name: x\n      min: 5\n      max: 1\n",
            "CERTIFIED",
        );
        let card = LoopCard::from_yaml(&yaml).unwrap();
        match Orchestrator::dispatch(&card) {
            Err(LoopError::IllPosed(_)) => {}
            other => panic!("expected IllPosed, got {:?}", other),
        }
    }

    // RED→GREEN: DRAFT status → refuse dispatch even with a valid precondition.
    #[test]
    fn draft_loop_not_dispatched_without_certified_gate() {
        let yaml = cert_yaml("  fields:\n    - name: x\n      pinned: 7\n", "DRAFT");
        let card = LoopCard::from_yaml(&yaml).unwrap();
        assert_eq!(Orchestrator::dispatch(&card), Err(LoopError::NotCertified));
    }

    // GREEN: CERTIFIED + well-formed precondition → dispatch allowed.
    #[test]
    fn certified_well_formed_loop_dispatches() {
        let yaml = cert_yaml("  fields:\n    - name: x\n      pinned: 7\n", "CERTIFIED");
        let card = LoopCard::from_yaml(&yaml).unwrap();
        let t = Orchestrator::dispatch(&card).expect("certified+valid must dispatch");
        assert_eq!(t.id, "test-loop");
    }

    // GREEN: real contract check — empty trigger → ContractViolation.
    #[test]
    fn missing_trigger_is_contract_violation() {
        let yaml = "id: no-trig\nversion: 1.0\nstatus: CERTIFIED\ntrigger: \"\"\n";
        let card = LoopCard::from_yaml(yaml).unwrap();
        assert!(matches!(
            Orchestrator::dispatch(&card),
            Err(LoopError::ContractViolation(_))
        ));
    }

    // RED→GREEN: under-determined spec (dof > 0) → IllPosed (never silently pick).
    #[test]
    fn under_determined_precondition_rejects() {
        let yaml = cert_yaml(
            "  fields:\n    - name: free\n      min: 0\n      max: 100\n",
            "CERTIFIED",
        );
        let card = LoopCard::from_yaml(&yaml).unwrap();
        match Orchestrator::dispatch(&card) {
            Err(LoopError::IllPosed(IntakeError::UnderDetermined { .. })) => {}
            other => panic!("expected UnderDetermined, got {:?}", other),
        }
    }

    // The hand-rolled parser must survive a full real loop card: comments,
    // quoted Ukrainian strings, flow lists, and a certification block.
    #[test]
    fn parses_real_card_shape_with_comments_and_flow_lists() {
        let yaml = "id: audit-gate\n\
                    version: 0.1\n\
                    status: DRAFT            # → /build-verify-loop verify\n\
                    trigger: \"/audit-gate [скоуп]\"\n\
                    preconditions: [\"dev-сервер+бекенд піднімаються\", \"shared-шар винесені\"]\n\
                    certification:\n  fields:\n    - name: admit_epoch\n      pinned: 1\n\
                    iron_principles: [proof-by-artifact-not-words, no-fake-green]\n";
        let card = LoopCard::from_yaml(yaml).unwrap();
        assert_eq!(card.id, "audit-gate");
        assert_eq!(card.status(), LoopStatus::Draft);
        assert_eq!(card.trigger, "/audit-gate [скоуп]");
        assert_eq!(card.preconditions.len(), 2);
        let cert = card.certification.expect("certification parsed");
        assert_eq!(cert.fields.len(), 1);
        assert_eq!(cert.fields[0].name, "admit_epoch");
        assert_eq!(cert.fields[0].pinned, Some(1));
    }
}
