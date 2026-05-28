use genai::chat::{
    ChatMessage as GenaiChatMessage, ChatRole as GenaiRole,
    ChatStreamEvent as GenaiStreamEvent, MessageContent,
};
use llm_mux_core::adapter::{Adapter, AdapterError};
use llm_mux_core::types::Protocol;

use super::ChatCompletionsCodec;
use crate::models::*;

impl Adapter for ChatCompletionsCodec {
    fn protocol(&self) -> Protocol {
        Protocol::OpenAiChat
    }

    fn decode_request(&self, body: &[u8]) -> Result<genai::chat::ChatRequest, AdapterError> {
        let req: ChatCompletionRequest = serde_json::from_slice(body)
            .map_err(|e| AdapterError::Decode(format!("invalid chat request: {e}")))?;

        let mut messages: Vec<GenaiChatMessage> = Vec::new();
        for msg in &req.messages {
            let role = match msg.role.as_str() {
                "system" | "developer" => GenaiRole::System,
                "assistant" => GenaiRole::Assistant,
                "tool" => GenaiRole::Tool,
                _ => GenaiRole::User,
            };
            let content = adapter_extract_content(msg);
            messages.push(GenaiChatMessage::new(role, content));
        }

        Ok(genai::chat::ChatRequest::new(messages))
    }

    fn encode_response(&self, response: &genai::chat::ChatResponse) -> Result<Vec<u8>, AdapterError> {
        let text = response.first_text().unwrap_or("").to_string();

        let chat_resp = ChatCompletionResponse {
            id: String::new(),
            object: "chat.completion".into(),
            created: 0,
            model: String::new(),
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    reasoning_content: None,
                    role: "assistant".into(),
                    content: Some(ChatContent::Text(text)),
                    name: None,
                    tool_calls: Vec::new(),
                    tool_call_id: None,
                },
                finish_reason: Some("stop".into()),
            }],
            usage: None,
        };
        serde_json::to_vec(&chat_resp)
            .map_err(|e| AdapterError::Encode(format!("serialize: {e}")))
    }

    fn encode_stream_event(&self, event: &GenaiStreamEvent) -> Result<String, AdapterError> {
        match event {
            GenaiStreamEvent::Chunk(chunk) => Ok(serde_json::json!({
                "choices": [{"delta": {"content": &chunk.content}, "index": 0}],
                "object": "chat.completion.chunk"
            }).to_string()),
            GenaiStreamEvent::End(_) => Ok("[DONE]".into()),
            _ => Ok(String::new()),
        }
    }

    fn encode_error(&self, error: &AdapterError) -> Vec<u8> {
        let err = ChatError {
            error: ChatErrorDetail {
                message: error.to_string(),
                error_type: "invalid_request_error".into(),
                code: Some("500".into()),
                param: None,
            },
        };
        serde_json::to_vec(&err).unwrap_or_default()
    }
}

fn adapter_extract_content(msg: &ChatMessage) -> MessageContent {
    let mut parts: Vec<genai::chat::ContentPart> = Vec::new();
    if let Some(content) = &msg.content {
        match content {
            ChatContent::Text(text) => {
                if !text.is_empty() {
                    parts.push(genai::chat::ContentPart::Text(text.clone()));
                }
            }
            ChatContent::Parts(content_parts) => {
                for p in content_parts {
                    match p.part_type.as_str() {
                        "text" => parts.push(genai::chat::ContentPart::Text(
                            p.text.clone().unwrap_or_default(),
                        )),
                        "image_url" => {
                            if let Some(img) = &p.image_url {
                                parts.push(genai::chat::ContentPart::Binary(genai::chat::Binary::from_url(
                                    "image/jpeg",
                                    img.url.clone(),
                                    None,
                                )));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    for tc in &msg.tool_calls {
        let args: serde_json::Value =
            serde_json::from_str(&tc.function.arguments).unwrap_or_default();
        parts.push(genai::chat::ContentPart::Custom(genai::chat::CustomPart {
            model_iden: None,
            data: serde_json::json!({
                "type": "tool_call",
                "call_id": tc.id,
                "fn_name": tc.function.name,
                "fn_arguments": args,
            }),
        }));
    }
    MessageContent::from_parts(parts)
}
