use std::collections::HashMap;

use llm_mux_core::codec::{Codec, CodecError};
use llm_mux_core::ir::{
    IrMessage, IrRequest, IrResponse, IrStreamEvent, IrThinkingConfig, IrTool, IrToolChoice,
    IrUsage, StreamEventType,
};
use llm_mux_core::types::{
    ContentBlock as IrBlock, ContentType, ImageContent, Protocol, RedactedThinkingContent, Role,
    StopReason, TextContent, ThinkingContent, ToolResultContent, ToolUseContent,
};

use crate::models as m;

pub struct MessagesCodec;

impl Codec for MessagesCodec {
    fn protocol(&self) -> Protocol {
        Protocol::Anthropic
    }

    fn known_fields(&self) -> &HashMap<String, bool> {
        static FIELDS: std::sync::LazyLock<HashMap<String, bool>> =
            std::sync::LazyLock::new(|| {
                let mut m = HashMap::new();
                m.insert("model".into(), true);
                m.insert("messages".into(), true);
                m.insert("system".into(), true);
                m.insert("max_tokens".into(), true);
                m.insert("stop_sequences".into(), true);
                m.insert("stream".into(), true);
                m.insert("temperature".into(), true);
                m.insert("top_p".into(), true);
                m.insert("top_k".into(), true);
                m.insert("tools".into(), true);
                m.insert("tool_choice".into(), true);
                m.insert("thinking".into(), true);
                m.insert("metadata".into(), true);
                m
            });
        &FIELDS
    }

