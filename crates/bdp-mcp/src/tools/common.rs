// crates/bdp-mcp/src/tools/common.rs

use rmcp::model::{CallToolResult, Content};

pub fn decode_cursor(cursor: Option<&str>) -> i64 {
    cursor
        .and_then(|c| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.decode(c).ok()
        })
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v["offset"].as_i64())
        .unwrap_or(0)
}

pub fn encode_cursor(offset: i64) -> String {
    use base64::Engine;
    let json = serde_json::json!({"offset": offset}).to_string();
    base64::engine::general_purpose::STANDARD.encode(json)
}

pub fn clamp_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(50).clamp(1, 200)
}

/// Build a not_yet_available stub result (is_error: false — planned capability, not a failure).
pub fn stub_result(tool_name: &str, reason: &str, tracking: &str) -> CallToolResult {
    let payload = serde_json::json!({
        "status": "not_yet_available",
        "tool": tool_name,
        "reason": reason,
        "tracking": tracking,
        "expected": "2026-Q3"
    });
    let text = format!("{tool_name}: {reason} (tracked: {tracking})");
    let mut result = CallToolResult::success(vec![Content::text(text)]);
    result.structured_content = Some(payload);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_roundtrip() {
        let encoded = encode_cursor(50);
        assert_eq!(decode_cursor(Some(&encoded)), 50);
    }

    #[test]
    fn test_clamp_limit() {
        assert_eq!(clamp_limit(None), 50);
        assert_eq!(clamp_limit(Some(300)), 200);
        assert_eq!(clamp_limit(Some(0)), 1);
    }

    #[test]
    fn test_stub_result_is_not_error() {
        let r = stub_result("test_tool", "needs pipeline", "BDP-99");
        assert!(!r.is_error.unwrap_or(false));
        let s = r.structured_content.unwrap();
        assert_eq!(s["status"], "not_yet_available");
        assert_eq!(s["tracking"], "BDP-99");
    }
}
