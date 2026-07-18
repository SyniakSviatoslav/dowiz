// P45-W0 §4b.2 fence-check — enforce tools/ops-alert/fences.toml must-never assertions.
// Pure std. Exits 0 if all fences hold, 1 if any trips (S0 per spec -> RED CI).
// Repo root is located via the FENCES_ROOT env or by walking up from cwd to find fences.toml.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::exit;

struct Fence {
    id: String,
    kind: String,
    severity: String,
    note: String,
    // grep-absent
    pattern: String,
    glob: String,
    // cargo-feature-absent
    crate_name: String,
    feature: String,
    // workflow-present
    file: String,
    require_schedule: bool,
    require_probes: bool,
}

fn find_root() -> PathBuf {
    if let Ok(r) = env::var("FENCES_ROOT") {
        return PathBuf::from(r);
    }
    let mut dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    loop {
        if dir.join("tools/ops-alert/fences.toml").exists() {
            return dir;
        }
        if !dir.pop() {
            // fall back to cwd
            return env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        }
    }
}

// Minimal TOML parse for our fences.toml subset: top-level `key = int`,
// and [[fence]] arrays of `key = "value"` / `key = true`.
fn parse_fences(root: &Path) -> (usize, Vec<Fence>) {
    let txt = fs::read_to_string(root.join("tools/ops-alert/fences.toml"))
        .unwrap_or_else(|e| fail(&format!("cannot read fences.toml: {e}")));
    let mut fence_count = 0usize;
    let mut fences: Vec<Fence> = Vec::new();
    let mut cur: Option<Fence> = None;
    for raw in txt.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[fence]]" {
            if let Some(f) = cur.take() {
                fences.push(f);
            }
            cur = Some(Fence {
                id: String::new(),
                kind: String::new(),
                severity: String::new(),
                note: String::new(),
                pattern: String::new(),
                glob: String::new(),
                crate_name: String::new(),
                feature: String::new(),
                file: String::new(),
                require_schedule: false,
                require_probes: false,
            });
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim();
        if k == "fence_count" {
            fence_count = v.parse().unwrap_or(0);
            continue;
        }
        let f = match cur.as_mut() {
            Some(f) => f,
            None => continue,
        };
        match k {
            "id" => f.id = unq(v),
            "kind" => f.kind = unq(v),
            "severity" => f.severity = unq(v),
            "note" => f.note = unq(v),
            "pattern" => f.pattern = unq(v),
            "glob" => f.glob = unq(v),
            "crate" => f.crate_name = unq(v),
            "feature" => f.feature = unq(v),
            "file" => f.file = unq(v),
            "require_schedule" => f.require_schedule = v == "true",
            "require_probes" => f.require_probes = v == "true",
            _ => {}
        }
    }
    if let Some(f) = cur.take() {
        fences.push(f);
    }
    (fence_count, fences)
}

fn unq(s: &str) -> String {
    s.trim_matches('"').to_string()
}

fn fail(msg: &str) -> ! {
    eprintln!("fence-check: FATAL {msg}");
    exit(2);
}

fn walk_glob(root: &Path, glob: &str) -> Vec<PathBuf> {
    // support only the `**/*.sql` shape we use
    let mut out = Vec::new();
    let leaf = glob.trim_start_matches("**/").trim_start_matches("*/");
    let ext = Path::new(leaf)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().map(|n| n == "target").unwrap_or(false) {
                    continue;
                }
                stack.push(p);
            } else if p.extension().and_then(|x| x.to_str()) == Some(ext) {
                out.push(p);
            }
        }
    }
    out
}

fn check_grep_absent(root: &Path, f: &Fence) -> Result<(), String> {
    for p in walk_glob(root, &f.glob) {
        // skip documentation/proposal artifacts — only live code/migrations count
        let pstr = p.to_string_lossy();
        if pstr.contains("/docs/") {
            continue;
        }
        if let Ok(txt) = fs::read_to_string(&p) {
            if txt.contains(&f.pattern) {
                return Err(format!(
                    "{} found in {} ({} fence)",
                    f.pattern,
                    p.display(),
                    f.id
                ));
            }
        }
    }
    Ok(())
}

