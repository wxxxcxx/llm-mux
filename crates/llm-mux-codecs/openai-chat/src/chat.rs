use std::collections::HashMap;

use llm_mux_core::codec::{Codec, CodecError};
use llm_mux_core::ir::{
    IrMessage, IrRequest, IrResponse, IrStreamEvent, IrTool, IrToolChoice, IrUsage, StreamEventType,
};
use llm_mux_core::types::{
    ContentBlock, ContentType, ImageContent, Protocol, Role, StopReason, TextContent,
    ToolResultContent, ToolUseContent,
};

use crate::models::*;

/// Codec for OpenAI Chat Completions protocol.
pub struct ChatCompletionsCodec;

impl Codec for ChatCompletionsCodec {
    fn protocol(&self) -> Protocol {
        Protocol::OpenAiChat
    }

    fn known_fields(&self) -> &HashMap<String, bool> {
        static FIELDS: std::sync::LazyLock<HashMap<String, bool>> =
            std::sync::LazyLock::new(|| {
                let mut m = HashMap::new();
                m.insert("model".into(), true);
                m.insert("messages".into(), true);
                m.insert("stream".into(), true);
                m.insert("temperature".into(), true);
                m.insert("max_tokens".into(), true);
                m.insert("max_completion_tokens".into(), true);
                m.insert("top_p".into(), true);
                m.insert("stop".into(), true);
                m.insert("tools".into(), true);
                m.insert("tool_choice".into(), true);
                m.insert("response_format".into(), true);
                m.insert("seed".into(), true);
                m.insert("user".into(), true);
                m
            });
        &FIELDS
    }

