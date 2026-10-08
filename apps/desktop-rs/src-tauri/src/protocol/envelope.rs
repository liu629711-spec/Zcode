//! zcode-protocol 信封层。
//!
//! 对应 `packages/shared/src/zcode-protocol/index.ts` 的信封定义
//! （zcodeProtocolRequestSchema / Notification / Response / Error / Message）。
//! 协议版本：ZCODE_PROTOCOL_VERSION = 1，V4 wire version = 3。
//!
//! 注意：TS 侧 schema 为 `.strict()`；Rust 侧信封层暂不 `deny_unknown_fields`，
//! 避免对端新增字段时反序列化直接失败——严格校验放在业务层按需补齐。

use serde::{Deserialize, Serialize};

/// 请求 id：字符串或整数（zcodeProtocolRequestIdSchema）。
/// 需要 Eq/Hash：作为 client pending 表的 HashMap key。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Str(String),
    Int(i64),
}

/// 可选链路追踪信息（zcodeProtocolTraceSchema）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Trace {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traceparent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(rename = "parentId", skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
}

/// 请求（有 id，期待响应）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolRequest {
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Trace>,
}

/// 通知（无 id，不期待响应）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolNotification {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Trace>,
}

/// 成功响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolResponse {
    pub id: RequestId,
    pub result: serde_json::Value,
}

/// 错误响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolError {
    pub id: RequestId,
    pub error: ProtocolErrorBody,
}

/// 错误响应体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolErrorBody {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// 协议消息联合（zcodeProtocolMessageSchema 的 untagged union）。
/// serde untagged 依序尝试；四个变体的必填字段互斥，顺序安全。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProtocolMessage {
    Response(ProtocolResponse),
    Error(ProtocolError),
    Request(ProtocolRequest),
    Notification(ProtocolNotification),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let json = r#"{"id":"r1","method":"session.list","params":{"a":1}}"#;
        let msg: ProtocolMessage = serde_json::from_str(json).unwrap();
        match &msg {
            ProtocolMessage::Request(req) => {
                assert_eq!(req.id, RequestId::Str("r1".into()));
                assert_eq!(req.method, "session.list");
            }
            other => panic!("expected request, got {other:?}"),
        }
        let back = serde_json::to_string(&msg).unwrap();
        assert!(back.contains("\"method\":\"session.list\""));
    }

    #[test]
    fn notification_without_id() {
        let json = r#"{"method":"session.updated"}"#;
        let msg: ProtocolMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ProtocolMessage::Notification(_)));
    }

    #[test]
    fn response_with_numeric_id() {
        let json = r#"{"id":7,"result":{"ok":true}}"#;
        let msg: ProtocolMessage = serde_json::from_str(json).unwrap();
        match &msg {
            ProtocolMessage::Response(res) => assert_eq!(res.id, RequestId::Int(7)),
            other => panic!("expected response, got {other:?}"),
        }
    }

    #[test]
    fn error_message() {
        let json = r#"{"id":"r2","error":{"code":-32601,"message":"method not found"}}"#;
        let msg: ProtocolMessage = serde_json::from_str(json).unwrap();
        match &msg {
            ProtocolMessage::Error(err) => assert_eq!(err.error.code, -32601),
            other => panic!("expected error, got {other:?}"),
        }
    }
}
