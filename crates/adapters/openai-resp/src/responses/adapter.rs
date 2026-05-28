use genai::chat::{ChatMessage as GChatMsg, ChatRole as GChatRole, ChatStreamEvent};
use llm_mux_core::adapter::{Adapter, AdapterError};
use llm_mux_core::types::Protocol;

use super::ResponsesCodec;
use crate::models as m;

impl Adapter for ResponsesCodec {
    fn protocol(&self) -> Protocol {
        Protocol::OpenAiResponses
    }

    fn decode_request(&self, body: &[u8]) -> Result<genai::chat::ChatRequest, AdapterError> {
        let req: m::ResponsesRequest = serde_json::from_slice(body)
            .map_err(|e| AdapterError::Decode(format!("invalid: {e}")))?;

        let mut messages: Vec<GChatMsg> = Vec::new();

        if let Some(instructions) = &req.instructions {
            messages.push(GChatMsg::new(GChatRole::System, instructions.clone()));
        }

        for item in &req.input {
            match item {
                m::ResponseInputItem::Text { text } => {
                    messages.push(GChatMsg::new(GChatRole::User, text.clone()));
                }
                _ => {}
            }
        }

        Ok(genai::chat::ChatRequest::new(messages))
    }

    fn encode_response(&self, response: &genai::chat::ChatResponse) -> Result<Vec<u8>, AdapterError> {
        let text = response.first_text().unwrap_or("").to_string();

        let output = if text.is_empty() { vec![] } else {
            vec![m::ResponseOutputItem::Message {
                id: "msg_0".into(),
                role: "assistant".into(),
                content: vec![m::ResponseContentPart::OutputText {
                    text,
                    annotations: vec![],
                }],
                status: Some("completed".into()),
            }]
        };

        let resp = m::ResponsesResponse {
            id: "resp_0".into(),
            object: "response".into(),
            model: String::new(),
            output,
            usage: None,
            status: Some("completed".into()),
        };
        serde_json::to_vec(&resp)
            .map_err(|e| AdapterError::Encode(format!("serialize: {e}")))
    }

    fn encode_stream_event(&self, event: &ChatStreamEvent) -> Result<String, AdapterError> {
        match event {
            ChatStreamEvent::Chunk(c) => Ok(format!(
                "data: {{\"type\":\"response.output_text.delta\",\"delta\":\"{}\"}}\n\n",
                c.content
            )),
            ChatStreamEvent::End(_) => {
                Ok("data: {\"type\":\"response.completed\"}\n\ndata: [DONE]\n\n".into())
            }
            _ => Ok(String::new()),
        }
    }

    fn encode_error(&self, error: &AdapterError) -> Vec<u8> {
        serde_json::to_vec(&m::ResponsesError {
            error: m::ResponsesErrorDetail {
                message: error.to_string(),
                error_type: "invalid_request_error".into(),
                code: Some("500".into()),
            },
        }).unwrap_or_default()
    }
}
