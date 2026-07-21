---
description: Start local Ollama LLM service for P103 model-pair supervisor
argument-hint: (none)
allowed-tools: Bash
---

Start the local Ollama LLM service that powers the P103 model-pair supervisor:

```bash
bash scripts/start-local-llm.sh
```

This script:
- ✅ Checks if Ollama is already running
- ✅ Starts Ollama daemon if needed
- ✅ Waits for API to be ready (max 30s)
- ✅ Lists available models

**Available models:**
- `qwen2.5-coder:7b` — Code generation (7.6B params)
- `llama3.1:8b` — General reasoning (8.0B params)
- `nomic-embed-text` — Embeddings (137M params)
- `qwen3-embedding:0.6b` — Embeddings (595M params)

**Default endpoint:** `http://127.0.0.1:11434`

**For development:**
- Test LLM: `! bash scripts/start-local-llm.sh`
- Run llm-adapters tests: `cd llm-adapters && cargo test --lib`
- Test a chat request:
  ```bash
  curl -s http://localhost:11434/api/chat -d '{
    "model": "qwen2.5-coder:7b",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": false
  }'
  ```
