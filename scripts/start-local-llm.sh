#!/bin/bash
# Start Ollama service for local LLM support (P103 model-pair supervisor)
#
# Models available:
#   - qwen2.5-coder:7b (code generation, 7.6B parameters)
#   - llama3.1:8b (general reasoning, 8.0B parameters)
#   - nomic-embed-text (embeddings, 137M parameters)
#   - qwen3-embedding:0.6b (embeddings, 595M parameters)

set -e

OLLAMA_BASE_URL="${OLLAMA_BASE_URL:-http://127.0.0.1:11434}"
OLLAMA_PIDFILE="${OLLAMA_PIDFILE:-/var/run/ollama/ollama.pid}"
MAX_WAIT_SECONDS=30

echo "🚀 Starting Ollama local LLM service..."

# Check if Ollama is already running
if pgrep -f "ollama serve" > /dev/null 2>&1; then
    echo "✅ Ollama is already running"
    # Verify connectivity
    if curl -s "${OLLAMA_BASE_URL}/api/tags" > /dev/null 2>&1; then
        echo "✅ Ollama API is responsive at ${OLLAMA_BASE_URL}"
        exit 0
    else
        echo "⚠️  Ollama process exists but API is not responding at ${OLLAMA_BASE_URL}"
        echo "Try restarting with: killall ollama && $0"
        exit 1
    fi
fi

# Start Ollama service
echo "Starting ollama daemon..."
if command -v ollama &> /dev/null; then
    # Start in background, redirecting output to avoid blocking
    nohup ollama serve > /tmp/ollama.log 2>&1 &
    OLLAMA_PID=$!
    echo "Started Ollama with PID $OLLAMA_PID"
else
    echo "❌ Error: ollama command not found"
    echo "Install Ollama from https://ollama.ai or add it to PATH"
    exit 1
fi

# Wait for Ollama to become responsive
echo "Waiting for Ollama API to be ready (max ${MAX_WAIT_SECONDS}s)..."
WAITED=0
while [ $WAITED -lt $MAX_WAIT_SECONDS ]; do
    if curl -s "${OLLAMA_BASE_URL}/api/tags" > /dev/null 2>&1; then
        echo "✅ Ollama API is responsive at ${OLLAMA_BASE_URL}"

        # Check available models
        echo ""
        echo "📦 Available models:"
        curl -s "${OLLAMA_BASE_URL}/api/tags" | \
            grep -o '"name":"[^"]*"' | \
            cut -d'"' -f4 | \
            sed 's/^/   - /'

        exit 0
    fi

    sleep 1
    WAITED=$((WAITED + 1))
    echo -n "."
done

echo ""
echo "❌ Timeout: Ollama API did not become responsive within ${MAX_WAIT_SECONDS} seconds"
echo "Check logs: tail -f /tmp/ollama.log"
exit 1
