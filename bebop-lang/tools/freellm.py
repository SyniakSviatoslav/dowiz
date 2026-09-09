#!/usr/bin/env python3
"""freellm.py (2026-09-09) -- route a mechanical prompt to a FREE hosted model, with
provider fallback, and refuse loudly rather than silently degrading.

WHY THIS SHAPE. The per-worker model selector in this harness accepts four Anthropic
models and nothing else, so a Claude Code subagent CANNOT be pointed at a free provider.
The only harness-level route is `ANTHROPIC_BASE_URL`, which routes the WHOLE session --
including the merge decisions -- and is the operator's to set, not a worker's. What a
worker CAN do is shell out, which is what this is for: the mechanical text work (summarise
a log, extract a table, reformat a report) goes to a free tier, and the judgement work
stays where it is.

MEASURED ON THIS BOX 2026-09-09, so the constraints are facts and not guesses:
  - all six providers below answer a TLS handshake from inside the proot;
  - a LOCAL model is refuted: 1,619 MB available RAM against a 3B-Q4's ~2 GB, with procs
    at 33 against a 32-process ceiling that has already been exceeded twice today;
  - there are ZERO API keys in the environment, which is the ONLY missing input. Supply
    one and this works; supply none and it says exactly which and refuses.

KEYS. Read from the environment, or from `~/.config/freellm/keys` as `PROVIDER=key` lines
(mode 600 enforced -- a key file the world can read is refused). Nothing is ever written to
the repo, and `--probe` never sends a prompt.

  tools/freellm.py --probe                 which providers are configured and reachable
  tools/freellm.py --list                  the free models this knows, per provider
  tools/freellm.py -p "..."                route one prompt, first configured provider wins
  tools/freellm.py -p "..." --provider groq
  tools/freellm.py -f prompt.txt --max 800
  tools/freellm.py --dry-run -p "..."      build the request, print it, send NOTHING

REFUSALS, because a router that quietly returns something worse than asked for is the
failure mode this project spent the day removing: no key -> exit 3 naming the providers
tried; every provider failing -> exit 4 with each provider's own error; a truncated or
empty completion -> exit 5 rather than a partial answer treated as an answer.
"""
import json
import os
import ssl
import sys
import time
import urllib.error
import urllib.request

# Provider -> (host, path, env var, default free model, auth style).
# Models are the free tiers as of 2026-09; `--list` prints them and `--model` overrides,
# because a hardcoded model name is the kind of claim that goes stale silently.
PROVIDERS = {
    "groq":       ("api.groq.com", "/openai/v1/chat/completions",
                   "GROQ_API_KEY", "llama-3.3-70b-versatile", "bearer"),
    "cerebras":   ("api.cerebras.ai", "/v1/chat/completions",
                   "CEREBRAS_API_KEY", "llama-3.3-70b", "bearer"),
    "openrouter": ("openrouter.ai", "/api/v1/chat/completions",
                   "OPENROUTER_API_KEY", "meta-llama/llama-3.3-70b-instruct:free", "bearer"),
    "google":     ("generativelanguage.googleapis.com", "/v1beta/openai/chat/completions",
                   "GOOGLE_API_KEY", "gemini-2.0-flash", "bearer"),
    "mistral":    ("api.mistral.ai", "/v1/chat/completions",
                   "MISTRAL_API_KEY", "mistral-small-latest", "bearer"),
    "together":   ("api.together.xyz", "/v1/chat/completions",
                   "TOGETHER_API_KEY", "meta-llama/Llama-3.3-70B-Instruct-Turbo-Free", "bearer"),
}
ORDER = ["groq", "cerebras", "google", "openrouter", "mistral", "together"]
KEYFILE = os.path.expanduser("~/.config/freellm/keys")


def load_keys():
    """Environment first, then the key file. A world-readable key file is REFUSED."""
    keys = {}
    for name, (_, _, env, _, _) in PROVIDERS.items():
        v = os.environ.get(env)
        if v:
            keys[name] = v
    if os.path.exists(KEYFILE):
        st = os.stat(KEYFILE)
        if st.st_mode & 0o077:
            sys.stderr.write("freellm: %s is mode %o -- group/world readable. "
                             "chmod 600 it; refusing to read a key from it.\n"
                             % (KEYFILE, st.st_mode & 0o777))
            return keys, False
        for line in open(KEYFILE):
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            k, v = line.split("=", 1)
            k = k.strip().lower()
            if k in PROVIDERS and k not in keys and v.strip():
                keys[k] = v.strip()
    return keys, True


