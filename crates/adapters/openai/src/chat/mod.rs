pub mod convert;
pub mod encode;

use std::collections::HashMap;

use llm_mux_core::codec::{Codec, CodecError};
use llm_mux_core::ir::{
    IrMessage, IrRequest, IrResponse, IrStreamEvent, IrTool,
};
use llm_mux_core::types::{
    ContentBlock, ContentType, Protocol, Role,
    ToolResultContent, ToolUseContent,
};

use crate::chat::convert::*;
use crate::models::*;

/// Codec for OpenAI Chat Completions protocol.
pub struct ChatCompletionsCodec;

impl Codec for ChatCompletionsCodec {
    fn protocol(&self) -> Protocol {
        Protocol::OpenAiChat
    }

    fn decode_request(&self, body: &[u8]) -> Result<IrRequest, CodecError> {
        let req: ChatCompletionRequest = serde_json::from_slice(body)
            .map_err(|e| CodecError::Decode(format!("invalid chat request: {e}")))?;

        let mut ir = IrRequest::new(req.model.clone(), Protocol::OpenAiChat);
        ir.stream = req.stream;
        ir.temperature = req.temperature;
        ir.max_tokens = req.max_tokens.or(req.max_completion_tokens);
        ir.top_p = req.top_p;
        ir.stop_sequences = req.stop;

        if let Some(rf) = &req.response_format {
            ir.response_format = Some(llm_mux_core::ir::IrResponseFormat {
                format_type: rf.format_type.clone(),
                json_schema: rf.json_schema.clone(),
            });
        }

        // Provider extensions from the `extra` flattened fields
        ir.provider_extensions = req.extra.clone().into_iter().collect();

        // Provider extensions via genai extra_body
        ir.provider_extensions = req.extra.clone().into_iter().collect();
        // Forward unknown fields
        if let Ok(raw) = serde_json::from_slice::<serde_json::Value>(body) {
            if let Some(obj) = raw.as_object() {
                for key in obj.keys() {
                    if !ir.provider_extensions.contains_key(key.as_str()) {
                        ir.raw_extra.insert(key.clone(), obj[key].clone());
                    }
                }
            }
        }

        // Messages: system → system_prompt, others → IrMessage
        for msg in &req.messages {
            match msg.role.as_str() {
                "system" | "developer" => {
                    if let Some(content) = &msg.content {
                        let blocks = chat_content_to_blocks(content);
                        ir.system_prompt.extend(blocks);
                    }
                }
                "assistant" => {
                    let mut blocks = Vec::new();
                    if let Some(content) = &msg.content {
                        blocks.extend(chat_content_to_blocks(content));
                    }
                    for tc in &msg.tool_calls {
                        blocks.push(ContentBlock {
                            content_type: ContentType::ToolUse,
                            tool_use: Some(ToolUseContent {
                                id: tc.id.clone(),
                                name: tc.function.name.clone(),
                                arguments: Some(serde_json::Value::String(
                                    tc.function.arguments.clone(),
                                )),
                            }),
                            ..Default::default()
                        });
                    }
                    if !blocks.is_empty() || !msg.tool_calls.is_empty() {
                        ir.messages.push(IrMessage {
                            role: Role::Assistant,
                            content: blocks,
                        });
                    }
                }
                "tool" => {
                    let content_blocks = msg
                        .content
                        .as_ref()
                        .map(chat_content_to_blocks)
                        .unwrap_or_default();
                    ir.messages.push(IrMessage {
                        role: Role::Tool,
                        content: vec![ContentBlock {
                            content_type: ContentType::ToolResult,
                            tool_result: Some(ToolResultContent {
                                tool_use_id: msg.tool_call_id.clone().unwrap_or_default(),
                                content: content_blocks,
                                is_error: None,
                                name: msg.name.clone(),
                            }),
                            ..Default::default()
                        }],
                    });
                }
                "user" => {
                    let blocks = msg
                        .content
                        .as_ref()
                        .map(chat_content_to_blocks)
                        .unwrap_or_default();
                    ir.messages.push(IrMessage {
                        role: Role::User,
                        content: blocks,
                    });
                }
                _ => {}
            }
        }

        // Tools
        for tool in &req.tools {
            ir.tools.push(IrTool {
                r#type: Some(tool.tool_type.clone()),
                name: tool.function.name.clone(),
                description: tool.function.description.clone(),
                parameters: tool.function.parameters.clone(),
                strict: tool.function.strict,
                extra_fields: HashMap::new(),
            });
        }

        // Tool choice
        if let Some(tc) = &req.tool_choice {
            ir.tool_choice = parse_tool_choice(tc);
        }

        Ok(ir)
    }

    fn encode_response(&self, response: &IrResponse) -> Result<Vec<u8>, CodecError> {
        crate::chat::encode::encode_response_impl(response)
    }

    fn encode_stream_event(&self, event: &IrStreamEvent) -> Result<String, CodecError> {
        match event.event_type {
            llm_mux_core::ir::StreamEventType::Delta => {
                let text = event
                    .delta
                    .as_ref()
                    .filter(|b| b.content_type == ContentType::Text)
                    .and_then(|b| b.text.as_ref())
                    .map(|t| t.text.as_str())
                    .unwrap_or("");
                Ok(serde_json::json!({
                    "choices": [{"delta": {"content": text}, "index": 0}],
                    "object": "chat.completion.chunk"
                })
                .to_string())
            }
            llm_mux_core::ir::StreamEventType::Stop => Ok("[DONE]".into()),
            _ => Ok(String::new()),
        }
    }

    fn write_error(&self, status_code: u16, message: &str) -> Vec<u8> {
        let err = ChatError {
            error: ChatErrorDetail {
                message: message.into(),
                error_type: "invalid_request_error".into(),
                code: Some(status_code.to_string()),
                param: None,
            },
        };
        serde_json::to_vec(&err).unwrap_or_default()
    }
}


