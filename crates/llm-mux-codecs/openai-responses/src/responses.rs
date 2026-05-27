use std::collections::HashMap;

use llm_mux_core::codec::{Codec, CodecError};
use llm_mux_core::ir::{
    IrMessage, IrRequest, IrResponse, IrResponseFormat, IrStreamEvent, IrTool, IrToolChoice,
    IrUsage, StreamEventType,
};
use llm_mux_core::types::{
    ContentBlock, ContentType, Protocol, Role, StopReason, TextContent, ToolResultContent,
    ToolUseContent,
};

use crate::models as m;

pub struct ResponsesCodec;

impl Codec for ResponsesCodec {
    fn protocol(&self) -> Protocol {
        Protocol::OpenAiResponses
    }

    fn known_fields(&self) -> &HashMap<String, bool> {
        static FIELDS: std::sync::LazyLock<HashMap<String, bool>> =
            std::sync::LazyLock::new(|| {
                let mut m = HashMap::new();
                m.insert("model".into(), true);
                m.insert("input".into(), true);
                m.insert("instructions".into(), true);
                m.insert("tools".into(), true);
                m.insert("tool_choice".into(), true);
                m.insert("stream".into(), true);
                m.insert("temperature".into(), true);
                m.insert("top_p".into(), true);
                m.insert("max_output_tokens".into(), true);
                m.insert("previous_response_id".into(), true);
                m.insert("text".into(), true);
                m
            });
        &FIELDS
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
                    if !self.known_fields().contains_key(key.as_str()) {
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

    fn encode_request(&self, request: &IrRequest) -> Result<Vec<u8>, CodecError> {
        let instructions = if request.system_prompt.is_empty() {
            None
        } else {
            let text: String = request
                .system_prompt
                .iter()
                .filter_map(|b| b.text.as_ref().map(|t| t.text.as_str()))
                .collect::<Vec<_>>()
                .join("");
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        };

        let mut input: Vec<m::ResponseInputItem> = Vec::new();

        for msg in &request.messages {
            match msg.role {
                Role::User => {
                    for block in &msg.content {
                        if let Some(t) = &block.text {
                            input.push(m::ResponseInputItem::Text {
                                text: t.text.clone(),
                            });
                        }
                    }
                }
                Role::Assistant => {
                    for block in &msg.content {
                        match block.content_type {
                            ContentType::Text => {
                                if let Some(t) = &block.text {
                                    input.push(m::ResponseInputItem::OutputText {
                                        text: t.text.clone(),
                                        annotations: Vec::new(),
                                    });
                                }
                            }
                            ContentType::ToolUse => {
                                if let Some(tu) = &block.tool_use {
                                    let args = match &tu.arguments {
                                        Some(serde_json::Value::String(s)) => s.clone(),
                                        Some(v) => v.to_string(),
                                        None => String::new(),
                                    };
                                    input.push(m::ResponseInputItem::FunctionCall {
                                        call_id: tu.id.clone(),
                                        name: tu.name.clone(),
                                        arguments: args,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Role::Tool => {
                    for block in &msg.content {
                        if let Some(tr) = &block.tool_result {
                            let output: String = tr
                                .content
                                .iter()
                                .filter_map(|b| b.text.as_ref().map(|t| t.text.as_str()))
                                .collect::<Vec<_>>()
                                .join("");
                            input.push(m::ResponseInputItem::FunctionCallOutput {
                                call_id: tr.tool_use_id.clone(),
                                output,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        let tools: Option<Vec<serde_json::Value>> = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "type": t.r#type.as_deref().unwrap_or("function"),
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                            "strict": t.strict,
                        })
                    })
                    .collect(),
            )
        };

        let tool_choice = request.tool_choice.as_ref().map(|tc| match tc.choice_type.as_str() {
            "auto" | "none" | "required" => serde_json::json!(tc.choice_type),
            "any" => serde_json::json!("required"),
            _ => serde_json::json!({"type": "function", "function": {"name": tc.tool_name.clone().unwrap_or_default()}}),
        });

        let text = request
            .response_format
            .as_ref()
            .map(|rf| m::ResponseTextConfig {
                format: Some(m::ResponseFormat {
                    format_type: rf.format_type.clone(),
                    json_schema: rf.json_schema.clone(),
                }),
            });

        let previous_response_id = request
            .provider_extensions
            .get("previous_response_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let req = m::ResponsesRequest {
            model: request.model.clone(),
            instructions,
            input,
            tools,
            tool_choice,
            stream: request.stream,
            temperature: request.temperature,
            top_p: request.top_p,
            max_output_tokens: request.max_tokens,
            previous_response_id,
            text,
            extra: request
                .provider_extensions
                .clone()
                .into_iter()
                .filter(|(k, _)| k != "previous_response_id")
                .map(|(k, v)| (k, v.clone()))
                .collect(),
        };

        serde_json::to_vec(&req)
            .map_err(|e| CodecError::Encode(format!("failed to serialize responses request: {e}")))
    }

    fn decode_response(&self, body: &[u8]) -> Result<IrResponse, CodecError> {
        let resp: m::ResponsesResponse = serde_json::from_slice(body)?;

        let mut content = Vec::new();

        for item in &resp.output {
            match item {
                m::ResponseOutputItem::Message { content: parts, .. } => {
                    for part in parts {
                        match part {
                            m::ResponseContentPart::OutputText { text, .. } => {
                                content.push(ContentBlock {
                                    content_type: ContentType::Text,
                                    text: Some(TextContent { text: text.clone() }),
                                    ..Default::default()
                                });
                            }
                            m::ResponseContentPart::Refusal { refusal } => {
                                content.push(ContentBlock {
                                    content_type: ContentType::Refusal,
                                    refusal: Some(llm_mux_core::types::RefusalContent {
                                        refusal: refusal.clone(),
                                    }),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
                m::ResponseOutputItem::FunctionCall {
                    call_id,
                    name,
                    arguments,
                    ..
                } => {
                    let args: serde_json::Value = serde_json::from_str(arguments)
                        .unwrap_or(serde_json::Value::String(arguments.clone()));
                    content.push(ContentBlock {
                        content_type: ContentType::ToolUse,
                        tool_use: Some(ToolUseContent {
                            id: call_id.clone(),
                            name: name.clone(),
                            arguments: Some(args),
                        }),
                        ..Default::default()
                    });
                }
            }
        }

        let stop_reason = resp.status.as_deref().map(|s| match s {
            "completed" => StopReason::EndTurn,
            "incomplete" => StopReason::MaxTokens,
            _ => StopReason::Other(s.to_string()),
        });

        Ok(IrResponse {
            id: Some(resp.id),
            model: Some(resp.model),
            content,
            stop_reason,
            stop_sequence: None,
            usage: IrUsage {
                input_tokens: resp.usage.as_ref().map(|u| u.input_tokens),
                output_tokens: resp.usage.as_ref().map(|u| u.output_tokens),
                total_tokens: resp.usage.as_ref().map(|u| u.total_tokens),
                ..Default::default()
            },
            provider_extensions: HashMap::new(),
        })
    }

    fn encode_response(&self, response: &IrResponse) -> Result<Vec<u8>, CodecError> {
        let mut output: Vec<m::ResponseOutputItem> = Vec::new();

        let mut text_parts = Vec::new();
        let mut tool_use_items = Vec::new();

        for block in &response.content {
            match block.content_type {
                ContentType::Text => {
                    if let Some(t) = &block.text {
                        text_parts.push(t.text.clone());
                    }
                }
                ContentType::ToolUse => {
                    if let Some(tu) = &block.tool_use {
                        let args = match &tu.arguments {
                            Some(serde_json::Value::String(s)) => s.clone(),
                            Some(v) => v.to_string(),
                            None => String::new(),
                        };
                        tool_use_items.push(m::ResponseOutputItem::FunctionCall {
                            id: tu.id.clone(),
                            call_id: tu.id.clone(),
                            name: tu.name.clone(),
                            arguments: args,
                            status: Some("completed".into()),
                        });
                    }
                }
                _ => {}
            }
        }

        if !text_parts.is_empty() {
            let content_parts: Vec<m::ResponseContentPart> = text_parts
                .iter()
                .map(|t| m::ResponseContentPart::OutputText {
                    text: t.clone(),
                    annotations: Vec::new(),
                })
                .collect();
            output.push(m::ResponseOutputItem::Message {
                id: "msg_0".into(),
                role: "assistant".into(),
                content: content_parts,
                status: Some("completed".into()),
            });
        }

        output.extend(tool_use_items);

        let resp = m::ResponsesResponse {
            id: response.id.clone().unwrap_or_default(),
            object: "response".into(),
            model: response.model.clone().unwrap_or_default(),
            output,
            usage: Some(m::ResponseUsage {
                input_tokens: response.usage.input_tokens.unwrap_or(0),
                output_tokens: response.usage.output_tokens.unwrap_or(0),
                total_tokens: response.usage.total_tokens.unwrap_or(0),
            }),
            status: match response.stop_reason.as_ref() {
                Some(StopReason::EndTurn) => Some("completed".into()),
                Some(StopReason::MaxTokens) => Some("incomplete".into()),
                Some(other) => Some(format!("{:?}", other)),
                None => Some("completed".into()),
            },
        };

        serde_json::to_vec(&resp)
            .map_err(|e| CodecError::Encode(format!("failed to serialize responses response: {e}")))
    }

    fn decode_stream_event(
        &self,
        _event_type: Option<&str>,
        data: &str,
    ) -> Result<IrStreamEvent, CodecError> {
        let event: m::ResponsesStreamEvent = serde_json::from_str(data)
            .map_err(|e| CodecError::Decode(format!("invalid stream event: {e}")))?;

        match event {
            m::ResponsesStreamEvent::ResponseCreated { response } => {
                let usage = response.usage.as_ref().map(|u| IrUsage {
                    input_tokens: Some(u.input_tokens),
                    output_tokens: Some(u.output_tokens),
                    total_tokens: Some(u.total_tokens),
                    ..Default::default()
                });
                Ok(IrStreamEvent {
                    event_type: StreamEventType::Start,
                    response: Some(IrResponse {
                        id: Some(response.id),
                        model: Some(response.model),
                        content: Vec::new(),
                        stop_reason: None,
                        stop_sequence: None,
                        usage: IrUsage::default(),
                        provider_extensions: HashMap::new(),
                    }),
                    index: 0,
                    delta: None,
                    stop_reason: None,
                    usage,
                    error: None,
                })
            }
            m::ResponsesStreamEvent::ResponseInProgress { .. } => Ok(IrStreamEvent {
                event_type: StreamEventType::Delta,
                response: None,
                index: 0,
                delta: None,
                stop_reason: None,
                usage: None,
                error: None,
            }),
            m::ResponsesStreamEvent::OutputItemAdded { output_index, item } => {
                let delta = match &item {
                    m::ResponseOutputItem::Message { .. } => Some(ContentBlock {
                        content_type: ContentType::Text,
                        ..Default::default()
                    }),
                    m::ResponseOutputItem::FunctionCall { call_id, name, .. } => {
                        Some(ContentBlock {
                            content_type: ContentType::ToolUse,
                            tool_use: Some(ToolUseContent {
                                id: call_id.clone(),
                                name: name.clone(),
                                arguments: None,
                            }),
                            ..Default::default()
                        })
                    }
                };
                Ok(IrStreamEvent {
                    event_type: StreamEventType::ContentBlockStart,
                    response: None,
                    index: output_index,
                    delta,
                    stop_reason: None,
                    usage: None,
                    error: None,
                })
            }
            m::ResponsesStreamEvent::ContentPartAdded {
                item_id: _,
                output_index,
                content_index: _,
                part: _,
            } => Ok(IrStreamEvent {
                event_type: StreamEventType::Delta,
                response: None,
                index: output_index,
                delta: None,
                stop_reason: None,
                usage: None,
                error: None,
            }),
            m::ResponsesStreamEvent::OutputTextDelta {
                item_id: _,
                output_index,
                content_index: _,
                delta,
            } => Ok(IrStreamEvent {
                event_type: StreamEventType::Delta,
                response: None,
                index: output_index,
                delta: Some(ContentBlock {
                    content_type: ContentType::Text,
                    text: Some(TextContent { text: delta }),
                    ..Default::default()
                }),
                stop_reason: None,
                usage: None,
                error: None,
            }),
            m::ResponsesStreamEvent::FunctionCallArgumentsDelta {
                item_id,
                output_index,
                delta,
            } => Ok(IrStreamEvent {
                event_type: StreamEventType::Delta,
                response: None,
                index: output_index,
                delta: Some(ContentBlock {
                    content_type: ContentType::ToolUse,
                    tool_use: Some(ToolUseContent {
                        id: item_id,
                        name: String::new(),
                        arguments: Some(serde_json::Value::String(delta)),
                    }),
                    ..Default::default()
                }),
                stop_reason: None,
                usage: None,
                error: None,
            }),
            m::ResponsesStreamEvent::FunctionCallArgumentsDone {
                item_id: _,
                output_index,
                arguments: _,
            } => Ok(IrStreamEvent {
                event_type: StreamEventType::ContentBlockStop,
                response: None,
                index: output_index,
                delta: None,
                stop_reason: None,
                usage: None,
                error: None,
            }),
            m::ResponsesStreamEvent::OutputItemDone { item: _ } => Ok(IrStreamEvent {
                event_type: StreamEventType::ContentBlockStop,
                response: None,
                index: 0,
                delta: None,
                stop_reason: None,
                usage: None,
                error: None,
            }),
            m::ResponsesStreamEvent::OutputTextAnnotationAdded { .. } => Ok(IrStreamEvent {
                event_type: StreamEventType::Delta,
                response: None,
                index: 0,
                delta: None,
                stop_reason: None,
                usage: None,
                error: None,
            }),
            m::ResponsesStreamEvent::ResponseCompleted { response } => {
                let usage = response.usage.as_ref().map(|u| IrUsage {
                    input_tokens: Some(u.input_tokens),
                    output_tokens: Some(u.output_tokens),
                    total_tokens: Some(u.total_tokens),
                    ..Default::default()
                });
                Ok(IrStreamEvent {
                    event_type: StreamEventType::Stop,
                    response: None,
                    index: 0,
                    delta: None,
                    stop_reason: Some(StopReason::EndTurn),
                    usage,
                    error: None,
                })
            }
            m::ResponsesStreamEvent::Error { code, message } => Ok(IrStreamEvent {
                event_type: StreamEventType::Error,
                response: None,
                index: 0,
                delta: None,
                stop_reason: None,
                usage: None,
                error: Some(llm_mux_core::ir::StreamError {
                    error_type: Some("api_error".into()),
                    code,
                    message: Some(message),
                    param: None,
                }),
            }),
        }
    }

    fn encode_stream_event(&self, event: &IrStreamEvent) -> Result<String, CodecError> {
        let json = match event.event_type {
            StreamEventType::Start => {
                let resp = event.response.as_ref().map(|r| {
                    serde_json::json!({
                        "id": r.id.as_deref().unwrap_or(""),
                        "object": "response",
                        "model": r.model.as_deref().unwrap_or(""),
                        "output": [],
                        "usage": event.usage.as_ref().map(|u| serde_json::json!({
                            "input_tokens": u.input_tokens.unwrap_or(0),
                            "output_tokens": u.output_tokens.unwrap_or(0),
                            "total_tokens": u.total_tokens.unwrap_or(0),
                        })),
                    })
                });
                serde_json::json!({
                    "type": "response.created",
                    "response": resp,
                })
            }
            StreamEventType::ContentBlockStart => {
                let item = match event.delta.as_ref().map(|d| d.content_type.clone()) {
                    Some(ContentType::ToolUse) => {
                        let tu = event.delta.as_ref().and_then(|d| d.tool_use.as_ref());
                        let id = tu.map(|t| t.id.as_str()).unwrap_or("");
                        let name = tu.map(|t| t.name.as_str()).unwrap_or("");
                        serde_json::json!({
                            "type": "function_call",
                            "id": id,
                            "call_id": id,
                            "name": name,
                            "arguments": "",
                            "status": "in_progress",
                        })
                    }
                    _ => {
                        serde_json::json!({
                            "type": "message",
                            "id": "msg_0",
                            "role": "assistant",
                            "content": [],
                            "status": "in_progress",
                        })
                    }
                };
                serde_json::json!({
                    "type": "response.output_item.added",
                    "output_index": event.index,
                    "item": item,
                })
            }
            StreamEventType::Delta => match event.delta.as_ref().map(|d| d.content_type.clone()) {
                Some(ContentType::ToolUse) => {
                    let delta_str = event
                        .delta
                        .as_ref()
                        .and_then(|d| d.tool_use.as_ref())
                        .and_then(|tu| tu.arguments.as_ref())
                        .map(|v| match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .unwrap_or_default();
                    let item_id = event
                        .delta
                        .as_ref()
                        .and_then(|d| d.tool_use.as_ref())
                        .map(|tu| tu.id.clone())
                        .unwrap_or_default();
                    serde_json::json!({
                        "type": "response.function_call_arguments.delta",
                        "item_id": item_id,
                        "output_index": event.index,
                        "delta": delta_str,
                    })
                }
                _ => {
                    let text = event
                        .delta
                        .as_ref()
                        .and_then(|d| d.text.as_ref())
                        .map(|t| t.text.clone())
                        .unwrap_or_default();
                    serde_json::json!({
                        "type": "response.output_text.delta",
                        "item_id": "msg_0",
                        "output_index": event.index,
                        "content_index": 0,
                        "delta": text,
                    })
                }
            },
            StreamEventType::ContentBlockStop => {
                serde_json::json!({
                    "type": "response.output_item.done",
                    "output_index": event.index,
                })
            }
            StreamEventType::Stop => {
                let usage = event.usage.as_ref().map(|u| {
                    serde_json::json!({
                        "input_tokens": u.input_tokens.unwrap_or(0),
                        "output_tokens": u.output_tokens.unwrap_or(0),
                        "total_tokens": u.total_tokens.unwrap_or(0),
                    })
                });
                serde_json::json!({
                    "type": "response.completed",
                    "response": {
                        "id": event.response.as_ref().and_then(|r| r.id.clone()).unwrap_or_default(),
                        "object": "response",
                        "model": event.response.as_ref().and_then(|r| r.model.clone()).unwrap_or_default(),
                        "output": [],
                        "usage": usage,
                        "status": "completed",
                    },
                })
            }
            StreamEventType::Error => {
                let msg = event
                    .error
                    .as_ref()
                    .and_then(|e| e.message.clone())
                    .unwrap_or_else(|| "unknown error".into());
                serde_json::json!({
                    "type": "error",
                    "code": event.error.as_ref().and_then(|e| e.code.clone()),
                    "message": msg,
                })
            }
        };
        let s = serde_json::to_string(&json).map_err(|e| CodecError::Encode(e.to_string()))?;
        Ok(format!("data: {s}\n\n"))
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
