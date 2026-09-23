# ML-KEM-768 ACVP vectors (FIPS 203)

Gate: `pq::kem::acvp_tests` (`src/pq/kem/acvp_tests.rs`), run with
`cd crates/dowiz-core && cargo test --lib pq::kem` (no feature flags; `serde`/`serde_json`
are dev-dependencies only).

## Source

- Repository: https://github.com/usnistgov/ACVP-Server, branch `master`
- Fetched 2026-09-23. `master` was at `975de31eb83d87039ec88934fdc47d8c312b892d` (2026-08-12).
- `gen-val/json-files/ML-KEM-keyGen-FIPS203/internalProjection.json`
  last changed in `15c0f3deeefbfa8cb6cd32a99e1ca3b738c66bf0` (2026-04-16);
  upstream sha256 `d7a62a2c3476957f56dd8d24f9004ea6776ccfe995ffe71a65bb9506dc9c7b1b`.
- `gen-val/json-files/ML-KEM-encapDecap-FIPS203/internalProjection.json`
  last changed in `ad33b3d9504491767f1aa76382464f3b3fa2359e` (2026-07-28);
  upstream sha256 `a556952ce869bb89c3a3196a701dad89647c193a34c86eafb61a9d710d5b810f`.

`internalProjection.json` carries both the prompts and the expected results.

## Filter

Top-level fields kept unchanged; `testGroups` reduced to the groups whose
`parameterSet == "ML-KEM-768"`; written with `json.dumps(v, indent=2)`:

```python
import json
u = json.load(open("internalProjection.json"))
v = {k: x for k, x in u.items() if k != "testGroups"}
v["testGroups"] = [g for g in u["testGroups"] if g["parameterSet"] == "ML-KEM-768"]
open("mlkem768-<mode>.json", "w").write(json.dumps(v, indent=2) + "\n")
```

Re-running this on the upstream files on 2026-09-23 reproduced both vendored files
byte-for-byte (the ML-KEM-512 and ML-KEM-1024 groups are the only thing removed).

## What the files hold (80 vectors)

| file | tgId | testType | function | tcIds | count |
|---|---|---|---|---|---|
| mlkem768-keygen.json | 2 | AFT | keyGen | 26–50 | 25 |
| mlkem768-encapdecap.json | 2 | AFT | encapsulation | 26–50 | 25 |
| mlkem768-encapdecap.json | 5 | VAL | decapsulation (valid + "modified ciphertext" → implicit rejection) | 86–95 | 10 |
| mlkem768-encapdecap.json | 9 | VAL | decapsulationKeyCheck (§7.3 hash check) | 126–135 | 10 |
| mlkem768-encapdecap.json | 10 | VAL | encapsulationKeyCheck (§7.2 modulus check) | 136–145 | 10 |

The encapDecap upstream file has `"isSample": true` at top level; the vectors are
nonetheless the full expected-results projection for vsId 42.
