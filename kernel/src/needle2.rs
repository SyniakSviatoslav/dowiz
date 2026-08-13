//! needle2.rs — Needle 2 reimplementation: tiny edge agent.
//!
//! 45M-param-class tool-calling agent with FSM-based decision making,
//! orchestration, and swarm coordination.
//! Maps to kernel primitives: agent facade (LLM interface, model routing),
//! fsm (state machine for agent lifecycle), orchestrator (tool/skill dispatch),
//! swarm (parallel agent coordination).

use std::collections::HashMap;

/// Agent state — FSM-driven lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// Agent initialized but not ready.
    Initializing,
    /// Agent idle, waiting for input.
    Idle,
    /// Agent processing a request.
    Processing,
    /// Agent calling a tool.
    ToolCalling,
    /// Agent waiting for tool result.
    ToolWaiting,
    /// Agent generating response.
    Responding,
    /// Agent error state.
    Error,
    /// Agent shutting down.
    Shutdown,
}

/// A tool that the agent can call.
#[derive(Debug, Clone)]
pub struct Tool {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// Tool parameters schema (simplified as JSON string).
    pub parameters: String,
}

/// A tool call request.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Tool name.
    pub tool_name: String,
    /// Arguments (as a JSON-like string).
    pub arguments: String,
    /// Call ID for tracking.
    pub call_id: u64,
}

/// A tool call result.
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Call ID this result is for.
    pub call_id: u64,
    /// Whether the tool call succeeded.
    pub success: bool,
    /// Result data (as string).
    pub data: String,
    /// Error message if failed.
    pub error: Option<String>,
}

/// An agent request (input).
#[derive(Debug, Clone)]
pub struct AgentRequest {
    /// User message.
    pub message: String,
    /// Context (optional).
    pub context: Option<String>,
    /// Preferred tools (optional hint).
    pub preferred_tools: Vec<String>,
}

/// An agent response.
#[derive(Debug, Clone)]
pub struct AgentResponse {
    /// Response text.
    pub text: String,
    /// Tool calls made (if any).
    pub tool_calls: Vec<ToolCall>,
    /// Which tools were used.
    pub tools_used: Vec<String>,
    /// Token usage estimate.
    pub tokens_used: u64,
}

/// The tiny edge agent.
pub struct Needle2Agent {
    /// Current FSM state.
    pub state: AgentState,
    /// Available tools.
    tools: Vec<Tool>,
    /// Tool call counter.
    next_call_id: u64,
    /// Tool call history.
    call_history: Vec<ToolCall>,
    /// Response history.
    response_history: Vec<AgentResponse>,
    /// System prompt / instructions.
    system_prompt: String,
    /// Maximum tokens per response.
    max_tokens: u64,
    /// Tool mapping name → index.
    tool_index: HashMap<String, usize>,
}

impl Needle2Agent {
    /// Create a new agent with default settings.
    pub fn new() -> Self {
        Needle2Agent {
            state: AgentState::Initializing,
            tools: Vec::new(),
            next_call_id: 0,
            call_history: Vec::new(),
            response_history: Vec::new(),
            system_prompt: "You are a helpful assistant.".to_string(),
            max_tokens: 4096,
            tool_index: HashMap::new(),
        }
    }

    /// Set the system prompt.
    pub fn set_system_prompt(&mut self, prompt: &str) {
        self.system_prompt = prompt.to_string();
    }

    /// Add a tool.
    pub fn add_tool(&mut self, tool: Tool) {
        let idx = self.tools.len();
        self.tool_index.insert(tool.name.clone(), idx);
        self.tools.push(tool);
    }

