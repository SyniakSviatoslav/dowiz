//! P106 Phase 2 — Rust-native golden tool-calling test suite.
//!
//! BFCLV4-inspired: score the structured tool call, not prose. Every test is `#[ignore]`d
//! (host-noisy, needs live Ollama), run via `cargo test --ignored`. Advisory probe, not
//! baseline-gated CI — per the repo's standing rule for LLM-quality probes.
//!
//! Fixtures: `golden_toolcalls_fixtures.json` (checked in).
//! Scorecard: appended to `golden_toolcalls_scorecard.jsonl` (git-ignored).
//!
//! No LLM-as-judge anywhere — tool selection is mechanically checkable.

use serde::Deserialize;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// A single test fixture case.
#[derive(Debug, Deserialize)]
struct Fixture {
    id: String,
    prompt: String,
    tools_offered: Vec<String>,
    expect: Expectation,
}

#[derive(Debug, Deserialize)]
struct Expectation {
    /// The tool the model should select. `null` = the model must NOT pick any tool.
    tool: Option<String>,
    /// Required argument names that must appear in the tool call args.
    #[serde(default)]
    required_args: Vec<String>,
}

/// A scorecard entry written after each run.
#[derive(Debug, serde::Serialize)]
struct ScorecardEntry {
    model: String,
    timestamp: String,
    total: usize,
    passed: usize,
    failed: usize,
    cases: Vec<CaseResult>,
}

#[derive(Debug, serde::Serialize)]
struct CaseResult {
    id: String,
    passed: bool,
    detail: String,
}

/// Load fixtures from the checked-in JSON file.
fn load_fixtures() -> Vec<Fixture> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = Path::new(&manifest).join("tests/golden_toolcalls_fixtures.json");
    let data = std::fs::read_to_string(&path).expect("failed to read fixture file");
    serde_json::from_str(&data).expect("failed to parse fixture JSON")
}

/// Build the OpenAI-compatible tools JSON array from the offered tool names.
fn build_tools_json(tool_names: &[String]) -> serde_json::Value {
    let tools: Vec<serde_json::Value> = tool_names
        .iter()
        .map(|name| {
            let (description, param_name) = match name.as_str() {
                "read_order_status" => (
                    "Read the lifecycle status of a delivery order by its id.",
                    "order_id",
                ),
                "web_fetch" => (
                    "Fetch a URL and return its readable text content.",
                    "url",
                ),
                _ => ("Unknown tool", "input"),
            };
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": description,
                    "parameters": {
                        "type": "object",
                        "properties": {
                            param_name: {
                                "type": "string",
                                "description": param_name
                            }
                        },
                        "required": [param_name]
                    }
                }
            })
        })
        .collect();
    serde_json::json!(tools)
}

/// Parse the model's response to detect tool calls. Returns `None` if no tool was called,
/// or `Some((tool_name, args))` if a tool call was detected.
fn parse_tool_call(response: &str) -> Option<(String, serde_json::Value)> {
    // Try to find a tool call in the response. Models may emit tool calls in several formats:
    // 1. OpenAI-style `tool_calls` in the response (handled by the adapter, not visible here)
    // 2. The model might emit a JSON block with tool call info
    // 3. The model might just say the tool name in natural language

    // Strategy: look for JSON with "name" and "arguments" fields (OpenAI tool_call shape)
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(response) {
        // Direct tool_call object
        if let (Some(name), Some(args)) = (val.get("name"), val.get("arguments")) {
            return Some((name.as_str().unwrap_or("").to_string(), args.clone()));
        }
        // Array of tool calls
        if let Some(arr) = val.as_array() {
            if let Some(first) = arr.first() {
                if let (Some(name), Some(args)) = (first.get("name"), first.get("arguments")) {
                    return Some((name.as_str().unwrap_or("").to_string(), args.clone()));
                }
            }
        }
    }

    // Look for a JSON code block in the response (common model pattern)
    let response_lower = response.to_lowercase();
    for pattern in &[
        "```json",
        "```tool",
        "```",
    ] {
        if let Some(start) = response_lower.find(pattern) {
            let after = &response[start + pattern.len()..];
            if let Some(end) = after.find("```") {
                let json_str = after[..end].trim();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                    if let (Some(name), Some(args)) = (val.get("name"), val.get("arguments")) {
                        return Some((name.as_str().unwrap_or("").to_string(), args.clone()));
                    }
                }
            }
        }
    }

    // Heuristic: look for tool name mentions in the response text
    for tool in &["read_order_status", "web_fetch"] {
        if response_lower.contains(tool) {
            // Try to extract args from nearby JSON-like content
            return Some((tool.to_string(), serde_json::json!({})));
        }
    }

    None
}

