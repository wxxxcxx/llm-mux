#![allow(dead_code)]
// 辅助转换函数

use llm_mux_core::ir::IrToolChoice;
use llm_mux_core::types::{ContentBlock, ContentType, ImageContent, TextContent};

use crate::models::*;

/// Fields that are Anthropic-specific and should not be forwarded to OpenAI Chat API.
pub(crate) fn is_anthropic_only_field(key: &str) -> bool {
    matches!(
        key,
        "context_management"
            | "metadata"
            | "service_tier"
            | "thinking"
            | "cache_control"
            | "display_name"
            | "display_text"
    )
}

pub(crate) fn chat_content_to_blocks(content: &ChatContent) -> Vec<ContentBlock> {
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

pub(crate) fn parse_tool_choice(value: &serde_json::Value) -> Option<IrToolChoice> {
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

pub(crate) fn blocks_to_chat_content(blocks: &[ContentBlock]) -> ChatContent {
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
