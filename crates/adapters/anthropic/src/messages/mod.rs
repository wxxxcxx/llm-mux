pub mod adapter;
pub mod convert;

use std::collections::HashMap;

use llm_mux_core::codec::{Codec, CodecError};
use llm_mux_core::ir::{
    IrMessage, IrRequest, IrResponse, IrThinkingConfig, IrTool,
};
use llm_mux_core::types::{
    ContentBlock as IrBlock, ContentType, Protocol, Role,
    StopReason, TextContent,
};

use crate::messages::convert::*;
use crate::models as m;

pub struct MessagesCodec;

impl Codec for MessagesCodec {
    fn protocol(&self) -> Protocol {
        Protocol::Anthropic
    }

    fn decode_request(&self, body: &[u8]) -> Result<IrRequest, CodecError> {
        let req: m::MessagesRequest = serde_json::from_slice(body)
            .map_err(|e| CodecError::Decode(format!("invalid messages request: {e}")))?;

        let mut ir = IrRequest::new(req.model.clone(), Protocol::Anthropic);
        ir.stream = req.stream;
        ir.temperature = req.temperature;
        ir.top_p = req.top_p;
        ir.top_k = req.top_k;
        ir.max_tokens = req.max_tokens;
        ir.stop_sequences = req.stop_sequences.unwrap_or_default();

        if let Some(sys) = &req.system {
            match sys {
                m::SystemPrompt::Text(text) => {
                    ir.system_prompt.push(IrBlock {
                        content_type: ContentType::Text,
                        text: Some(TextContent { text: text.clone() }),
                        ..Default::default()
                    });
                }
                m::SystemPrompt::Blocks(blocks) => {
                    for b in blocks {
                        ir.system_prompt.push(IrBlock {
                            content_type: ContentType::Text,
                            text: Some(TextContent {
                                text: b.text.clone(),
                            }),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        for msg in &req.messages {
            let role = match msg.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                _ => continue,
            };
            let blocks = anthropic_content_to_blocks(&msg.content);
            ir.messages.push(IrMessage {
                role,
                content: blocks,
            });
        }

        // Provider extensions from the `extra` flattened fields
        ir.provider_extensions = req.extra.clone().into_iter().collect();

        // Capture unknown top-level fields from the raw JSON body
        if let Ok(raw) = serde_json::from_slice::<serde_json::Value>(body) {
            if let Some(obj) = raw.as_object() {
                for key in obj.keys() {
                    if !ir.provider_extensions.contains_key(key.as_str()) {
                        ir.raw_extra.insert(key.clone(), obj[key].clone());
                    }
                }
            }
        }

        if let Some(tools) = &req.tools {
            for tool in tools {
                ir.tools.push(IrTool {
                    r#type: Some("custom".into()),
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: Some(tool.input_schema.clone()),
                    strict: None,
                    extra_fields: HashMap::new(),
                });
            }
        }

        if let Some(tc) = &req.tool_choice {
            ir.tool_choice = parse_anthropic_tool_choice(tc);
        }

        if let Some(think) = &req.thinking {
            ir.thinking = Some(IrThinkingConfig {
                mode: Some(think.thinking_type.clone()),
                budget_tokens: think.budget_tokens,
                effort: None,
                include_thoughts: None,
                level: None,
            });
        }

        Ok(ir)
    }

    fn encode_response(&self, response: &IrResponse) -> Result<Vec<u8>, CodecError> {
        let mut content_blocks: Vec<m::ContentBlock> = Vec::new();

        for block in &response.content {
            match block.content_type {
                ContentType::Text => {
                    if let Some(text) = &block.text {
                        content_blocks.push(m::ContentBlock {
                            block_type: "text".into(),
                            text: Some(text.text.clone()),
                            ..Default::default()
                        });
                    }
                }
                ContentType::ToolUse => {
                    if let Some(tu) = &block.tool_use {
                        content_blocks.push(m::ContentBlock {
                            block_type: "tool_use".into(),
                            id: Some(tu.id.clone()),
                            name: Some(tu.name.clone()),
                            input: tu.arguments.clone(),
                            ..Default::default()
                        });
                    }
                }
                ContentType::Thinking => {
                    if let Some(tc) = &block.thinking {
                        content_blocks.push(m::ContentBlock {
                            block_type: "thinking".into(),
                            thinking: Some(tc.thinking.clone()),
                            signature: tc.signature.clone(),
                            ..Default::default()
                        });
                    }
                }
                ContentType::RedactedThinking => {
                    if let Some(rt) = &block.redacted_thinking {
                        content_blocks.push(m::ContentBlock {
                            block_type: "redacted_thinking".into(),
                            data: Some(rt.data.clone()),
                            ..Default::default()
                        });
                    }
                }
                _ => {}
            }
        }

        let resp = m::MessagesResponse {
            id: response.id.clone().unwrap_or_default(),
            response_type: "message".into(),
            role: "assistant".into(),
            model: response.model.clone().unwrap_or_default(),
            content: content_blocks,
            stop_reason: response.stop_reason.as_ref().map(|r| {
                match r {
                    StopReason::EndTurn => "end_turn",
                    StopReason::MaxTokens => "max_tokens",
                    StopReason::ToolUse => "tool_use",
                    StopReason::StopSequence => "stop_sequence",
                    StopReason::ContentFilter => "refusal",
                    StopReason::PauseTurn => "end_turn",
                    _ => "end_turn",
                }
                .into()
            }),
            stop_sequence: response.stop_sequence.clone(),
            usage: ir_usage_to_m(&response.usage),
        };

        serde_json::to_vec(&resp)
            .map_err(|e| CodecError::Encode(format!("failed to serialize messages response: {e}")))
    }

    fn write_error(&self, status_code: u16, message: &str) -> Vec<u8> {
        let err = m::AnthropicError {
            error_type: "error".into(),
            error: m::AnthropicErrorDetail {
                detail_type: match status_code {
                    400 => "invalid_request_error",
                    401 => "authentication_error",
                    403 => "permission_error",
                    404 => "not_found_error",
                    429 => "rate_limit_error",
                    _ => "api_error",
                }
                .into(),
                message: message.into(),
            },
        };
        serde_json::to_vec(&err).unwrap_or_default()
    }
}
