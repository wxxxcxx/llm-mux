pub mod convert;
pub mod encode;

use std::collections::HashMap;

use llm_mux_core::codec::{Codec, CodecError};
use llm_mux_core::ir::{
    IrMessage, IrRequest, IrResponse, IrResponseFormat, IrStreamEvent, IrTool,
};
use llm_mux_core::types::{
    ContentBlock, ContentType, Protocol, Role, TextContent, ToolResultContent,
    ToolUseContent,
};

use crate::models as m;
use crate::responses::convert::*;

pub struct ResponsesCodec;

impl Codec for ResponsesCodec {
    fn protocol(&self) -> Protocol {
        Protocol::OpenAiResponses
    }

    fn decode_request(&self, body: &[u8]) -> Result<IrRequest, CodecError> {
        let req: m::ResponsesRequest = serde_json::from_slice(body)
            .map_err(|e| CodecError::Decode(format!("invalid responses request: {e}")))?;

        let mut ir = IrRequest::new(req.model.clone(), Protocol::OpenAiResponses);
        ir.stream = req.stream;
        ir.temperature = req.temperature;
        ir.top_p = req.top_p;
        ir.max_tokens = req.max_output_tokens;

        if let Some(instructions) = &req.instructions {
            if !instructions.is_empty() {
                ir.system_prompt.push(ContentBlock {
                    content_type: ContentType::Text,
                    text: Some(TextContent {
                        text: instructions.clone(),
                    }),
                    ..Default::default()
                });
            }
        }

        if let Some(prev_id) = &req.previous_response_id {
            ir.provider_extensions.insert(
                "previous_response_id".into(),
                serde_json::Value::String(prev_id.clone()),
            );
        }

        ir.provider_extensions
            .extend(req.extra.clone().into_iter().map(|(k, v)| (k, v.clone())));

        if let Ok(raw) = serde_json::from_slice::<serde_json::Value>(body) {
            if let Some(obj) = raw.as_object() {
                for key in obj.keys() {
                    if !ir.provider_extensions.contains_key(key.as_str()) {
                        ir.raw_extra.insert(key.clone(), obj[key].clone());
                    }
                }
            }
        }

        for item in &req.input {
            match item {
                m::ResponseInputItem::Text { text } => {
                    ir.messages.push(IrMessage {
                        role: Role::User,
                        content: vec![ContentBlock {
                            content_type: ContentType::Text,
                            text: Some(TextContent { text: text.clone() }),
                            ..Default::default()
                        }],
                    });
                }
                m::ResponseInputItem::OutputText { text, .. } => {
                    let content = if text.is_empty() {
                        Vec::new()
                    } else {
                        vec![ContentBlock {
                            content_type: ContentType::Text,
                            text: Some(TextContent { text: text.clone() }),
                            ..Default::default()
                        }]
                    };
                    if !content.is_empty() {
                        ir.messages.push(IrMessage {
                            role: Role::Assistant,
                            content,
                        });
                    }
                }
                m::ResponseInputItem::FunctionCall {
                    call_id,
                    name,
                    arguments,
                } => {
                    ir.messages.push(IrMessage {
                        role: Role::Assistant,
                        content: vec![ContentBlock {
                            content_type: ContentType::ToolUse,
                            tool_use: Some(ToolUseContent {
                                id: call_id.clone(),
                                name: name.clone(),
                                arguments: Some(serde_json::Value::String(arguments.clone())),
                            }),
                            ..Default::default()
                        }],
                    });
                }
                m::ResponseInputItem::FunctionCallOutput { call_id, output } => {
                    ir.messages.push(IrMessage {
                        role: Role::Tool,
                        content: vec![ContentBlock {
                            content_type: ContentType::ToolResult,
                            tool_result: Some(ToolResultContent {
                                tool_use_id: call_id.clone(),
                                content: vec![ContentBlock {
                                    content_type: ContentType::Text,
                                    text: Some(TextContent {
                                        text: output.clone(),
                                    }),
                                    ..Default::default()
                                }],
                                is_error: None,
                                name: None,
                            }),
                            ..Default::default()
                        }],
                    });
                }
                m::ResponseInputItem::ItemReference { .. } => {}
            }
        }

        if let Some(tools) = &req.tools {
            for tool in tools {
                if let (Some(name), Some(desc), Some(params)) = (
                    tool.get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    tool.get("description")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    tool.get("parameters").cloned(),
                ) {
                    ir.tools.push(IrTool {
                        r#type: Some(
                            tool.get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("function")
                                .into(),
                        ),
                        name,
                        description: Some(desc),
                        parameters: Some(params),
                        strict: tool.get("strict").and_then(|v| v.as_bool()),
                        extra_fields: HashMap::new(),
                    });
                }
            }
        }

        if let Some(tc) = &req.tool_choice {
            ir.tool_choice = parse_tool_choice(tc);
        }

        if let Some(text_cfg) = &req.text {
            if let Some(fmt) = &text_cfg.format {
                ir.response_format = Some(IrResponseFormat {
                    format_type: fmt.format_type.clone(),
                    json_schema: fmt.json_schema.clone(),
                });
            }
        }

        Ok(ir)
    }

    fn encode_response(&self, response: &IrResponse) -> Result<Vec<u8>, CodecError> {
        crate::responses::encode::encode_response_impl(response)
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
                Ok(text.to_string())
            }
            llm_mux_core::ir::StreamEventType::Stop => Ok("{}".into()),
            _ => Ok(String::new()),
        }
    }

    fn write_error(&self, status_code: u16, message: &str) -> Vec<u8> {
        let err = m::ResponsesError {
            error: m::ResponsesErrorDetail {
                message: message.into(),
                error_type: "invalid_request_error".into(),
                code: Some(status_code.to_string()),
            },
        };
        serde_json::to_vec(&err).unwrap_or_default()
    }
}
