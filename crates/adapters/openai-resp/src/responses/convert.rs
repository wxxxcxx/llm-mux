use llm_mux_core::ir::IrToolChoice;

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
