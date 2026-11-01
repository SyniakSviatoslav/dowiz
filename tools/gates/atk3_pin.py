#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
"""W-MG-HOST: the pin. `const PINNED` in examples/read.mjs := `WASM_SHA256` of tools/verify/bebop-verify.mjs,
the one reader this tree ships. Run after every reader build (tools/verify/build.sh); tools/gates/atk3.sh
is RED until the two agree, so a reader change cannot leave read.mjs refusing, or trusting, the wrong one."""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sha = re.search(r'^const WASM_SHA256 = "([0-9a-f]{64})";$', open(ROOT + "/tools/verify/bebop-verify.mjs").read(), re.M)
if not sha:
    sys.exit("atk3_pin: no WASM_SHA256 in tools/verify/bebop-verify.mjs")
p = ROOT + "/examples/read.mjs"
t = open(p).read()
new, n = re.subn(r'^const PINNED = "[0-9a-f]{64}";$', 'const PINNED = "%s";' % sha.group(1), t, flags=re.M)
if n != 1:
    sys.exit("atk3_pin: no PINNED line in examples/read.mjs")
if new != t:
    open(p, "w").write(new)
print("atk3_pin: read.mjs pins %s" % sha.group(1))