fn check_cargo_feature_absent(root: &Path, f: &Fence) -> Result<(), String> {
    // Locate <crate>/Cargo.toml anywhere in the repo (cross-repo crates are simply absent
    // here, which is GREEN — they are enforced in their own repo's CI).
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name()
                    .map(|n| n == "target" || n == ".git")
                    .unwrap_or(false)
                {
                    continue;
                }
                stack.push(p);
            } else if p.file_name().map(|n| n == "Cargo.toml").unwrap_or(false)
                && p.parent()
                    .map(|pp| {
                        pp.file_name()
                            .map(|n| n == f.crate_name.as_str())
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
            {
                let txt = fs::read_to_string(&p).unwrap_or_default();
                // find [features] default = [ ... ]
                if let Some(idx) = txt.find("[features]") {
                    let tail = &txt[idx..];
                    if let Some(didx) = tail.find("default") {
                        let rest = &tail[didx..];
                        if let Some(sidx) = rest.find('[') {
                            let arr = &rest[sidx..];
                            if let Some(eidx) = arr.find(']') {
                                let defaults = &arr[..=eidx];
                                if defaults.contains(&f.feature) {
                                    return Err(format!(
                                        "{} has default feature '{}' ({} fence)",
                                        f.crate_name, f.feature, f.id
                                    ));
                                }
                            }
                        }
                    }
                }
                return Ok(()); // crate found, feature absent -> GREEN
            }
        }
    }
    // crate not present in this repo -> GREEN (enforced elsewhere)
    Ok(())
}

fn check_workflow_present(root: &Path, f: &Fence) -> Result<(), String> {
    let p = root.join(&f.file);
    let txt = match fs::read_to_string(&p) {
        Ok(t) => t,
        Err(_) => {
            return Err(format!(
                "pager workflow {} missing ({} fence)",
                f.file, f.id
            ))
        }
    };
    if f.require_schedule && !txt.contains("schedule:") {
        return Err(format!("{} has no `schedule:` ({} fence)", f.file, f.id));
    }
    if f.require_probes {
        // PROBE_TARGETS may be a shell assignment (PROBE_TARGETS="...") or an
        // `env:` block entry (PROBE_TARGETS: "..."). Either form, non-empty, is OK.
        let have = txt.lines().any(|l| {
            let t = l.trim();
            if t.starts_with("PROBE_TARGETS=") {
                let v = t.trim_end_matches('\\').trim_end();
                v.contains('=') && !v.ends_with("PROBE_TARGETS=")
            } else if t.starts_with("PROBE_TARGETS:") {
                let v = t["PROBE_TARGETS:".len()..].trim().trim_end_matches('\\').trim();
                // reject empty or empty-quote values
                !v.is_empty() && v != "\"\"" && v != "''"
            } else {
                false
            }
        });
        if !have {
            return Err(format!(
                "{} PROBE_TARGETS empty/absent ({} fence)",
                f.file, f.id
            ));
        }
    }
    Ok(())
}

fn main() {
    let root = find_root();
    let (declared, fences) = parse_fences(&root);
    if declared != fences.len() {
        eprintln!(
            "::error::fence_count={declared} but {} [[fence]] blocks present — fence_count mismatch (weakening-guardrail class)",
            fences.len()
        );
        exit(1);
    }
    let mut trips = 0;
    for f in &fences {
        let res = match f.kind.as_str() {
            "grep-absent" => check_grep_absent(&root, f),
            "cargo-feature-absent" => check_cargo_feature_absent(&root, f),
            "workflow-present" => check_workflow_present(&root, f),
            other => Err(format!("unknown fence kind '{other}' for {}", f.id)),
        };
        match res {
            Ok(()) => println!("OK   {} ({})", f.id, f.severity),
            Err(e) => {
                trips += 1;
                eprintln!("::error::[{}] {}", f.severity, e);
            }
        }
    }
    if trips > 0 {
        eprintln!("fence-check: {trips} fence(s) tripped — CI RED");
        exit(1);
    }
    println!("fence-check: all {} fences hold", fences.len());
}
