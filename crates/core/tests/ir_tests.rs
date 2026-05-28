#[cfg(test)]
mod tests {
    use llm_mux_core::ir::{IrMessage, IrRequest, IrResponse, IrUsage};
    use llm_mux_core::types::{ContentBlock, ContentType, Protocol, Role, TextContent};

    #[test]
    fn ir_request_serialization_roundtrip() {
        let req = IrRequest {
            model: "gpt-4o".into(),
            messages: vec![IrMessage {
                role: Role::User,
                content: vec![ContentBlock {
                    content_type: ContentType::Text,
                    text: Some(TextContent {
                        text: "Hello".into(),
                    }),
                    image: None,
                    tool_use: None,
                    tool_result: None,
                    server_tool_use: None,
                    web_search_tool_result: None,
                    document: None,
                    thinking: None,
                    redacted_thinking: None,
                    refusal: None,
                    citations: Vec::new(),
                }],
            }],
            ..IrRequest::new("gpt-4o".into(), Protocol::OpenAiChat)
        };

        let json = serde_json::to_string(&req).unwrap();
        let decoded: IrRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.model, "gpt-4o");
        assert_eq!(decoded.messages.len(), 1);
        assert_eq!(decoded.messages[0].role, Role::User);
        assert_eq!(
            decoded.messages[0].content[0].text.as_ref().unwrap().text,
            "Hello"
        );
    }

    #[test]
    fn ir_response_usage_defaults() {
        let resp = IrResponse {
            id: Some("resp_123".into()),
            model: Some("gpt-4o".into()),
            content: Vec::new(),
            stop_reason: None,
            stop_sequence: None,
            usage: IrUsage::default(),
            provider_extensions: Default::default(),
        };

        let json = serde_json::to_string(&resp).unwrap();
        let decoded: IrResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.id.as_deref(), Some("resp_123"));
    }

    #[test]
    fn ir_request_stream_default() {
        let req = IrRequest::new("claude-sonnet-4-6".into(), Protocol::Anthropic);
        assert!(!req.is_streaming());
        assert!(!req.has_tools());
        assert_eq!(req.inbound_protocol(), Protocol::Anthropic);
    }

    #[test]
    fn protocol_serde() {
        assert_eq!(
            serde_json::to_string(&Protocol::OpenAiChat).unwrap(),
            r#""openai_chat""#
        );
        assert_eq!(
            serde_json::to_string(&Protocol::Anthropic).unwrap(),
            r#""anthropic""#
        );
    }
}
