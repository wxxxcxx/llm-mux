use llm_mux_core::codec::CodecError;
use llm_mux_core::ir::IrResponse;
use llm_mux_core::types::{ContentType, StopReason};

use crate::models::*;

pub fn encode_response_impl(response: &IrResponse) -> Result<Vec<u8>, CodecError> {
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
        reasoning_content: None,
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