    fn decode_response(&self, body: &[u8]) -> Result<IrResponse, CodecError> {
        let resp: m::MessagesResponse = serde_json::from_slice(body)?;
        let content: Vec<IrBlock> = resp
            .content
            .into_iter()
            .map(|b| m_block_to_ir(&b))
            .collect();
        let stop_reason = resp.stop_reason.as_deref().map(|s| match s {
            "end_turn" => StopReason::EndTurn,
            "max_tokens" => StopReason::MaxTokens,
            "tool_use" => StopReason::ToolUse,
            "stop_sequence" => StopReason::StopSequence,
            _ => StopReason::Other(s.to_string()),
        });
        Ok(IrResponse {
            id: Some(resp.id),
            model: Some(resp.model),
            content,
            stop_reason,
            stop_sequence: resp.stop_sequence,
            usage: m_usage_to_ir(&resp.usage),
            provider_extensions: HashMap::new(),
        })
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
                    if !self.known_fields().contains_key(key.as_str()) {
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

    fn encode_request(&self, request: &IrRequest) -> Result<Vec<u8>, CodecError> {
        let system = if request.system_prompt.is_empty() {
            None
        } else {
            let texts: Vec<String> = request
                .system_prompt
                .iter()
                .filter_map(|b| b.text.as_ref().map(|t| t.text.clone()))
                .collect();
            if texts.len() == 1 {
                Some(m::SystemPrompt::Text(texts.into_iter().next().unwrap()))
            } else {
                Some(m::SystemPrompt::Blocks(
                    texts
                        .into_iter()
                        .map(|t| m::SystemTextBlock {
                            block_type: "text".into(),
                            text: t,
                            cache_control: None,
                        })
                        .collect(),
                ))
            }
        };

        let messages: Vec<m::Message> = request.messages.iter().filter_map(|msg| {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                _ => return None,
            };
            let blocks: Vec<m::ContentBlock> = msg.content.iter().filter_map(|b| match b.content_type {
                ContentType::Text => Some(m::ContentBlock {
                    block_type: "text".into(),
                    text: b.text.as_ref().map(|t| t.text.clone()),
                    ..Default::default()
                }),
                ContentType::ToolUse => b.tool_use.as_ref().map(|tu| m::ContentBlock {
                    block_type: "tool_use".into(),
                    id: Some(tu.id.clone()),
                    name: Some(tu.name.clone()),
                    input: tu.arguments.clone(),
                    ..Default::default()
                }),
                ContentType::ToolResult => b.tool_result.as_ref().map(|tr| {
                    let content: serde_json::Value = if tr.content.is_empty() {
                        serde_json::Value::String(String::new())
                    } else if tr.content.len() == 1 && tr.content[0].content_type == ContentType::Text {
                        serde_json::Value::String(tr.content[0].text.as_ref().map(|t| t.text.clone()).unwrap_or_default())
                    } else {
                        serde_json::Value::Array(tr.content.iter().map(|cb| serde_json::json!({
                            "type": "text",
                            "text": cb.text.as_ref().map(|t| &t.text).unwrap_or(&"".to_string()),
                        })).collect())
                    };
                    m::ContentBlock {
                        block_type: "tool_result".into(),
                        tool_use_id: Some(tr.tool_use_id.clone()),
                        content: Some(content),
                        is_error: tr.is_error,
                        ..Default::default()
                    }
                }),
                ContentType::Image => b.image.as_ref().map(|img| m::ContentBlock {
                    block_type: "image".into(),
                    source: Some(m::ImageSource {
                        source_type: if img.url.is_some() { "url" } else { "base64" }.into(),
                        media_type: img.media_type.clone(),
                        data: img.data.clone(),
                        url: img.url.clone(),
                    }),
                    ..Default::default()
                }),
                _ => None,
            }).collect();
            if blocks.is_empty() && role != "assistant" {
                None
            } else {
                Some(m::Message {
                    role: role.into(),
                    content: m::Content::Blocks(blocks),
                })
            }
        }).collect();

        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| m::Tool {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        input_schema: t
                            .parameters
                            .clone()
                            .unwrap_or(serde_json::json!({"type": "object"})),
                    })
                    .collect(),
            )
        };

        let tool_choice = request
            .tool_choice
            .as_ref()
            .map(|tc| match tc.choice_type.as_str() {
                "auto" | "any" | "none" => serde_json::json!(tc.choice_type),
                _ => {
                    let mut obj = serde_json::Map::new();
                    obj.insert("type".into(), serde_json::json!("tool"));
                    obj.insert(
                        "name".into(),
                        serde_json::json!(tc.tool_name.clone().unwrap_or_default()),
                    );
                    serde_json::Value::Object(obj)
                }
            });

        let thinking = request.thinking.as_ref().map(|tc| m::ThinkingConfig {
            thinking_type: tc.mode.clone().unwrap_or_else(|| "enabled".into()),
            budget_tokens: tc.budget_tokens,
        });

        let req = m::MessagesRequest {
            model: request.model.clone(),
            messages,
            system,
            max_tokens: request.max_tokens,
            stop_sequences: if request.stop_sequences.is_empty() {
                None
            } else {
                Some(request.stop_sequences.clone())
            },
            stream: request.stream,
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: request.top_k,
            metadata: None,
            tools,
            tool_choice,
            thinking,
            service_tier: None,
            extra: request
                .provider_extensions
                .clone()
                .into_iter()
                .map(|(k, v)| (k, v.clone()))
                .collect(),
        };

        serde_json::to_vec(&req)
            .map_err(|e| CodecError::Encode(format!("failed to serialize messages request: {e}")))
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

    fn decode_stream_event(
        &self,
        _event_type: Option<&str>,
        data: &str,
    ) -> Result<IrStreamEvent, CodecError> {
        let event: m::StreamEvent = serde_json::from_str(data)
            .map_err(|e| CodecError::Decode(format!("invalid stream event: {e}")))?;

        match event.event_type.as_str() {
            "message_start" => {
                let msg = event.message.as_ref();
                Ok(IrStreamEvent {
                    event_type: StreamEventType::Start,
                    response: msg.map(|m| IrResponse {
                        id: Some(m.id.clone()),
                        model: Some(m.model.clone()),
                        content: Vec::new(),
                        stop_reason: None,
                        stop_sequence: None,
                        usage: m_usage_to_ir(&m.usage),
                        provider_extensions: HashMap::new(),
                    }),
                    index: 0,
                    delta: None,
                    stop_reason: None,
                    usage: event.usage.as_ref().map(m_usage_to_ir),
                    error: None,
                })
            }
            "content_block_start" => {
                let cb = event.content_block.as_ref();
                Ok(IrStreamEvent {
                    event_type: StreamEventType::ContentBlockStart,
                    response: None,
                    index: event.index.unwrap_or(0) as i32,
                    delta: Some(IrBlock {
                        content_type: cb
                            .map_or(ContentType::Text, |b| block_type_to_ir(&b.block_type)),
                        tool_use: cb.and_then(tool_use_start),
                        thinking: cb.and_then(thinking_block),
                        redacted_thinking: cb.and_then(redacted_thinking_block),
                        ..Default::default()
                    }),
                    stop_reason: None,
                    usage: None,
                    error: None,
                })
            }
            "content_block_delta" => {
                let delta = event.delta.as_ref();
                let ct = delta
                    .and_then(|d| d.delta_type.as_deref())
                    .map(delta_type_to_ir)
                    .unwrap_or(ContentType::Text);
                Ok(IrStreamEvent {
                    event_type: StreamEventType::Delta,
                    response: None,
                    index: event.index.unwrap_or(0) as i32,
                    delta: Some(IrBlock {
                        content_type: ct,
                        text: delta
                            .and_then(|d| d.text.as_ref())
                            .map(|t| TextContent { text: t.clone() }),
                        tool_use: delta
                            .and_then(|d| d.partial_json.as_ref())
                            .and_then(|json| {
                                let input: serde_json::Value = serde_json::from_str(json).ok()?;
                                Some(ToolUseContent {
                                    id: String::new(),
                                    name: String::new(),
                                    arguments: Some(input),
                                })
                            }),
                        ..Default::default()
                    }),
                    stop_reason: None,
                    usage: None,
                    error: None,
                })
            }
            "content_block_stop" => Ok(IrStreamEvent {
                event_type: StreamEventType::ContentBlockStop,
                response: None,
                index: event.index.unwrap_or(0) as i32,
                delta: None,
                stop_reason: None,
                usage: None,
                error: None,
            }),
            "message_delta" => {
                let stop = event
                    .delta
                    .as_ref()
                    .and_then(|d| d.stop_reason.as_ref())
                    .map(|s| match s.as_str() {
                        "end_turn" => StopReason::EndTurn,
                        "max_tokens" => StopReason::MaxTokens,
                        "tool_use" => StopReason::ToolUse,
                        "stop_sequence" => StopReason::StopSequence,
                        s => StopReason::Other(s.into()),
                    });
                Ok(IrStreamEvent {
                    event_type: StreamEventType::Delta,
                    response: None,
                    index: 0,
                    delta: None,
                    stop_reason: stop,
                    usage: event.usage.as_ref().map(m_usage_to_ir),
                    error: None,
                })
            }
            "message_stop" => Ok(IrStreamEvent {
                event_type: StreamEventType::Stop,
                response: None,
                index: 0,
                delta: None,
                stop_reason: None,
                usage: None,
                error: None,
            }),
            "error" => Ok(IrStreamEvent {
                event_type: StreamEventType::Error,
                response: None,
                index: 0,
                delta: None,
                stop_reason: None,
                usage: None,
                error: Some(llm_mux_core::ir::StreamError {
                    error_type: event
                        .message
                        .as_ref()
                        .and_then(|m| m.content.first())
                        .map(|b| b.text.clone().unwrap_or_default())
                        .or_else(|| {
                            serde_json::from_str::<serde_json::Value>(data)
                                .ok()
                                .and_then(|v| {
                                    v.get("error")
                                        .and_then(|e| e.get("type"))
                                        .and_then(|t| t.as_str())
                                        .map(|s| s.to_string())
                                })
                        }),
                    code: None,
                    message: {
                        let err_msg = serde_json::from_str::<serde_json::Value>(data)
                            .ok()
                            .and_then(|v| {
                                v.get("error")
                                    .and_then(|e| e.get("message"))
                                    .and_then(|m| m.as_str())
                                    .map(|s| s.to_string())
                            });
                        err_msg
                    },
                    param: None,
                }),
            }),
            _ => Ok(IrStreamEvent {
                event_type: StreamEventType::Delta,
                response: None,
                index: 0,
                delta: None,
                stop_reason: None,
                usage: None,
                error: None,
            }),
        }
    }

    fn encode_stream_event(&self, event: &IrStreamEvent) -> Result<String, CodecError> {
        let json = match event.event_type {
            StreamEventType::Start => {
                let msg = event.response.as_ref().map(|r| m::MessagesResponse {
                    id: r.id.clone().unwrap_or_default(),
                    response_type: "message".into(),
                    role: "assistant".into(),
                    model: r.model.clone().unwrap_or_default(),
                    content: Vec::new(),
                    stop_reason: None,
                    stop_sequence: None,
                    usage: m::Usage::default(),
                });
                serde_json::to_string(&m::StreamEvent {
                    event_type: "message_start".into(),
                    message: msg,
                    index: None,
                    content_block: None,
                    delta: None,
                    usage: event.usage.as_ref().map(ir_usage_to_m),
                })
            }
            StreamEventType::ContentBlockStart => {
                let block = event.delta.as_ref().map(|d| m::ContentBlock {
                    block_type: ir_block_type_to_anthropic_str(d),
                    text: d.text.as_ref().map(|t| t.text.clone()),
                    id: d.tool_use.as_ref().map(|tu| tu.id.clone()),
                    name: d.tool_use.as_ref().map(|tu| tu.name.clone()),
                    input: d.tool_use.as_ref().and_then(|tu| tu.arguments.clone()),
                    thinking: d.thinking.as_ref().map(|t| t.thinking.clone()),
                    signature: d.thinking.as_ref().and_then(|t| t.signature.clone()),
                    data: d.redacted_thinking.as_ref().map(|rt| rt.data.clone()),
                    ..Default::default()
                });
                serde_json::to_string(&m::StreamEvent {
                    event_type: "content_block_start".into(),
                    message: None,
                    index: Some(event.index as i64),
                    content_block: block,
                    delta: None,
                    usage: None,
                })
            }
            StreamEventType::Delta => {
                let delta_type = event.delta.as_ref().map(|d| {
                    match d.content_type {
                        ContentType::Text => "text_delta",
                        ContentType::ToolUse => "input_json_delta",
                        ContentType::Thinking => "thinking_delta",
                        _ => "text_delta",
                    }
                    .to_string()
                });
                let stop_str = event.stop_reason.as_ref().map(|r| {
                    match r {
                        StopReason::EndTurn => "end_turn",
                        StopReason::MaxTokens => "max_tokens",
                        StopReason::ToolUse => "tool_use",
                        StopReason::StopSequence => "stop_sequence",
                        StopReason::ContentFilter => "refusal",
                        StopReason::PauseTurn => "end_turn",
                        StopReason::Other(s) => s.as_str(),
                    }
                    .to_string()
                });
                serde_json::to_string(&m::StreamEvent {
                    event_type: "content_block_delta".into(),
                    message: None,
                    index: Some(event.index as i64),
                    content_block: None,
                    delta: Some(m::StreamDelta {
                        delta_type,
                        text: event
                            .delta
                            .as_ref()
                            .and_then(|d| d.text.as_ref().map(|t| t.text.clone())),
                        stop_reason: stop_str,
                        stop_sequence: None,
                        partial_json: event
                            .delta
                            .as_ref()
                            .and_then(|d| d.tool_use.as_ref())
                            .and_then(|tu| tu.arguments.as_ref())
                            .map(|v| v.to_string()),
                    }),
                    usage: event.usage.as_ref().map(ir_usage_to_m),
                })
            }
            StreamEventType::ContentBlockStop => serde_json::to_string(&m::StreamEvent {
                event_type: "content_block_stop".into(),
                message: None,
                index: Some(event.index as i64),
                content_block: None,
                delta: None,
                usage: None,
            }),
            StreamEventType::Stop => return Ok("data: [DONE]\n\n".into()),
            StreamEventType::Error => {
                let msg = event
                    .error
                    .as_ref()
                    .and_then(|e| e.message.clone())
                    .unwrap_or_else(|| "unknown error".into());
                return Ok(format!(
                    "data: {{\"type\":\"error\",\"error\":{{\"type\":\"api_error\",\"message\":\"{msg}\"}}}}\n\ndata: [DONE]\n\n"
                ));
            }
        };
        let s = json.map_err(|e| CodecError::Encode(e.to_string()))?;
        Ok(format!("data: {s}\n\n"))
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

// --- Helpers ---

fn anthropic_content_to_blocks(content: &m::Content) -> Vec<IrBlock> {
    match content {
        m::Content::Text(text) => {
            if text.is_empty() {
                Vec::new()
            } else {
                vec![IrBlock {
                    content_type: ContentType::Text,
                    text: Some(TextContent { text: text.clone() }),
                    ..Default::default()
                }]
            }
        }
        m::Content::Blocks(blocks) => blocks.iter().map(m_block_to_ir).collect(),
    }
}

fn m_block_to_ir(block: &m::ContentBlock) -> IrBlock {
    match block.block_type.as_str() {
        "text" => IrBlock {
            content_type: ContentType::Text,
            text: Some(TextContent {
                text: block.text.clone().unwrap_or_default(),
            }),
            ..Default::default()
        },
        "tool_use" => IrBlock {
            content_type: ContentType::ToolUse,
            tool_use: Some(ToolUseContent {
                id: block.id.clone().unwrap_or_default(),
                name: block.name.clone().unwrap_or_default(),
                arguments: block.input.clone(),
            }),
            ..Default::default()
        },
        "tool_result" => IrBlock {
            content_type: ContentType::ToolResult,
            tool_result: Some(ToolResultContent {
                tool_use_id: block.tool_use_id.clone().unwrap_or_default(),
                name: None,
                content: tool_result_inner(block),
                is_error: block.is_error,
            }),
            ..Default::default()
        },
        "thinking" => IrBlock {
            content_type: ContentType::Thinking,
            thinking: Some(ThinkingContent {
                thinking: block.thinking.clone().unwrap_or_default(),
                signature: block.signature.clone(),
            }),
            ..Default::default()
        },
        "redacted_thinking" => IrBlock {
            content_type: ContentType::RedactedThinking,
            redacted_thinking: Some(RedactedThinkingContent {
                data: block.data.clone().unwrap_or_default(),
            }),
            ..Default::default()
        },
        "image" => IrBlock {
            content_type: ContentType::Image,
            image: block.source.as_ref().map(|src| ImageContent {
                data: src.data.clone(),
                url: src.url.clone(),
                media_type: src.media_type.clone(),
                detail: None,
            }),
            ..Default::default()
        },
        _ => IrBlock::default(),
    }
}

fn tool_result_inner(block: &m::ContentBlock) -> Vec<IrBlock> {
    match block.content.as_ref() {
        Some(serde_json::Value::String(s)) => {
            vec![IrBlock {
                content_type: ContentType::Text,
                text: Some(TextContent { text: s.clone() }),
                ..Default::default()
            }]
        }
        Some(serde_json::Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| serde_json::from_value::<m::ContentBlock>(v.clone()).ok())
            .map(|b| m_block_to_ir(&b))
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_anthropic_tool_choice(value: &serde_json::Value) -> Option<IrToolChoice> {
    match value {
        serde_json::Value::String(s) => {
            let ct = match s.as_str() {
                "auto" | "any" | "none" => s.clone(),
                _ => return None,
            };
            Some(IrToolChoice {
                choice_type: ct,
                tool_name: None,
                allowed_tool_names: Vec::new(),
                allow_parallel_calls: None,
            })
        }
        serde_json::Value::Object(obj) => Some(IrToolChoice {
            choice_type: obj
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .into(),
            tool_name: obj.get("name").and_then(|v| v.as_str()).map(|s| s.into()),
            allowed_tool_names: Vec::new(),
            allow_parallel_calls: obj
                .get("disable_parallel_tool_use")
                .and_then(|v| v.as_bool())
                .map(|b| !b),
        }),
        _ => None,
    }
}

fn m_usage_to_ir(u: &m::Usage) -> IrUsage {
    IrUsage {
        input_tokens: u.input_tokens,
        output_tokens: u.output_tokens,
        total_tokens: Some(
            u.input_tokens.unwrap_or(0)
                + u.output_tokens.unwrap_or(0)
                + u.cache_read_input_tokens.unwrap_or(0)
                + u.cache_creation_input_tokens.unwrap_or(0),
        ),
        cache_read_tokens: u.cache_read_input_tokens,
        cache_creation_tokens: u.cache_creation_input_tokens,
        thinking_tokens: None,
    }
}

fn ir_usage_to_m(u: &IrUsage) -> m::Usage {
    m::Usage {
        input_tokens: u.input_tokens,
        output_tokens: u.output_tokens,
        cache_read_input_tokens: u.cache_read_tokens,
        cache_creation_input_tokens: u.cache_creation_tokens,
    }
}

fn block_type_to_ir(bt: &str) -> ContentType {
    match bt {
        "text" => ContentType::Text,
        "tool_use" => ContentType::ToolUse,
        "thinking" => ContentType::Thinking,
        "redacted_thinking" => ContentType::RedactedThinking,
        "image" => ContentType::Image,
        _ => ContentType::Text,
    }
}

fn delta_type_to_ir(dt: &str) -> ContentType {
    match dt {
        "text_delta" => ContentType::Text,
        "input_json_delta" => ContentType::ToolUse,
        "thinking_delta" => ContentType::Thinking,
        _ => ContentType::Text,
    }
}

fn tool_use_start(block: &m::ContentBlock) -> Option<ToolUseContent> {
    (block.block_type == "tool_use").then(|| ToolUseContent {
        id: block.id.clone().unwrap_or_default(),
        name: block.name.clone().unwrap_or_default(),
        arguments: None,
    })
}

fn thinking_block(block: &m::ContentBlock) -> Option<ThinkingContent> {
    (block.block_type == "thinking").then(|| ThinkingContent {
        thinking: block.thinking.clone().unwrap_or_default(),
        signature: block.signature.clone(),
    })
}

fn redacted_thinking_block(block: &m::ContentBlock) -> Option<RedactedThinkingContent> {
    (block.block_type == "redacted_thinking").then(|| RedactedThinkingContent {
        data: block.data.clone().unwrap_or_default(),
    })
}

fn ir_block_type_to_anthropic_str(block: &IrBlock) -> String {
    match block.content_type {
        ContentType::Text => "text",
        ContentType::ToolUse => "tool_use",
        ContentType::Thinking => "thinking",
        ContentType::RedactedThinking => "redacted_thinking",
        ContentType::Image => "image",
        _ => "text",
    }
    .into()
}
