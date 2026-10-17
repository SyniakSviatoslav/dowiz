#!/usr/bin/env python3
"""CI-REFS (W-INT2 #58): what of the GitHub workflows can be proven from inside the repo.

The workflows run on GitHub, and their run history is not readable from this box (`gh` is not
installed, and a passing YAML parse says nothing about a run). What IS checkable here, and what
this gate refuses:

  1. every workflow file parses as YAML and has `on:` and `jobs:`;
  2. every repo file a `run:` step names (a token that looks like a script or config path) exists,
     relative to the repo root, the step's `working-directory`, or a directory the step `cd`s into;
  3. every `secrets.X` and `vars.X` a workflow reads is NAMED in docs/operations.md, so the owner
     knows what to set -- a workflow whose secret nobody wrote down is a job that goes red, or
     worse skips, on the first run after someone else set up the repository;
  4. an `env:` at workflow or job level reads only the contexts GitHub offers THERE (github, needs,
     strategy, matrix, vars, secrets, inputs). `${{ runner.temp }}` in a job's env made GitHub refuse
     the whole qa-nightly file -- 0 jobs, a red run named after the file (891d7326, 2026-10-17) --
     while the YAML parsed and every other rule here was green.

Skipped on purpose, and printed: URLs, `$VAR`/`${{ }}` paths, globs, and paths inside a heredoc
that the step writes itself. Exit 0 green, 1 red (each finding named), 3 when there is nothing
to read (no workflows: a gate that counts nothing must not pass).

Usage: python3 tools/gates/ci-refs.py [ROOT]
"""
import os
import re
import sys

try:
    import yaml
except ImportError:  # loud: a gate that cannot read must not pass
    print("ci-refs: REFUSED -- python3 yaml is not installed")
    sys.exit(3)

ROOT = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "..", ".."))
WF = os.path.join(ROOT, ".github", "workflows")
DOC = os.path.join(ROOT, "docs", "operations.md")
PATH_RE = re.compile(r"(?<![\w$/.:-])((?:\.claude/|\.github/)?[A-Za-z0-9_][A-Za-z0-9_./-]*\.(?:sh|py|mjs|cjs|js|toml))\b")
SECRET_RE = re.compile(r"\b(secrets|vars)\.([A-Z][A-Z0-9_]*)")
EXPR_RE = re.compile(r"\$\{\{(.*?)\}\}", re.S)
CTX_RE = re.compile(r"(?<![\w.'\"])([a-z_]+)\s*(?:\.|\[)")
LEVEL_CTX = {"github", "needs", "strategy", "matrix", "vars", "secrets", "inputs"}
CD_RE = re.compile(r"(?:^|[;&|(]\s*|\n\s*)cd\s+([A-Za-z0-9_./-]+)")


def steps(doc):
    for job in (doc.get("jobs") or {}).values():
        default = ((job.get("defaults") or {}).get("run") or {}).get("working-directory")
        for st in job.get("steps") or []:
            if isinstance(st, dict) and isinstance(st.get("run"), str):
                yield st["run"], st.get("working-directory") or default


def level_env_misuse(doc):
    """(where, context) for each expression in a workflow- or job-level env that reads a context
    GitHub does not offer at that level."""
    envs = [("env", doc.get("env"))] + [(f"jobs.{k}.env", (j or {}).get("env")) for k, j in (doc.get("jobs") or {}).items() if isinstance(j, dict)]
    for where, env in envs:
        for key, val in (env or {}).items() if isinstance(env, dict) else []:
            for expr in EXPR_RE.findall(str(val)):
                # A quoted literal (a fallback URL) names no context.
                for ctx in CTX_RE.findall(re.sub(r"'[^']*'", "''", expr)):
                    if ctx not in LEVEL_CTX:
                        yield f"{where}.{key}", ctx


def heredoc_free(script):
    out, end = [], None
    for line in script.splitlines():
        if end is not None:
            if line.strip() == end:
                end = None
            continue
        m = re.search(r"<<-?\s*['\"]?([A-Za-z_]+)['\"]?", line)
        if m:
            end = m.group(1)
        out.append(line)
    return "\n".join(out)


def main():
    if not os.path.isdir(WF):
        print("ci-refs: REFUSED -- no .github/workflows here")
        return 3
    files = sorted(f for f in os.listdir(WF) if f.endswith((".yml", ".yaml")))
    if not files:
        print("ci-refs: REFUSED -- no workflow files")
        return 3
    doc = open(DOC, encoding="utf-8").read() if os.path.exists(DOC) else ""
    bad, checked, names = [], 0, set()
    for f in files:
        text = open(os.path.join(WF, f), encoding="utf-8").read()
        try:
            y = yaml.safe_load(text)
        except yaml.YAMLError as e:
            bad.append(f"{f}: does not parse: {str(e).splitlines()[0]}")
            continue
        # PyYAML reads the bare key `on` as the boolean True.
        if not isinstance(y, dict) or "jobs" not in y or not ({"on", True} & set(y)):
            bad.append(f"{f}: no `on:` or no `jobs:`")
            continue
        for where, ctx in level_env_misuse(y):
            bad.append(f"{f}: {where} reads `{ctx}`, a context GitHub does not offer at that level (move it to a step's env)")
        for kind, name in SECRET_RE.findall(text):
            names.add((kind, name, f))
        for script, wd in steps(y):
            # A single-quoted string is an argument (a test-name pattern, a message), not a path.
            body = re.sub(r"'[^'\n]*'", "''", heredoc_free(script))
            bases = [ROOT] + ([os.path.join(ROOT, wd)] if wd and "$" not in wd else [])
            bases += [os.path.join(b, d) for d in CD_RE.findall(body) if "$" not in d for b in list(bases)]
            for p in PATH_RE.findall(body):
                if "*" in p or p.startswith(("http", "//")):
                    continue
                checked += 1
                if not any(os.path.exists(os.path.join(b, p)) for b in bases):
                    bad.append(f"{f}: run step names {p}, which is not in the repo")
    for kind, name, f in sorted(names):
        if name not in doc:
            bad.append(f"{f}: {kind}.{name} is read but docs/operations.md never names it")
    for b in bad:
        print(f"ci-refs: {b}")
    verdict = "RED" if bad else "GREEN"
    print(f"ci-refs: {verdict} -- {len(files)} workflows, {checked} paths, {len(names)} secret/var reads, {len(bad)} findings")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
