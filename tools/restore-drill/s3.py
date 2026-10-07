#!/usr/bin/env python3
"""Read-only S3 (SigV4) for the weekly restore drill (W-PITR2): list a prefix, get one key.

Stdlib only, path-style, GET only -- the drill never writes the bucket. Credentials come from
the environment (`S3_KEY`, `S3_SECRET`, `S3_ENDPOINT`, `S3_REGION` default `auto` = R2), which
`weekly.sh` sources from /root/.dowiz_offsite_s3. The signer is the one `workers/api/src/cloud.rs`
writes in Rust; `--selftest` pins it to the AWS worked example, as cloud.rs's test does.

  s3.py list <bucket> <prefix>         one key per line
  s3.py get  <bucket> <key> <outfile>
  s3.py --selftest
"""
import datetime, hashlib, hmac, os, sys, urllib.parse, urllib.request

EMPTY = hashlib.sha256(b"").hexdigest()


def _h(key, msg):
    return hmac.new(key, msg.encode(), hashlib.sha256).digest()


def signing_key(secret, date, region, service):
    return _h(_h(_h(_h(("AWS4" + secret).encode(), date), region), service), "aws4_request")


def enc(s, slash=True):
    return urllib.parse.quote(s, safe="-_.~" + ("/" if slash else ""))


def request(method, bucket, key="", query=None):
    endpoint, region = os.environ["S3_ENDPOINT"].rstrip("/"), os.environ.get("S3_REGION", "auto")
    host = urllib.parse.urlparse(endpoint).netloc
    now = datetime.datetime.now(datetime.timezone.utc)
    amz, date = now.strftime("%Y%m%dT%H%M%SZ"), now.strftime("%Y%m%d")
    path = "/" + enc(bucket, False) + ("/" + enc(key) if key else "")
    q = "&".join(f"{enc(k, False)}={enc(v, False)}" for k, v in sorted((query or {}).items()))
    canon = f"{method}\n{path}\n{q}\nhost:{host}\nx-amz-content-sha256:{EMPTY}\nx-amz-date:{amz}\n\nhost;x-amz-content-sha256;x-amz-date\n{EMPTY}"
    scope = f"{date}/{region}/s3/aws4_request"
    sts = f"AWS4-HMAC-SHA256\n{amz}\n{scope}\n{hashlib.sha256(canon.encode()).hexdigest()}"
    sig = hmac.new(signing_key(os.environ["S3_SECRET"], date, region, "s3"), sts.encode(), hashlib.sha256).hexdigest()
    auth = f"AWS4-HMAC-SHA256 Credential={os.environ['S3_KEY']}/{scope}, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature={sig}"
    req = urllib.request.Request(endpoint + path + ("?" + q if q else ""), method=method,
                                 headers={"x-amz-date": amz, "x-amz-content-sha256": EMPTY, "Authorization": auth})
    with urllib.request.urlopen(req, timeout=120) as r:
        return r.read()


def keys_in(xml):
    out, rest = [], xml
    while "<Key>" in rest:
        rest = rest.split("<Key>", 1)[1]
        k, _, rest = rest.partition("</Key>")
        out.append(k)
    return out


def main(a):
    if a == ["--selftest"]:
        # AWS SigV4 worked example (the same vector cloud.rs pins).
        got = signing_key("wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY", "20150830", "us-east-1", "iam").hex()
        want = "c4afb1cc5771d871763a393e44b703571b55cc28424d1a5e86da6ed3c154a4b9"
        print("s3.py selftest", "OK" if got == want else f"FAIL {got}")
        return 0 if got == want else 1
    if len(a) == 3 and a[0] == "list":
        for k in keys_in(request("GET", a[1], query={"list-type": "2", "max-keys": "1000", "prefix": a[2]}).decode()):
            print(k)
        return 0
    if len(a) == 4 and a[0] == "get":
        open(a[3], "wb").write(request("GET", a[1], a[2]))
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
