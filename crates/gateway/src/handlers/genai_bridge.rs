use llm_mux_core::ir::{IrResponse, IrUsage};
use llm_mux_core::types::{ContentBlock, ContentType, TextContent};

/// 将 genai ChatResponse 桥接为内部 IrResponse。
pub fn ir_from_genai_response(
    genai_resp: &genai::chat::ChatResponse,
    model: &str,
) -> IrResponse {
    let text = genai_resp.first_text().unwrap_or("").to_string();
    let stop_reason = genai_resp
        .stop_reason
        .as_ref()
        .map(llm_mux_core::ir::stop_reason_from_genai);
    let content_block = ContentBlock {
        content_type: ContentType::Text,
        text: Some(TextContent { text }),
        ..Default::default()
    };
    IrResponse {
        id: Some(format!("msg_{}", uuid::Uuid::new_v4())),
        model: Some(model.to_string()),
        content: vec![content_block],
        stop_reason,
        stop_sequence: None,
        usage: IrUsage {
            input_tokens: genai_resp.usage.prompt_tokens.map(|n| n as i64),
            output_tokens: genai_resp.usage.completion_tokens.map(|n| n as i64),
            total_tokens: genai_resp.usage.total_tokens.map(|n| n as i64),
            ..Default::default()
        },
        provider_extensions: Default::default(),
    }
}
