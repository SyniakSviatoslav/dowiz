# Proposal: GitHub issue & PR templates

**Status:** proposal — for the operator to apply.
**Why a proposal:** `.github/` is a protect-path and must not be edited by an
agent. This file contains the ready-to-paste template contents; the operator
creates the files under `.github/` when the repo goes public.

These give first-time contributors a clear on-ramp and keep issues/PRs
consistent with the project's ship-discipline and DCO norms.

---

## 1. `.github/ISSUE_TEMPLATE/config.yml`

Disables blank issues and routes security reports to the private channel.

```yaml
blank_issues_enabled: false
contact_links:
  - name: Security vulnerability
    url: https://github.com/<org>/<repo>/security/advisories/new
    about: Report vulnerabilities privately — do NOT open a public issue. See SECURITY.md.
  - name: Questions & discussion
    url: https://github.com/<org>/<repo>/discussions
    about: Ask usage questions or discuss ideas here.
```

## 2. `.github/ISSUE_TEMPLATE/bug_report.yml`

```yaml
name: Bug report
description: Something isn't working as documented
labels: [bug, triage]
body:
  - type: markdown
    attributes:
      value: |
        Thanks for filing a bug. Please do **not** report security issues here —
        see [SECURITY.md](../../SECURITY.md) for private disclosure.
  - type: textarea
    id: what-happened
    attributes:
      label: What happened?
      description: A clear description of the bug and what you expected instead.
    validations:
      required: true
  - type: textarea
    id: repro
    attributes:
      label: Steps to reproduce
      description: Minimal steps. Include the affected route/endpoint if relevant.
      placeholder: |
        1. Go to '/s/:slug' ...
        2. ...
        3. See error
    validations:
      required: true
  - type: input
    id: version
    attributes:
      label: Version / commit
      description: Tag, release, or commit SHA where you saw this.
  - type: textarea
    id: environment
    attributes:
      label: Environment
      description: OS, Node.js version, hosted vs self-hosted, browser (if UI).
  - type: textarea
    id: logs
    attributes:
      label: Logs / screenshots
      description: Relevant output. Redact any secrets, tokens, or personal data.
      render: shell
```

## 3. `.github/ISSUE_TEMPLATE/feature_request.yml`

```yaml
name: Feature request
description: Suggest an improvement or new capability
labels: [enhancement, triage]
body:
  - type: textarea
    id: problem
    attributes:
      label: Problem
      description: What problem are you trying to solve? Who is affected?
    validations:
      required: true
  - type: textarea
    id: proposal
    attributes:
      label: Proposed solution
      description: What would you like to see? Reference an ADR in docs/adr/ if it exists.
  - type: textarea
    id: alternatives
    attributes:
      label: Alternatives considered
  - type: checkboxes
    id: ethics
    attributes:
      label: Ethics
      options:
        - label: This request is consistent with the project's ethics charter (no military/warfare/surveillance-for-harm use). See CODE_OF_CONDUCT.md.
          required: true
```

## 4. `.github/PULL_REQUEST_TEMPLATE.md`

```markdown
## What & why

<!-- Describe the intent and the decisions, not just the diff. Link the issue/ADR. -->

Closes #

## Checklist

- [ ] Commits are **DCO-signed** (`git commit -s`) — see CONTRIBUTING.md
- [ ] `pnpm verify:all` is green (typecheck, lint, tests, RLS, migrations)
- [ ] Behaviour changes ship with a **red→green guardrail** (test / lint rule / hook)
- [ ] UI changes include an E2E assertion on real DOM; API changes include a request-level assertion
- [ ] Docs updated where relevant (README / ADR in `docs/adr/`)
- [ ] No secrets, credentials, tokens, or personal data added to the repo

## Proof

<!-- Paste the relevant test/gate output. "It should work" is not proof. -->
```

---

## Notes for the operator

- Replace `<org>/<repo>` placeholders with the real slug once decided.
- Enable **Private vulnerability reporting** (Settings → Security) so the
  security contact link resolves, and set the security/conduct contact addresses
  referenced in `SECURITY.md` and `CODE_OF_CONDUCT.md`.
- Consider enabling **Discussions** if you keep the contact link above.
