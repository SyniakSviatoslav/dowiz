#!/usr/bin/env python3
"""SHA256_VENUES as wrangler itself reads it (W-MG-TRANS, H1): the TOML parsed, never a sed.

    python3 tools/deploy/scheme_var.py <wrangler.toml>

Prints `<declared 0|1> <value>`. `[vars]` and every `[env.*.vars]` count; the first non-empty
value wins, because a flip in any environment is a flip. Exit 2 when the file cannot be parsed:
deploy.sh refuses before the upload, and rollback.sh counts it as flipped (a floor it cannot rule
out is a floor).
"""
import sys
import tomllib

VAR = "SHA256_VENUES"


def tables(doc):
    yield doc  # a bare key: what a hand-written or test toml carries
    yield doc.get("vars", {})
    for env in doc.get("env", {}).values():
        yield env.get("vars", {})


def main():
    try:
        with open(sys.argv[1], "rb") as f:
            doc = tomllib.load(f)
    except OSError:
        print("0 ")
        return 0
    except tomllib.TOMLDecodeError as e:
        print(f"scheme_var: {sys.argv[1]}: {e}", file=sys.stderr)
        return 2
    found = [t[VAR] for t in tables(doc) if VAR in t]
    value = next((str(v) for v in found if str(v).strip()), "")
    print(f"{1 if found else 0} {value}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