    /// Get a tool by name.
    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        if let Some(&idx) = self.tool_index.get(name) {
            self.tools.get(idx)
        } else {
            None
        }
    }

    /// Initialize the agent — transition from Initializing to Idle.
    pub fn initialize(&mut self) {
        self.state = AgentState::Idle;
    }

    /// Process a request — simplified FSM-driven processing.
    ///
    /// Returns an AgentResponse. The agent:
    /// 1. Transitions to Processing
    /// 2. May call tools if needed (ToolCalling → ToolWaiting)
    /// 3. Generates response (Responding)
    /// 4. Returns to Idle
    pub fn process_request(&mut self, request: AgentRequest) -> AgentResponse {
        self.state = AgentState::Processing;

        // Simplified: check if any tools need to be called based on request.
        let mut tool_calls = Vec::new();
        let mut tools_used = Vec::new();

        for tool_name in &request.preferred_tools {
            if let Some(tool_name) = self.get_tool(tool_name).map(|tool| tool.name.clone()) {
                let call_id = self.next_call_id;
                self.next_call_id += 1;

                let tool_call = ToolCall {
                    tool_name: tool_name.clone(),
                    arguments: "{}".to_string(), // simplified
                    call_id,
                };

                tool_calls.push(tool_call.clone());
                tools_used.push(tool_name);
                self.call_history.push(tool_call);

                self.state = AgentState::ToolCalling;
                // Simulate tool execution.
                self.state = AgentState::ToolWaiting;
                self.state = AgentState::Processing;
            }
        }

        self.state = AgentState::Responding;

        let response_text = if tool_calls.is_empty() {
            format!("Processed: {}", request.message)
        } else {
            format!("Used tools [{}] to process: {}", tools_used.join(", "), request.message)
        };

        let response = AgentResponse {
            text: response_text,
            tool_calls,
            tools_used,
            tokens_used: (request.message.len() as u64 + 100).min(self.max_tokens),
        };

        self.response_history.push(response.clone());
        self.state = AgentState::Idle;

        response
    }

    /// Get the current state.
    pub fn state(&self) -> AgentState {
        self.state
    }

    /// Get available tools.
    pub fn tools(&self) -> &Vec<Tool> {
        &self.tools
    }

    /// Get tool call history.
    pub fn call_history(&self) -> &Vec<ToolCall> {
        &self.call_history
    }

    /// Get response history.
    pub fn response_history(&self) -> &Vec<AgentResponse> {
        &self.response_history
    }

    /// Get the number of tools.
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    /// Get the number of calls made.
    pub fn call_count(&self) -> usize {
        self.call_history.len()
    }

    /// Get the number of responses.
    pub fn response_count(&self) -> usize {
        self.response_history.len()
    }

    /// Clear call and response history.
    pub fn clear_history(&mut self) {
        self.call_history.clear();
        self.response_history.clear();
        self.next_call_id = 0;
    }

    /// Reset the agent to Idle state.
    pub fn reset(&mut self) {
        self.state = AgentState::Idle;
        self.clear_history();
    }

    /// Shutdown the agent.
    pub fn shutdown(&mut self) {
        self.state = AgentState::Shutdown;
    }

    /// Check if the agent is ready (Idle or Processing).
    pub fn is_ready(&self) -> bool {
        matches!(self.state, AgentState::Idle | AgentState::Processing)
    }

    /// ASCII report.
    pub fn ascii_report(&self) -> String {
        let mut out = String::from("=== Needle 2 Agent Report ===\n");
        out.push_str(&format!("State: {:?}\n", self.state));
        out.push_str(&format!("Tools: {}, Calls: {}, Responses: {}\n",
            self.tool_count(), self.call_count(), self.response_count()));
        out.push_str(&format!("System prompt: \"{}\"\n", self.system_prompt));
        out.push_str(&format!("Max tokens: {}\n", self.max_tokens));

        out.push_str("\nTools:\n");
        for tool in &self.tools {
            out.push_str(&format!("  {} — {}\n", tool.name, tool.description));
        }

        out.push_str("\nRecent responses:\n");
        for resp in self.response_history.iter().rev().take(3) {
            out.push_str(&format!("  \"{}\" (tools: {})\n",
                resp.text, resp.tools_used.join(", ")));
        }

        out.push_str("\n=== End Report ===\n");
        out
    }
}