    fn decode_response(&self, body: &[u8]) -> Result<IrResponse, CodecError> {
        let resp: ChatCompletionResponse = serde_json::from_slice(body)?;
        let mut content = Vec::new();
        if let Some(choice) = resp.choices.first() {
            if let Some(ref msg_content) = choice.message.content {
                content.extend(chat_content_to_blocks(msg_content));
            }
            if !choice.message.tool_calls.is_empty() {
                for tc in &choice.message.tool_calls {
                    let args: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                    content.push(ContentBlock {
                        content_type: ContentType::ToolUse,
                        tool_use: Some(ToolUseContent {
                            id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            arguments: Some(args),
                        }),
                        ..Default::default()
                    });
                }
            }
        }
        let stop_reason = resp
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .map(|fr| match fr.as_str() {
                "stop" => StopReason::EndTurn,
                "length" => StopReason::MaxTokens,
                "tool_calls" => StopReason::ToolUse,
                "content_filter" => StopReason::ContentFilter,
                _ => StopReason::Other(fr.clone()),
            });
        Ok(IrResponse {
            id: Some(resp.id),
            model: Some(resp.model),
            content,
            stop_reason,
            stop_sequence: None,
            usage: IrUsage {
                input_tokens: resp.usage.as_ref().map(|u| u.prompt_tokens),
                output_tokens: resp.usage.as_ref().map(|u| u.completion_tokens),
                total_tokens: resp.usage.as_ref().map(|u| u.total_tokens),
                ..Default::default()
            },
            provider_extensions: HashMap::new(),
        })
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

    fn encode_request(&self, request: &IrRequest) -> Result<Vec<u8>, CodecError> {
        let mut messages: Vec<ChatMessage> = Vec::new();

        for block in &request.system_prompt {
            if let Some(text) = &block.text {
                messages.push(ChatMessage {
                    role: "system".into(),
                    content: Some(ChatContent::Text(text.text.clone())),
                    name: None,
                    tool_calls: Vec::new(),
                    tool_call_id: None,
                });
            }
        }

        for msg in &request.messages {
            match msg.role {
                Role::User => {
                    let parts = blocks_to_chat_content(&msg.content);
                    messages.push(ChatMessage {
                        role: "user".into(),
                        content: Some(parts),
                        name: None,
                        tool_calls: Vec::new(),
                        tool_call_id: None,
                    });
                }
                Role::Assistant => {
                    let mut text_parts = Vec::new();
                    let mut tool_calls = Vec::new();
                    for block in &msg.content {
                        match block.content_type {
                            ContentType::Text => {
                                if let Some(t) = &block.text {
                                    text_parts.push(t.text.clone());
                                }
                            }
                            ContentType::ToolUse => {
                                if let Some(tu) = &block.tool_use {
                                    let args = tu
                                        .arguments
                                        .as_ref()
                                        .map(|v| match v {
                                            serde_json::Value::String(s) => s.clone(),
                                            other => other.to_string(),
                                        })
                                        .unwrap_or_default();
                                    tool_calls.push(ToolCall {
                                        id: tu.id.clone(),
                                        call_type: "function".into(),
                                        function: FunctionCall {
                                            name: tu.name.clone(),
                                            arguments: args,
                                        },
                                        index: None,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                    let content = if text_parts.is_empty() && !tool_calls.is_empty() {
                        None
                    } else {
                        Some(ChatContent::Text(text_parts.join("")))
                    };
                    messages.push(ChatMessage {
                        role: "assistant".into(),
                        content,
                        name: None,
                        tool_calls,
                        tool_call_id: None,
                    });
                }
                Role::Tool => {
                    let content = if let Some(block) = msg.content.first() {
                        if let Some(tr) = &block.tool_result {
                            let text = tr
                                .content
                                .iter()
                                .filter_map(|b| b.text.as_ref().map(|t| t.text.as_str()))
                                .collect::<Vec<_>>()
                                .join("");
                            ChatContent::Text(text)
                        } else {
                            ChatContent::Text(String::new())
                        }
                    } else {
                        ChatContent::Text(String::new())
                    };
                    messages.push(ChatMessage {
                        role: "tool".into(),
                        content: Some(content),
                        name: None,
                        tool_calls: Vec::new(),
                        tool_call_id: msg
                            .content
                            .first()
                            .and_then(|b| b.tool_result.as_ref().map(|tr| tr.tool_use_id.clone())),
                    });
                }
                _ => {}
            }
        }

        let mut tools = Vec::new();
        for tool in &request.tools {
            tools.push(ChatTool {
                tool_type: tool.r#type.clone().unwrap_or_else(|| "function".into()),
                function: FunctionDef {
                    name: tool.name.clone(),
                    description: tool.description.clone(),
                    parameters: tool.parameters.clone(),
                    strict: tool.strict,
                },
            });
        }

        let req = ChatCompletionRequest {
            model: request.model.clone(),
            messages,
            stream: request.stream,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            max_completion_tokens: None,
            top_p: request.top_p,
            stop: request.stop_sequences.clone(),
            tools,
            tool_choice: request.tool_choice.as_ref().map(|tc| match tc.choice_type.as_str() {
                "auto" | "none" | "required" => serde_json::json!(tc.choice_type),
                _ => serde_json::json!({"type": "function", "function": {"name": tc.tool_name.clone().unwrap_or_default()}}),
            }),
            response_format: request.response_format.as_ref().map(|rf| ResponseFormat {
                format_type: rf.format_type.clone(),
                json_schema: rf.json_schema.clone(),
            }),
            seed: None,
            user: None,
            extra: request.provider_extensions.clone().into_iter().map(|(k, v)| (k, v.clone())).collect(),
        };

        serde_json::to_vec(&req)
            .map_err(|e| CodecError::Encode(format!("failed to serialize chat request: {e}")))
    }

    fn encode_response(&self, response: &IrResponse) -> Result<Vec<u8>, CodecError> {
        let mut text_parts: Vec<String> = Vec::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        for block in &response.content {
            match block.content_type {
                ContentType::Text => {
                    if let Some(text) = &block.text {
                        text_parts.push(text.text.clone());
                    }
                }
                ContentType::ToolUse => {
                    if let Some(tu) = &block.tool_use {
                        let args = match tu.arguments.as_ref() {
                            Some(serde_json::Value::String(s)) => s.clone(),
                            Some(v) => v.to_string(),
                            None => String::new(),
                        };
                        tool_calls.push(ToolCall {
                            id: tu.id.clone(),
                            call_type: "function".into(),
                            function: FunctionCall {
                                name: tu.name.clone(),
                                arguments: args,
                            },
                            index: None,
                        });
                    }
                }
                _ => {}
            }
        }

        let content = if text_parts.is_empty() && !tool_calls.is_empty() {
            None
        } else {
            Some(ChatContent::Text(text_parts.join("")))
        };

        let message = ChatMessage {
            role: "assistant".into(),
            content,
            name: None,
            tool_calls,
            tool_call_id: None,
        };

        let chat_resp = ChatCompletionResponse {
            id: response.id.clone().unwrap_or_default(),
            object: "chat.completion".into(),
            created: 0,
            model: response.model.clone().unwrap_or_default(),
            choices: vec![ChatChoice {
                index: 0,
                message,
                finish_reason: response.stop_reason.as_ref().map(|r| match r {
                    StopReason::EndTurn => "stop".into(),
                    StopReason::MaxTokens => "length".into(),
                    StopReason::ToolUse => "tool_calls".into(),
                    StopReason::StopSequence => "stop".into(),
                    StopReason::ContentFilter => "content_filter".into(),
                    StopReason::PauseTurn => "pause_turn".into(),
                    StopReason::Other(s) => s.clone(),
                }),
            }],
            usage: Some(ChatUsage {
                prompt_tokens: response.usage.input_tokens.unwrap_or(0),
                completion_tokens: response.usage.output_tokens.unwrap_or(0),
                total_tokens: response.usage.total_tokens.unwrap_or(0),
            }),
        };

        serde_json::to_vec(&chat_resp)
            .map_err(|e| CodecError::Encode(format!("failed to serialize chat response: {e}")))
    }

    fn decode_stream_event(
        &self,
        _event_type: Option<&str>,
        data: &str,
    ) -> Result<IrStreamEvent, CodecError> {
        let chunk: ChatCompletionChunk = serde_json::from_str(data)
            .map_err(|e| CodecError::Decode(format!("invalid stream chunk: {e}")))?;

        if let Some(choice) = chunk.choices.first() {
            if choice.delta.role.is_some() {
                return Ok(IrStreamEvent {
                    event_type: StreamEventType::ContentBlockStart,
                    response: Some(IrResponse {
                        id: Some(chunk.id),
                        model: Some(chunk.model),
                        content: Vec::new(),
                        stop_reason: None,
                        stop_sequence: None,
                        usage: IrUsage::default(),
                        provider_extensions: HashMap::new(),
                    }),
                    index: choice.index,
                    delta: Some(ContentBlock {
                        content_type: ContentType::Text,
                        text: Some(TextContent {
                            text: String::new(),
                        }),
                        ..Default::default()
                    }),
                    stop_reason: None,
                    usage: None,
                    error: None,
                });
            }

            if let Some(ref content) = choice.delta.content {
                return Ok(IrStreamEvent {
                    event_type: StreamEventType::Delta,
                    response: None,
                    index: choice.index,
                    delta: Some(ContentBlock {
                        content_type: ContentType::Text,
                        text: Some(TextContent {
                            text: content.clone(),
                        }),
                        ..Default::default()
                    }),
                    stop_reason: None,
                    usage: None,
                    error: None,
                });
            }

            if !choice.delta.tool_calls.is_empty() {
                if let Some(tc) = choice.delta.tool_calls.first() {
                    let name = tc
                        .function
                        .as_ref()
                        .and_then(|f| f.name.as_deref())
                        .unwrap_or("")
                        .to_string();
                    let arguments = tc
                        .function
                        .as_ref()
                        .and_then(|f| f.arguments.as_deref())
                        .unwrap_or("")
                        .to_string();
                    let id = tc.id.clone().unwrap_or_default();

                    let dt = ContentBlock {
                        content_type: ContentType::ToolUse,
                        tool_use: Some(ToolUseContent {
                            id,
                            name: name.clone(),
                            arguments: if !arguments.is_empty() {
                                Some(serde_json::Value::String(arguments))
                            } else {
                                None
                            },
                        }),
                        ..Default::default()
                    };

                    return Ok(IrStreamEvent {
                        event_type: StreamEventType::Delta,
                        response: None,
                        index: choice.index,
                        delta: Some(dt),
                        stop_reason: None,
                        usage: None,
                        error: None,
                    });
                }
            }

            if let Some(ref finish) = choice.finish_reason {
                let stop = match finish.as_str() {
                    "stop" => StopReason::EndTurn,
                    "length" => StopReason::MaxTokens,
                    "tool_calls" => StopReason::ToolUse,
                    "content_filter" => StopReason::ContentFilter,
                    s => StopReason::Other(s.into()),
                };

                return Ok(IrStreamEvent {
                    event_type: StreamEventType::Stop,
                    response: None,
                    index: choice.index,
                    delta: None,
                    stop_reason: Some(stop),
                    usage: chunk.usage.map(|u| IrUsage {
                        input_tokens: Some(u.prompt_tokens),
                        output_tokens: Some(u.completion_tokens),
                        total_tokens: Some(u.total_tokens),
                        ..Default::default()
                    }),
                    error: None,
                });
            }
        }

        Ok(IrStreamEvent {
            event_type: StreamEventType::Delta,
            response: None,
            index: 0,
            delta: None,
            stop_reason: None,
            usage: None,
            error: None,
        })
    }

    fn encode_stream_event(&self, event: &IrStreamEvent) -> Result<String, CodecError> {
        match event.event_type {
            StreamEventType::ContentBlockStart => {
                let chunk = ChatCompletionChunk {
                    id: event
                        .response
                        .as_ref()
                        .and_then(|r| r.id.clone())
                        .unwrap_or_default(),
                    object: "chat.completion.chunk".into(),
                    created: 0,
                    model: event
                        .response
                        .as_ref()
                        .and_then(|r| r.model.clone())
                        .unwrap_or_default(),
                    choices: vec![ChatChunkChoice {
                        index: event.index,
                        delta: ChatDelta {
                            role: Some("assistant".into()),
                            content: None,
                            tool_calls: Vec::new(),
                        },
                        finish_reason: None,
                    }],
                    usage: None,
                };
                Ok(format!(
                    "data: {}\n\n",
                    serde_json::to_string(&chunk).map_err(|e| CodecError::Encode(e.to_string()))?
                ))
            }
            StreamEventType::Delta => {
                let delta = event.delta.as_ref();
                let content = delta.and_then(|d| d.text.as_ref().map(|t| t.text.clone()));
                let tool_calls = delta
                    .map(|d| {
                        if let Some(tu) = &d.tool_use {
                            let args = tu
                                .arguments
                                .as_ref()
                                .map(|v| match v {
                                    serde_json::Value::String(s) => s.clone(),
                                    other => other.to_string(),
                                })
                                .unwrap_or_default();
                            vec![ToolCallDelta {
                                index: 0,
                                id: if tu.id.is_empty() {
                                    None
                                } else {
                                    Some(tu.id.clone())
                                },
                                call_type: Some("function".into()),
                                function: Some(FunctionCallDelta {
                                    name: if tu.name.is_empty() {
                                        None
                                    } else {
                                        Some(tu.name.clone())
                                    },
                                    arguments: if args.is_empty() { None } else { Some(args) },
                                }),
                            }]
                        } else {
                            Vec::new()
                        }
                    })
                    .unwrap_or_default();

                let chunk = ChatCompletionChunk {
                    id: String::new(),
                    object: "chat.completion.chunk".into(),
                    created: 0,
                    model: String::new(),
                    choices: vec![ChatChunkChoice {
                        index: event.index,
                        delta: ChatDelta {
                            role: None,
                            content,
                            tool_calls,
                        },
                        finish_reason: None,
                    }],
                    usage: None,
                };
                Ok(format!(
                    "data: {}\n\n",
                    serde_json::to_string(&chunk).map_err(|e| CodecError::Encode(e.to_string()))?
                ))
            }
            StreamEventType::Stop => {
                let finish_reason = event
                    .stop_reason
                    .as_ref()
                    .map(|r| match r {
                        StopReason::EndTurn => "stop",
                        StopReason::MaxTokens => "length",
                        StopReason::ToolUse => "tool_calls",
                        StopReason::StopSequence => "stop",
                        StopReason::ContentFilter => "content_filter",
                        StopReason::PauseTurn => "pause_turn",
                        StopReason::Other(s) => s.as_str(),
                    })
                    .unwrap_or("stop");

                let usage = event.usage.as_ref().map(|u| ChatUsage {
                    prompt_tokens: u.input_tokens.unwrap_or(0),
                    completion_tokens: u.output_tokens.unwrap_or(0),
                    total_tokens: u.total_tokens.unwrap_or(0),
                });

                let chunk = ChatCompletionChunk {
                    id: String::new(),
                    object: "chat.completion.chunk".into(),
                    created: 0,
                    model: String::new(),
                    choices: vec![ChatChunkChoice {
                        index: event.index,
                        delta: ChatDelta::default(),
                        finish_reason: Some(finish_reason.to_string()),
                    }],
                    usage,
                };
                let body =
                    serde_json::to_string(&chunk).map_err(|e| CodecError::Encode(e.to_string()))?;
                Ok(format!("data: {body}\n\ndata: [DONE]\n\n"))
            }
            StreamEventType::Error => {
                let msg = event
                    .error
                    .as_ref()
                    .and_then(|e| e.message.clone())
                    .unwrap_or_else(|| "unknown error".into());
                Ok(format!(
                    "data: {{\"error\":{{\"message\":\"{msg}\"}}}}\n\ndata: [DONE]\n\n"
                ))
            }
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

fn chat_content_to_blocks(content: &ChatContent) -> Vec<ContentBlock> {
    match content {
        ChatContent::Text(text) => {
            if text.is_empty() {
                Vec::new()
            } else {
                vec![ContentBlock {
                    content_type: ContentType::Text,
                    text: Some(TextContent { text: text.clone() }),
                    ..Default::default()
                }]
            }
        }
        ChatContent::Parts(parts) => parts
            .iter()
            .map(|p| match p.part_type.as_str() {
                "text" => ContentBlock {
                    content_type: ContentType::Text,
                    text: Some(TextContent {
                        text: p.text.clone().unwrap_or_default(),
                    }),
                    ..Default::default()
                },
                "image_url" => ContentBlock {
                    content_type: ContentType::Image,
                    image: p.image_url.as_ref().map(|img| ImageContent {
                        data: None,
                        url: Some(img.url.clone()),
                        media_type: None,
                        detail: img.detail.clone(),
                    }),
                    ..Default::default()
                },
                _ => ContentBlock::default(),
            })
            .collect(),
    }
}

fn parse_tool_choice(value: &serde_json::Value) -> Option<IrToolChoice> {
    match value {
        serde_json::Value::String(s) => match s.as_str() {
            "auto" => Some(IrToolChoice {
                choice_type: "auto".into(),
                tool_name: None,
                allowed_tool_names: Vec::new(),
                allow_parallel_calls: None,
            }),
            "none" => Some(IrToolChoice {
                choice_type: "none".into(),
                tool_name: None,
                allowed_tool_names: Vec::new(),
                allow_parallel_calls: None,
            }),
            "required" => Some(IrToolChoice {
                choice_type: "any".into(),
                tool_name: None,
                allowed_tool_names: Vec::new(),
                allow_parallel_calls: None,
            }),
            _ => None,
        },
        serde_json::Value::Object(obj) => {
            let choice_type = obj
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string();
            let tool_name = obj
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());
            Some(IrToolChoice {
                choice_type,
                tool_name,
                allowed_tool_names: Vec::new(),
                allow_parallel_calls: None,
            })
        }
        _ => None,
    }
}

fn blocks_to_chat_content(blocks: &[ContentBlock]) -> ChatContent {
    if blocks.len() == 1 && blocks[0].content_type == ContentType::Text {
        if let Some(text) = &blocks[0].text {
            return ChatContent::Text(text.text.clone());
        }
    }
    let parts: Vec<ContentPart> = blocks
        .iter()
        .filter_map(|b| match b.content_type {
            ContentType::Text => b.text.as_ref().map(|t| ContentPart {
                part_type: "text".into(),
                text: Some(t.text.clone()),
                image_url: None,
            }),
            ContentType::Image => b.image.as_ref().map(|img| ContentPart {
                part_type: "image_url".into(),
                text: None,
                image_url: Some(ImageUrl {
                    url: img.url.clone().unwrap_or_default(),
                    detail: img.detail.clone(),
                }),
            }),
            _ => None,
        })
        .collect();
    ChatContent::Parts(parts)
}
