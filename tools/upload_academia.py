#!/usr/bin/env python3
"""Upload Academia matrix to HuggingFace via zero-trace proxy pool.

Використовує proxy_redirect.py логіку: кожен аплоад через різний IP.
Якщо HF_TOKEN не задано — читає з .env або ~/.huggingface/token
"""
import os, sys, json, hashlib, requests

HF_TOKEN = os.environ.get("HF_TOKEN", "")
MATRIX_FILE = os.environ.get("MATRIX", "tools/academia_matrix_v1.bin")
REPO = "Delulu12/academia-matrix"

if not HF_TOKEN:
    # Try reading from huggingface config
    for path in [os.path.expanduser("~/.huggingface/token"), ".env"]:
        if os.path.exists(path):
            with open(path) as f: HF_TOKEN = f.read().strip()
            break

if not HF_TOKEN:
    print("❌ Потрібен HF_TOKEN: export HF_TOKEN=hf_...")
    sys.exit(1)

# Zero-trace proxy pool (from agent_browser + proxy_redirect)
PROXIES = os.environ.get("PROXIES", "").split(",") if os.environ.get("PROXIES") else []
# If no proxies, use direct connection (single IP)

# Upload matrix
print(f"📤 Uploading {MATRIX_FILE} to {REPO}...")
files = {"file": open(MATRIX_FILE, "rb")}
data = {"path": "academia_v1_matrix.bin", "repo_id": REPO}

for attempt in range(3):
    proxy = {"http": PROXIES[attempt % len(PROXIES)], "https": PROXIES[attempt % len(PROXIES)]} if PROXIES else None
    
    try:
        r = requests.post(
            f"https://huggingface.co/api/datasets/{REPO}/upload",
            headers={"Authorization": f"Bearer {HF_TOKEN}"},
            files=files, data=data, proxies=proxy, timeout=120
        )
        if r.status_code in (200, 201):
            print(f"  ✅ Uploaded (IP: {attempt if PROXIES else 'direct'})")
            break
        else:
            print(f"  ⚠ Attempt {attempt}: {r.status_code} — retrying")
    except Exception as e:
        print(f"  ⚠ Attempt {attempt}: {e}")

# Upload metadata
meta = {}
with open(MATRIX_FILE, "rb") as f:
    d = f.read()
    meta["papers"] = int.from_bytes(d[:4], "little")
    meta["sha3_256"] = hashlib.sha3_256(d).hexdigest()
meta["dims"] = 8
meta["format"] = "N×8 u8 matrix"
meta["version"] = 1
meta["note"] = "Академія Дмитра Євдокимова"

print(f"📄 Uploading metadata ({meta['papers']:,} papers)...")
requests.post(
    f"https://huggingface.co/api/datasets/{REPO}/upload",
    headers={"Authorization": f"Bearer {HF_TOKEN}"},
    files={"file": ("metadata.json", json.dumps(meta))},
    data={"path": "metadata.json", "repo_id": REPO},
    timeout=30
)

url = f"https://huggingface.co/datasets/{REPO}/resolve/main/academia_v1_matrix.bin"
print(f"\n✅ Matrix: {url}")
print(f"   Papers: {meta['papers']:,}")
print(f"   SHA3:   {meta['sha3_256'][:16]}...")
print(f"\nЗавантаження (будь-хто):")
print(f"   curl -L {url} -o academia_matrix.bin")
print(f"   # або через Academia::from_snapshot()")
