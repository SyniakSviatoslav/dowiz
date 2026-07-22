#![cfg(feature = "bebop")]

use dowiz_kernel::ports::llm::{
    Caps, ChatRequest, ChatResponse, EmbedRequest, EmbedResponse, LlmBackend, LlmError,
    RerankRequest, RerankResponse, TaskClass,
};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const BEBOP_MODEL_ID: &str = "bebop-agent:7b";

pub struct BebopAdapter {
    engine: std::sync::Mutex<bebop_model::native_engine::BebopEngine>,
    model_id: String,
    next_utterance: std::sync::atomic::AtomicU64,
}

impl BebopAdapter {
    pub fn new() -> Self {
        BebopAdapter {
            engine: std::sync::Mutex::new(bebop_model::native_engine::BebopEngine::new()),
            model_id: BEBOP_MODEL_ID.to_string(),
            next_utterance: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn route_model(&self, req: &ChatRequest) -> String {
        if !req.model_id.is_empty() {
            return req.model_id.clone();
        }
        match req.task_class {
            TaskClass::Code => "bebop-agent:7b-code".to_string(),
            TaskClass::General => BEBOP_MODEL_ID.to_string(),
            TaskClass::Embedding => "bebop-agent:7b-embed".to_string(),
        }
    }
}

impl LlmBackend for BebopAdapter {
    fn id(&self) -> &str {
        &self.model_id
    }

    fn caps(&self) -> Caps {
        Caps {
            chat: true,
            embed: true,
            rerank: false,
            tool_calling: false,
        }
    }

    fn chat(&self, req: &ChatRequest) -> Result<ChatResponse, LlmError> {
        let model_id = self.route_model(req);
        let mut engine = self.engine.lock().map_err(|e| {
            LlmError::BadRequest(format!("bebop lock: {e}"))
        })?;
        let prompt = req.messages.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        let content = engine.generate(&prompt, req.max_tokens as usize);
        let uid = self.next_utterance.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(ChatResponse {
            content,
            model_id,
            utterance_id: uid,
            tool_calls: Vec::new(),
            usage: dowiz_kernel::ports::llm::Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
        })
    }

    fn embed(&self, req: &EmbedRequest) -> Result<EmbedResponse, LlmError> {
        let mut engine = self.engine.lock().map_err(|e| {
            LlmError::BadRequest(format!("bebop lock: {e}"))
        })?;
        let embedding = engine.embed(&req.input);
        Ok(EmbedResponse {
            embedding,
            model_id: req.model_id.clone(),
        })
    }

    fn rerank(&self, _req: &RerankRequest) -> Result<RerankResponse, LlmError> {
        Err(LlmError::Unsupported)
    }

    fn health(&self) -> Result<(), LlmError> {
        let engine = self.engine.lock().map_err(|e| {
            LlmError::BadRequest(format!("bebop lock: {e}"))
        })?;
        if engine.is_ready() {
            Ok(())
        } else {
            Err(LlmError::Unavailable)
        }
    }
}
