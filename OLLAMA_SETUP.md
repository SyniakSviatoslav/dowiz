# Ollama Local LLM Setup — Complete ✅

> P103 model-pair supervisor: dual-witness (qwen2.5-coder:7b + llama3.1:8b)

## 📍 Status

### Service
- **Status:** ✅ Running
- **Endpoint:** `http://127.0.0.1:11434`
- **API:** OpenAI-compatible (`http://127.0.0.1:11434/v1`)
- **PID:** `ps aux | grep "ollama serve"` to check

### Models Downloaded
```
✅ llama3.1:8b       (8.0B params, 128K context) — Main model
✅ qwen2.5-coder:7b  (7.6B params, 32K context) — Fallback
```

---

## 🚀 Usage

### Test Direct (via curl)
```bash
curl -s http://127.0.0.1:11434/api/chat -d '{
  "model": "llama3.1:8b",
  "messages": [{"role": "user", "content": "Hello!"}],
  "stream": false
}' | jq '.message.content'
```

### Use with Hermes (when fallback activates)
```bash
# Hermes will use Ollama as fallback if primary provider unavailable
hermes -z "Your query here"
```

### Use with Python / Node
```python
# Python example (OpenAI-compatible)
from openai import OpenAI
client = OpenAI(
    base_url="http://127.0.0.1:11434/v1",
    api_key="ollama"
)
response = client.chat.completions.create(
    model="llama3.1:8b",
    messages=[{"role": "user", "content": "Hi"}]
)
print(response.choices[0].message.content)
```

```javascript
// Node.js example
const fetch = require('node-fetch');

const response = await fetch('http://127.0.0.1:11434/api/chat', {
  method: 'POST',
  body: JSON.stringify({
    model: 'llama3.1:8b',
    messages: [{ role: 'user', content: 'Hello' }],
    stream: false
  })
});
const data = await response.json();
console.log(data.message.content);
```

### Rust / kernel Integration
See `kernel/src/ports/llm.rs` for the LLM adapter system.

---

## 🔧 Configuration

### Hermes Config
**File:** `/root/.hermes/config.yaml`

Current fallback chain:
```yaml
model:
  default: sakana/fugu-ultra          # Primary (Nous)
  provider: nous

fallback_providers:
  - provider: custom                   # ← Ollama falls back here
    model: llama3.1:8b
  - provider: custom
    model: qwen2.5-coder:7b
```

### Environment Variables
**File:** `/root/.hermes/.env`

```bash
CUSTOM_BASE_URL=http://127.0.0.1:11434/v1
CUSTOM_API_KEY=ollama
HERMES_INIT_LLM=true        # Auto-start Ollama
```

---

## 🛠️ Troubleshooting

### Ollama Not Responding
```bash
# Restart Ollama
killall ollama
sleep 2
nohup ollama serve > /tmp/ollama.log 2>&1 &
sleep 3

# Test connectivity
curl http://127.0.0.1:11434/api/tags
```

### Model Not Found
```bash
# List available models
ollama list

# Pull a model
ollama pull llama3.1:8b
ollama pull qwen2.5-coder:7b
```

### Check Logs
```bash
# View Ollama logs
tail -100 /tmp/ollama.log

# Or monitor live
tail -f /tmp/ollama.log
```

---

## 📊 Model Specs

| Model | Params | Context | Quant | Use Case |
|-------|--------|---------|-------|----------|
| llama3.1:8b | 8.0B | 128K | Q4_K_M | General reasoning, P103 primary |
| qwen2.5-coder:7b | 7.6B | 32K | Q4_K_M | Code generation, P103 fallback |

---

## 🔌 Hermes Integration

### Skills Available
```bash
# Start/check Ollama
hermes start-ollama

# List models
hermes llm-models

# Test with query
hermes test-llm llama3.1:8b "What is 2+2?"
```

### Auto-Startup
- ✅ Hermes auto-starts Ollama on launch (via `/root/.hermes/hooks/startup-local-llm.sh`)
- ✅ Shell init includes Ollama startup
- ✅ Systemd service ready (see `/root/dowiz/scripts/start-local-llm.sh`)

---

## 📋 Notes

- **Local-first:** All models run on-device; no cloud API calls unless Ollama unavailable
- **OpenAI-compatible:** Use any OpenAI client library with `base_url="http://127.0.0.1:11434/v1"`
- **No auth:** `api_key` can be anything (e.g., "ollama", "local", etc.)
- **Default port:** 11434 (can override with `OLLAMA_HOST`)
- **VRAM:** Needs ~8-16GB for loaded model; auto-unloads idle models

---

## 🎯 Next Steps

1. ✅ Verify Ollama is running: `curl http://127.0.0.1:11434/api/tags`
2. ✅ Test a query: `curl -s http://127.0.0.1:11434/api/chat -d '...'`
3. ✅ Use with Rust kernel via LLM adapter
4. ✅ Hermes will auto-fallback to Ollama when primary unavailable

---

**Last Updated:** 2026-07-21  
**Integration:** dowiz kernel ↔ Local Ollama ↔ P103 Supervisor
