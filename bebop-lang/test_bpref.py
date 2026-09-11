#!/usr/bin/env python3
import sys
sys.path.insert(0, "/root/dowiz/bebop-lang/tools")
from tools.bpref import run

# Test c70_qdsl
with open("bench/parity_constructs/c70_qdsl.bp") as f:
    result = run(f.read())
print(f"c70_qdsl.bp => {result}")

# Test c70_qdsl_neg
with open("bench/parity_constructs/c70_qdsl_neg.bp") as f:
    result = run(f.read())
print(f"c70_qdsl_neg.bp => {result}")