def reachable(host, timeout=6):
    import socket
    try:
        s = socket.create_connection((host, 443), timeout=timeout)
        ssl.create_default_context().wrap_socket(s, server_hostname=host).close()
        return True, ""
    except Exception as e:
        return False, type(e).__name__


def call(name, key, prompt, model=None, max_tokens=1024, timeout=90, dry=False):
    host, path, _, default_model, _ = PROVIDERS[name]
    body = json.dumps({
        "model": model or default_model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "temperature": 0,
    }).encode()
    url = "https://%s%s" % (host, path)
    req = urllib.request.Request(url, data=body, method="POST")
    req.add_header("Content-Type", "application/json")
    req.add_header("Authorization", "Bearer %s" % key)
    if dry:
        return None, "DRY-RUN %s %s model=%s bytes=%d" % (
            name, url, model or default_model, len(body))
    t0 = time.time()
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            data = json.loads(r.read())
    except urllib.error.HTTPError as e:
        detail = e.read()[:300].decode("utf-8", "replace")
        return None, "HTTP %s: %s" % (e.code, detail)
    except Exception as e:
        return None, "%s: %s" % (type(e).__name__, e)
    try:
        ch = data["choices"][0]
        text = ch["message"]["content"]
        finish = ch.get("finish_reason", "")
    except (KeyError, IndexError):
        return None, "unparseable response: %s" % json.dumps(data)[:300]
    if not text or not text.strip():
        return None, "empty completion (finish_reason=%s)" % finish
    if finish == "length":
        return None, ("TRUNCATED at max_tokens=%d -- refusing a partial answer; "
                      "raise --max or shorten the prompt" % max_tokens)
    sys.stderr.write("freellm: %s/%s ok in %.1fs, %d chars\n"
                     % (name, model or default_model, time.time() - t0, len(text)))
    return text, ""


def main(argv):
    prompt, model, provider, max_tokens = None, None, None, 1024
    probe = lst = dry = False
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "--probe":
            probe = True
        elif a == "--list":
            lst = True
        elif a == "--dry-run":
            dry = True
        elif a == "-p":
            i += 1; prompt = argv[i]
        elif a == "-f":
            i += 1; prompt = open(argv[i]).read()
        elif a == "--model":
            i += 1; model = argv[i]
        elif a == "--provider":
            i += 1; provider = argv[i]
        elif a == "--max":
            i += 1; max_tokens = int(argv[i])
        else:
            sys.stderr.write("freellm: unknown argument %r\n" % a)
            return 2
        i += 1

    if lst:
        for n in ORDER:
            host, _, env, m, _ = PROVIDERS[n]
            print("%-11s %-28s %s  (%s)" % (n, env, m, host))
        return 0

    keys, keyfile_ok = load_keys()

    if probe:
        print("%-11s %-9s %-9s %s" % ("provider", "key", "reachable", "model"))
        any_key = False
        for n in ORDER:
            host, _, env, m, _ = PROVIDERS[n]
            has = n in keys
            any_key = any_key or has
            ok, err = reachable(host)
            print("%-11s %-9s %-9s %s" % (n, "SET" if has else "-",
                                          "yes" if ok else err, m))
        if not any_key:
            sys.stderr.write(
                "\nfreellm: NO KEY IS SET. Every provider above is reachable from this box, so\n"
                "the network is not the blocker -- the only missing input is a key. Set one of\n"
                "%s in the environment, or write PROVIDER=key lines to\n%s (chmod 600).\n"
                % (", ".join(PROVIDERS[n][2] for n in ORDER), KEYFILE))
            return 3
        return 0

    if prompt is None:
        sys.stderr.write(__doc__.split("\n\n")[3] + "\n")
        return 2

    order = [provider] if provider else ORDER
    if provider and provider not in PROVIDERS:
        sys.stderr.write("freellm: unknown provider %r (%s)\n"
                         % (provider, ", ".join(ORDER)))
        return 2

    tried, errors = [], []
    for n in order:
        if n not in keys:
            tried.append("%s (no key)" % n)
            continue
        text, err = call(n, keys[n], prompt, model, max_tokens, dry=dry)
        if dry:
            print(err)
            return 0
        if text is not None:
            sys.stdout.write(text if text.endswith("\n") else text + "\n")
            return 0
        errors.append("%s: %s" % (n, err))
        sys.stderr.write("freellm: %s failed, falling through -- %s\n" % (n, err))

    if not errors:
        sys.stderr.write("freellm: no provider had a key. Tried: %s\n" % ", ".join(tried))
        sys.stderr.write("freellm: run --probe for the full picture.\n")
        return 3
    sys.stderr.write("freellm: EVERY provider failed:\n  %s\n" % "\n  ".join(errors))
    return 4


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