/// Send a chat request to the local Ollama daemon and return the response content.
fn chat_request(prompt: &str, tools: &serde_json::Value) -> Result<String, String> {
    let client = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let body = serde_json::json!({
        "model": "qwen2.5-coder:7b",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant. You may use tools when appropriate. When you use a tool, respond with a JSON object: {\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}. If no tool is needed, respond naturally."},
            {"role": "user", "content": prompt}
        ],
        "tools": tools,
        "temperature": 0,
        "max_tokens": 256
    });

    let resp = client
        .post("http://127.0.0.1:11434/v1/chat/completions")
        .set("Content-Type", "application/json")
        .send_json(&body)
        .map_err(|e| format!("HTTP error: {e}"))?;

    let val: serde_json::Value = resp
        .into_json()
        .map_err(|e| format!("JSON parse error: {e}"))?;

    // Extract the response content
    if let Some(content) = val
        .pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
    {
        return Ok(content.to_string());
    }

    // Some models return tool_calls in the message instead of content
    if let Some(tool_calls) = val.pointer("/choices/0/message/tool_calls") {
        return Ok(tool_calls.to_string());
    }

    Err(format!("unexpected response shape: {val}"))
}

/// Append a scorecard entry to the `.jsonl` file.
fn append_scorecard(entry: &ScorecardEntry) {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = Path::new(&manifest).join("tests/golden_toolcalls_scorecard.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("failed to open scorecard");
    let line = serde_json::to_string(entry).expect("scorecard serialize");
    writeln!(file, "{line}").expect("scorecard write");
}

#[test]
#[ignore = "needs live Ollama daemon — run via: cargo test --ignored golden_toolcalls"]
fn golden_toolcalls() {
    let fixtures = load_fixtures();
    let tools = build_tools_json(
        &fixtures[0].tools_offered, // all fixtures use the same tool set
    );

    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    for fixture in &fixtures {
        let result = run_case(fixture, &tools);
        if result.passed {
            passed += 1;
        } else {
            failed += 1;
        }
        println!(
            "[{}] {} — {}",
            if result.passed { "PASS" } else { "FAIL" },
            fixture.id,
            result.detail
        );
        results.push(result);
    }

    let scorecard = ScorecardEntry {
        model: "qwen2.5-coder:7b".to_string(),
        timestamp: chrono_free_timestamp(),
        total: fixtures.len(),
        passed,
        failed,
        cases: results,
    };
    append_scorecard(&scorecard);

    println!(
        "\n=== Golden Tool-Calling: {passed}/{total} passed, {failed} failed ===",
        total = fixtures.len()
    );
    assert_eq!(failed, 0, "{failed} fixture(s) failed");
}

fn run_case(fixture: &Fixture, tools: &serde_json::Value) -> CaseResult {
    match chat_request(&fixture.prompt, tools) {
        Ok(response) => {
            let tool_call = parse_tool_call(&response);
            match (&fixture.expect.tool, tool_call) {
                // Expect no tool, model didn't pick one → PASS
                (None, None) => CaseResult {
                    id: fixture.id.clone(),
                    passed: true,
                    detail: "correctly did not pick a tool".into(),
                },
                // Expect no tool, but model picked one → FAIL
                (None, Some((name, _))) => CaseResult {
                    id: fixture.id.clone(),
                    passed: false,
                    detail: format!("should not have picked a tool, but picked '{name}'"),
                },
                // Expect a tool, model didn't pick one → FAIL
                (Some(expected), None) => CaseResult {
                    id: fixture.id.clone(),
                    passed: false,
                    detail: format!("should have picked '{expected}', but picked none"),
                },
                // Expect a tool, model picked one → check correctness
                (Some(expected), Some((actual, args))) => {
                    if &actual != expected {
                        return CaseResult {
                            id: fixture.id.clone(),
                            passed: false,
                            detail: format!("expected '{expected}', got '{actual}'"),
                        };
                    }
                    // Check required args
                    for req_arg in &fixture.expect.required_args {
                        if !args.as_object().map_or(false, |o| o.contains_key(req_arg)) {
                            return CaseResult {
                                id: fixture.id.clone(),
                                passed: false,
                                detail: format!(
                                    "tool '{actual}' missing required arg '{req_arg}' in args: {args}"
                                ),
                            };
                        }
                    }
                    CaseResult {
                        id: fixture.id.clone(),
                        passed: true,
                        detail: format!("correctly picked '{actual}' with required args"),
                    }
                }
            }
        }
        Err(e) => CaseResult {
            id: fixture.id.clone(),
            passed: false,
            detail: format!("request failed: {e}"),
        },
    }
}

fn chrono_free_timestamp() -> String {
    // Simple timestamp without adding chrono as a dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}
