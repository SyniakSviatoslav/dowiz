//! Bebop guard — the Operating System baked in as native, falsifiable behavior (not a prompt).
//!
//! Ground truth: the repo's red-line globs (auth, money, RLS, migrations, bulk-edit) from
//! AGENTS.md / docs/agent-rules/INVARIANTS.md, and the Verified-by-Math rule (every gate must be
//! able to go RED on bad input — no false-green metrics). This module is the mechanical denial
//! layer for the AGENT SURFACE: it blocks file mutations before they happen, and it refuses to
//! certify a guardrail that cannot fail.

/// The repo's red-line globs — same set the product protects. Touching these requires explicit
/// human go-ahead, never a blanket permission.
pub const RED_LINE_GLOBS: &[&str] = &[
    "**/auth/**",
    "**/migrations/**",
    "**/rls/**",
    "**/*.sql",
    "**/packages/db/migrations/**",
    "**/money/**",
    "**/payments/**",
    "**/bulk-edit/**",
];

/// The agreed scope the agent may touch without re-asking (mirrors .opencode/scope.jsonc).
pub const DEFAULT_SCOPE_GLOBS: &[&str] = &[
    "tools/bebop/**",
    "crates/bebop/**",
    "docs/design/dowiz-agent-cli/**",
];

fn to_regex(glob: &str) -> String {
    // single-pass glob → regex: ** matches across segments, * within one, ? one char
    let mut re = String::new();
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    re.push_str(".*");
                    i += 1; // consume second '*'
                    if i + 1 < chars.len() && chars[i + 1] == '/' {
                        i += 1; // consume the slash after "**/"
                    }
                } else {
                    re.push_str("[^/]*");
                }
            }
            '?' => re.push_str("[^/]"),
            '.' | '+' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\' => {
                re.push('\\');
                re.push(c);
            }
            _ => re.push(c),
        }
        i += 1;
    }
    format!("^(?:{re})$")
}

fn glob_matches(glob: &str, path: &str) -> bool {
    let re = to_regex(glob);
    regex::Regex::new(&re)
        .map(|r| r.is_match(path))
        .unwrap_or(false)
}

/// True if `path` matches any glob. Path is matched both raw and relative to `cwd` so a glob like
/// `tools/bebop/**` matches an absolute repo path when cwd is the repo root.
pub fn matches_any(path: &str, globs: &[&str], cwd: &str) -> bool {
    let rel = if std::path::Path::new(path).is_absolute() {
        path.strip_prefix(cwd).unwrap_or(path)
    } else {
        path
    };
    let rel = rel.strip_prefix('/').unwrap_or(rel);
    globs.iter().any(|g| glob_matches(g, path) || glob_matches(g, rel))
}

/// The mechanical denial decision for a file mutation.
#[derive(PartialEq, Eq, Debug)]
pub enum GuardKind {
    RedLine,
    Scope,
    Ok,
}

pub fn guard_path(target: &str, cwd: &str) -> GuardKind {
    if matches_any(target, RED_LINE_GLOBS, cwd) {
        return GuardKind::RedLine;
    }
    if !matches_any(target, DEFAULT_SCOPE_GLOBS, cwd) {
        return GuardKind::Scope;
    }
    GuardKind::Ok
}

/// A falsifiable gate: it must go GREEN on good input AND RED on bad input. A gate that cannot
/// fail is a false-green and is rejected (Verified-by-Math).
pub struct Gate {
    pub name: &'static str,
    pub green: fn() -> bool,
    pub red: fn() -> bool,
}

pub fn certify(g: &Gate) -> Result<(), String> {
    let green_ok = (g.green)();
    let red_fails = !(g.red)(); // red case must make the assertion false
    if green_ok && red_fails {
        Ok(())
    } else if !green_ok {
        Err(format!("gate '{}' NOT green on good input — fix before ship", g.name))
    } else {
        Err(format!(
            "gate '{}' reads green but CANNOT go red — false-green, rejected",
            g.name
        ))
    }
}

/// Self-test: prove the red-line deny actually denies. The CLI refuses to start if this fails.
pub fn self_test() -> Result<(), String> {
    let deny = certify(&Gate {
        name: "redline-deny",
        green: || guard_path("tools/bebop/src/core.rs", "/repo").eq(&GuardKind::Ok),
        red: || guard_path("packages/db/migrations/002.sql", "/repo").eq(&GuardKind::RedLine),
    });
    deny?;
    let scope = certify(&Gate {
        name: "scope-block",
        green: || guard_path("tools/bebop/src/brand.rs", "/repo").eq(&GuardKind::Ok),
        red: || guard_path("apps/api/src/server.ts", "/repo").eq(&GuardKind::Scope),
    });
    scope?;
    Ok(())
}
