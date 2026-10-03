#!/bin/sh
# CI-REFS (W-INT2 #58) -- the in-repo half of the GitHub workflows: each parses, each file a
# `run:` step names exists, each secret/var it reads is named in docs/operations.md. The rules
# live in ci-refs.py (read its header); ci-refs.prove.sh triggers each before it is trusted.
exec python3 "$(dirname "$0")/ci-refs.py" ${1:+"$1"}
