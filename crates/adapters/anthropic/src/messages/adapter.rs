use genai::chat::{ChatMessage as GChatMsg, ChatRole as GChatRole, ChatStreamEvent, MessageContent};
use llm_mux_core::adapter::{Adapter, AdapterError};
use llm_mux_core::types::Protocol;

use super::MessagesCodec;
use crate::models as m;

impl Adapter for MessagesCodec {
    fn protocol(&self) -> Protocol {
        Protocol::Anthropic
    }

    fn decode_request(&self, body: &[u8]) -> Result<genai::chat::ChatRequest, AdapterError> {
        let req: m::MessagesRequest = serde_json::from_slice(body)
            .map_err(|e| AdapterError::Decode(format!("invalid: {e}")))?;

        let mut messages: Vec<GChatMsg> = Vec::new();

        if let Some(sys) = &req.system {
            let sys_text = match sys {
                m::SystemPrompt::Text(t) => t.clone(),
                m::SystemPrompt::Blocks(blocks) => {
                    blocks.iter().filter_map(|b| Some(b.text.as_str())).collect::<Vec<_>>().join("\n")
                }
            };
            if !sys_text.is_empty() {
                messages.push(GChatMsg::new(GChatRole::System, sys_text));
            }
        }

        for msg in &req.messages {
            let role = match msg.role.as_str() {
                "user" => GChatRole::User,
                "assistant" => GChatRole::Assistant,
                _ => GChatRole::User,
            };
            let content = match &msg.content {
                m::Content::Text(text) => MessageContent::from_text(text.clone()),
                m::Content::Blocks(blocks) => {
                    let parts: Vec<genai::chat::ContentPart> = blocks.iter().filter_map(|b| {
                        match b.block_type.as_str() {
                            "text" => b.text.as_ref().map(|t| genai::chat::ContentPart::Text(t.clone())),
                            "image" => b.source.as_ref().and_then(|s| {
                                s.data.as_ref().map(|d| {
                                    genai::chat::ContentPart::Binary(genai::chat::Binary::from_base64(
                                        s.media_type.as_deref().unwrap_or("image/jpeg"),
                                        d.clone(),
                                        None,
                                    ))
                                })
                            }),
                            _ => None,
                        }
                    }).collect();
                    MessageContent::from_parts(parts)
                }
            };
            messages.push(GChatMsg::new(role, content));
        }

        Ok(genai::chat::ChatRequest::new(messages))
    }

    fn encode_response(&self, response: &genai::chat::ChatResponse) -> Result<Vec<u8>, AdapterError> {
        let text = response.first_text().unwrap_or("").to_string();
        let stop = response.stop_reason.as_ref().map(|s| match s {
            genai::chat::StopReason::Completed(_) => "end_turn",
            genai::chat::StopReason::MaxTokens(_) => "max_tokens",
            genai::chat::StopReason::ToolCall(_) => "tool_use",
            _ => "end_turn",
        }).unwrap_or("end_turn");

        let resp = m::MessagesResponse {
            id: "msg_0".into(),
            response_type: "message".into(),
            role: "assistant".into(),
            model: String::new(),
            content: vec![m::ContentBlock {
                block_type: "text".into(),
                text: Some(text),
                ..Default::default()
            }],
            stop_reason: Some(stop.to_string()),
            stop_sequence: None,
            usage: m::Usage {
                input_tokens: response.usage.prompt_tokens.map(|n| n as i64),
                output_tokens: response.usage.completion_tokens.map(|n| n as i64),
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
            },
        };
        serde_json::to_vec(&resp)
            .map_err(|e| AdapterError::Encode(format!("serialize: {e}")))
    }

    fn encode_stream_event(&self, event: &ChatStreamEvent) -> Result<String, AdapterError> {
        match event {
            ChatStreamEvent::Chunk(c) => Ok(serde_json::json!({
                "type": "content_block_delta",
                "delta": {"type": "text_delta", "text": &c.content}
            }).to_string()),
            ChatStreamEvent::End(_) => Ok(r#"{"type":"message_stop"}"#.into()),
            _ => Ok(String::new()),
        }
    }

    fn encode_error(&self, error: &AdapterError) -> Vec<u8> {
        serde_json::to_vec(&m::AnthropicError {
            error_type: "error".into(),
            error: m::AnthropicErrorDetail {
                detail_type: "invalid_request_error".into(),
                message: error.to_string(),
            },
        }).unwrap_or_default()
    }
}
