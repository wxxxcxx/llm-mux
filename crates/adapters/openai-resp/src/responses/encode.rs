use std::collections::HashMap;

use llm_mux_core::codec::CodecError;
use llm_mux_core::ir::{IrResponse, IrUsage};
use llm_mux_core::types::{ContentBlock, ContentType, StopReason, TextContent, ToolUseContent};

use crate::models as m;

pub fn decode_response_impl(body: &[u8]) -> Result<IrResponse, CodecError> {
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

pub fn encode_response_impl(response: &IrResponse) -> Result<Vec<u8>, CodecError> {
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
