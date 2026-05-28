#![allow(dead_code)]
// 辅助转换函数

use llm_mux_core::ir::{IrToolChoice, IrUsage};
use llm_mux_core::types::{
    ContentBlock as IrBlock, ContentType, ImageContent, RedactedThinkingContent,
    TextContent, ThinkingContent, ToolResultContent, ToolUseContent,
};

use crate::models as m;

pub(crate) fn anthropic_content_to_blocks(content: &m::Content) -> Vec<IrBlock> {
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

pub(crate) fn m_block_to_ir(block: &m::ContentBlock) -> IrBlock {
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

pub(crate) fn tool_result_inner(block: &m::ContentBlock) -> Vec<IrBlock> {
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

pub(crate) fn parse_anthropic_tool_choice(value: &serde_json::Value) -> Option<IrToolChoice> {
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

pub(crate) fn m_usage_to_ir(u: &m::Usage) -> IrUsage {
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

pub(crate) fn ir_usage_to_m(u: &IrUsage) -> m::Usage {
    m::Usage {
        input_tokens: u.input_tokens,
        output_tokens: u.output_tokens,
        cache_read_input_tokens: u.cache_read_tokens,
        cache_creation_input_tokens: u.cache_creation_tokens,
    }
}

pub(crate) fn block_type_to_ir(bt: &str) -> ContentType {
    match bt {
        "text" => ContentType::Text,
        "tool_use" => ContentType::ToolUse,
        "thinking" => ContentType::Thinking,
        "redacted_thinking" => ContentType::RedactedThinking,
        "image" => ContentType::Image,
        _ => ContentType::Text,
    }
}

pub(crate) fn delta_type_to_ir(dt: &str) -> ContentType {
    match dt {
        "text_delta" => ContentType::Text,
        "input_json_delta" => ContentType::ToolUse,
        "thinking_delta" => ContentType::Thinking,
        _ => ContentType::Text,
    }
}

pub(crate) fn tool_use_start(block: &m::ContentBlock) -> Option<ToolUseContent> {
    (block.block_type == "tool_use").then(|| ToolUseContent {
        id: block.id.clone().unwrap_or_default(),
        name: block.name.clone().unwrap_or_default(),
        arguments: None,
    })
}

pub(crate) fn thinking_block(block: &m::ContentBlock) -> Option<ThinkingContent> {
    (block.block_type == "thinking").then(|| ThinkingContent {
        thinking: block.thinking.clone().unwrap_or_default(),
        signature: block.signature.clone(),
    })
}

pub(crate) fn redacted_thinking_block(block: &m::ContentBlock) -> Option<RedactedThinkingContent> {
    (block.block_type == "redacted_thinking").then(|| RedactedThinkingContent {
        data: block.data.clone().unwrap_or_default(),
    })
}

pub(crate) fn ir_block_type_to_anthropic_str(block: &IrBlock) -> String {
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