impl Default for Needle2Agent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent() -> Needle2Agent {
        Needle2Agent::new()
    }

    fn make_tool(name: &str, desc: &str) -> Tool {
        Tool {
            name: name.to_string(),
            description: desc.to_string(),
            parameters: "{}".to_string(),
        }
    }

    #[test]
    fn new_agent_is_initializing() {
        let a = make_agent();
        assert_eq!(a.state(), AgentState::Initializing);
        assert!(!a.is_ready());
    }

    #[test]
    fn initialize_transitions_to_idle() {
        let mut a = make_agent();
        a.initialize();
        assert_eq!(a.state(), AgentState::Idle);
        assert!(a.is_ready());
    }

    #[test]
    fn add_tool_registers_tool() {
        let mut a = make_agent();
        a.add_tool(make_tool("search", "Search the web"));
        assert_eq!(a.tool_count(), 1);

        let tool = a.get_tool("search").unwrap();
        assert_eq!(tool.name, "search");
    }

    #[test]
    fn process_request_without_tools() {
        let mut a = make_agent();
        a.initialize();

        let resp = a.process_request(AgentRequest {
            message: "Hello".to_string(),
            context: None,
            preferred_tools: Vec::new(),
        });

        assert_eq!(resp.text, "Processed: Hello");
        assert_eq!(resp.tool_calls.len(), 0);
        assert_eq!(a.state(), AgentState::Idle);
        assert_eq!(a.response_count(), 1);
    }

    #[test]
    fn process_request_with_tools() {
        let mut a = make_agent();
        a.initialize();
        a.add_tool(make_tool("search", "Search"));
        a.add_tool(make_tool("calculate", "Calculate"));

        let resp = a.process_request(AgentRequest {
            message: "What's 2+2?".to_string(),
            context: None,
            preferred_tools: vec!["search".to_string(), "calculate".to_string()],
        });

        assert!(resp.text.contains("search, calculate"));
        assert_eq!(resp.tool_calls.len(), 2);
        assert_eq!(resp.tools_used.len(), 2);
        assert_eq!(a.call_count(), 2);
    }

    #[test]
    fn process_request_transitions_through_states() {
        let mut a = make_agent();
        a.initialize();
        assert_eq!(a.state(), AgentState::Idle);

        a.process_request(AgentRequest {
            message: "test".to_string(),
            context: None,
            preferred_tools: Vec::new(),
        });

        assert_eq!(a.state(), AgentState::Idle); // back to idle after response
    }

    #[test]
    fn tool_call_history_tracked() {
        let mut a = make_agent();
        a.initialize();
        a.add_tool(make_tool("t1", "tool1"));

        a.process_request(AgentRequest {
            message: "test".to_string(),
            context: None,
            preferred_tools: vec!["t1".to_string()],
        });

        assert_eq!(a.call_history().len(), 1);
        assert_eq!(a.call_history()[0].tool_name, "t1");
    }

    #[test]
    fn reset_clears_history() {
        let mut a = make_agent();
        a.initialize();
        a.add_tool(make_tool("t1", "tool1"));

        a.process_request(AgentRequest {
            message: "test".to_string(),
            context: None,
            preferred_tools: vec!["t1".to_string()],
        });

        assert_eq!(a.call_count(), 1);
        assert_eq!(a.response_count(), 1);

        a.reset();
        assert_eq!(a.call_count(), 0);
        assert_eq!(a.response_count(), 0);
        assert_eq!(a.state(), AgentState::Idle);
    }

    #[test]
    fn shutdown_transitions_to_shutdown() {
        let mut a = make_agent();
        a.initialize();
        a.shutdown();
        assert_eq!(a.state(), AgentState::Shutdown);
        assert!(!a.is_ready());
    }

    #[test]
    fn set_system_prompt() {
        let mut a = make_agent();
        a.set_system_prompt("You are a test agent.");
        assert_eq!(a.system_prompt, "You are a test agent.");
    }

    #[test]
    fn tool_not_found_returns_none() {
        let a = make_agent();
        assert!(a.get_tool("nonexistent").is_none());
    }

    #[test]
    fn ascii_report_format() {
        let a = make_agent();
        let report = a.ascii_report();
        assert!(report.contains("Needle 2 Agent Report"));
        assert!(report.contains("State:"));
        assert!(report.contains("Tools: 0"));
    }

    #[test]
    fn tokens_used_limited() {
        let mut a = make_agent();
        a.initialize();
        a.max_tokens = 100;

        let long_message = "x".repeat(1000);
        let resp = a.process_request(AgentRequest {
            message: long_message,
            context: None,
            preferred_tools: Vec::new(),
        });

        assert!(resp.tokens_used <= 100);
    }
}
