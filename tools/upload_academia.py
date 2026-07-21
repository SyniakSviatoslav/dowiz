#!/usr/bin/env python3
import requests, os, sys

HF_TOKEN = os.environ.get("HF_TOKEN", "")
REPO = "academia-matrix"  # HuggingFace dataset repo name
FILE = "academia_v1_matrix.bin"  # Will be stored as binary

if not HF_TOKEN:
    print("Потрібен HF_TOKEN: https://huggingface.co/settings/tokens")
    print("Запустіть: export HF_TOKEN=hf_...")
    sys.exit(1)

# Upload matrix snapshot
with open("/root/academia_matrix_v1.bin", "rb") as f:
    r = requests.put(
        f"https://huggingface.co/datasets/{{user}}/{REPO}/resolve/main/{FILE}",
        data=f, headers={"Authorization": f"Bearer {HF_TOKEN}"}
    )
print(f"Upload: {r.status_code}")

# Upload metadata
import json
meta = {"papers": 547536, "format": "N×8 u8", "dims": 8, "bytes_per_paper": 8}
requests.put(
    f"https://huggingface.co/datasets/{{user}}/{REPO}/resolve/main/metadata.json",
    data=json.dumps(meta), headers={"Authorization": f"Bearer {HF_TOKEN}"}
)
print("Metadata uploaded")
