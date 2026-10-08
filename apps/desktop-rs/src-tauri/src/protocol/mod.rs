//! zcode-protocol 的 Rust 实现。
//!
//! 协议事实标准在 `packages/shared/src/zcode-protocol/index.ts`（3811 行），
//! 本模块按层逐步翻译：先信封层（envelope），再通知/请求方法表与业务载荷。
//! 只做协议客户端实现，不改协议本身。

pub mod envelope;

pub use envelope::{
    ProtocolError, ProtocolErrorBody, ProtocolMessage, ProtocolNotification, ProtocolRequest,
    ProtocolResponse, RequestId, Trace,
};

/// 协议常量（与 TS 侧保持一致）。
pub const PROTOCOL_NAME: &str = "ZCode Protocol";
pub const PROTOCOL_VERSION: i64 = 1;
pub const V4_WIRE_VERSION: i64 = 3;
