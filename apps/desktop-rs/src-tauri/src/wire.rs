//! wire 分片重组（对齐真源 `wire-assembler.ts` 574 行 + `wire-binary.ts`）。
//!
//! 真源有两层实现，本模块对应**生产用的第二层**（增量状态机）：
//! - L1 `wire-reassembly.ts`：批量纯函数，仓库里没有生产调用点（只被 assembler 借了两个纯函数）
//! - L2 `wire-assembler.ts`：`TopicWireFrameAssembler.accept()` 逐片状态机，**生产走这条**
//!
//! 分组键 = `topic + "\0" + subscriptionId`（wire-assembler.ts:115-117），
//! **不含 logicalFrameId** ——后者是路由内的身份校验与淘汰依据。
//!
//! 校验顺序不可改（省 CPU + 便于定位 fault）：
//! `ordinal/墓碑 → envelope 校验 → base64 → 组一致性 → 收齐 → 长度 → CRC32 → UTF-8 → JSON → schema`
//!
//! **两条容易踩的规则**：
//! 1. 分片必然切断多字节 UTF-8 字符（chunkBytes 是任意字节数，无字符边界对齐），
//!    所以绝不能逐片解成 String，只能全部字节拼完后一次性解码。
//! 2. fault 不抛错也不中断流——真源把 fault 当瞬态，恢复阶梯是
//!    same-sub resync → forceSnapshot → fail closed（wire-fault.ts:1-11）。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::conversation_stream::{TopicFrame, WireFrame, WireFrameKind};

/// 协议限额（真源 `core.ts:66-70`）。
pub const MAX_LOGICAL_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_FRAGMENTS: usize = 1024;
pub const MAX_CONCURRENT: usize = 32;
pub const MAX_STAGED_BYTES: usize = 32 * 1024 * 1024;
/// 组装超时（真源 logicalFrameAssemblyTimeoutMs）。
pub const TIMEOUT_MS: u64 = 30_000;

/// fault 原因码（真源 wire-assembler.ts 各处 `this.fault(assembly, reason)`）。
/// 语义见 `wire-fault.ts:19-30`：只有 InvalidPayload 是确定性内容失败。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultCode {
    MetadataMismatch,
    OrdinalConflict,
    Superseded,
    InvalidBase64,
    InvalidUtf8,
    InvalidJson,
    LengthMismatch,
    ChecksumMismatch,
    FragmentConflict,
    InconsistentMetadata,
    TimedOut,
    TooManyAssemblies,
    StagedBudgetExceeded,
    InvalidPayload,
    IncompleteFrame,
}

impl FaultCode {
    /// 真源 reasonCode 字符串（便于对照日志）。
    pub fn reason_code(self) -> &'static str {
        match self {
            Self::MetadataMismatch => "proto.frameAssemblyMetadataMismatch",
            Self::OrdinalConflict => "proto.frameAssemblyOrdinalConflict",
            Self::Superseded => "proto.frameAssemblySuperseded",
            Self::InvalidBase64 => "proto.frameAssemblyInvalidBase64",
            Self::InvalidUtf8 => "proto.frameAssemblyInvalidUtf8",
            Self::InvalidJson => "proto.frameAssemblyInvalidJson",
            Self::LengthMismatch => "proto.frameAssemblyLengthMismatch",
            Self::ChecksumMismatch => "proto.frameAssemblyChecksumMismatch",
            Self::FragmentConflict => "proto.frameAssemblyFragmentConflict",
            Self::InconsistentMetadata => "proto.frameAssemblyInconsistentMetadata",
            Self::TimedOut => "proto.frameAssemblyTimedOut",
            Self::TooManyAssemblies => "proto.frameAssemblyTooManyAssemblies",
            Self::StagedBudgetExceeded => "proto.frameAssemblyStagedBudgetExceeded",
            Self::InvalidPayload => "proto.frameAssemblyInvalidPayload",
            Self::IncompleteFrame => "proto.frameAssemblyIncomplete",
        }
    }

    /// 确定性内容失败（重投必然再被拒，真源 `isDeterministicContentFault`）。
    /// 只有 InvalidPayload 算；InvalidJson 刻意不算——JSON 非法可能来自分片拼接边界。
    pub fn is_deterministic_content(self) -> bool {
        matches!(self, Self::InvalidPayload)
    }
}

/// 组装事件（真源 `TopicWireAssemblyEvent`）。
#[derive(Debug, Clone, PartialEq)]
pub enum AssemblyEvent {
    /// 一个逻辑帧重组完成。
    Complete(TopicFrame),
    /// 某 route 出错（不中断流，由上层走恢复阶梯）。
    Fault {
        topic: String,
        subscription_id: String,
        code: FaultCode,
    },
}

/// 投递种类（真源 `TopicFrameDeliveryKind`，wire.ts:8）。
/// `recovery` 是唯一能无 fault 取代在组装帧、也是唯一能解开 fail-closed 门的。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryKind {
    Initial,
    Online,
    Recovery,
}

impl DeliveryKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "initial" => Some(Self::Initial),
            "online" => Some(Self::Online),
            "recovery" => Some(Self::Recovery),
            _ => None,
        }
    }
}

/// 校验和（真源 wire.ts:6-13）。
#[derive(Debug, Clone, PartialEq)]
pub struct Checksum {
    pub value: String,
}

/// 一个在组装中的逻辑帧（真源 `FragmentAssembly`，wire-assembler.ts 内部结构）。
#[derive(Debug)]
struct Assembly {
    topic: String,
    subscription_id: String,
    logical_frame_id: String,
    ordinal: u32,
    delivery_kind: DeliveryKind,
    fragment_count: usize,
    logical_bytes: usize,
    checksum: String,
    /// 定长稀疏数组：按 fragmentIndex 存放（真源 `Array.from({length: fragmentCount})`）。
    fragments: Vec<Option<Vec<u8>>>,
    received_count: usize,
    decoded_bytes: usize,
    /// 首片到达时刻（**不滑动**，真源 firstSeenAt）。
    first_seen_ms: u64,
}

/// route 的墓碑：记录已结算的最大 ordinal（真源 `settledByRoute`）。
#[derive(Debug, Clone)]
struct Settled {
    ordinal: u32,
    logical_frame_id: String,
}

/// 增量重组器（真源 `TopicWireFrameAssembler`）。
#[derive(Debug, Default)]
pub struct WireAssembler {
    /// key = (topic, subscriptionId)。
    assemblies: HashMap<(String, String), Assembly>,
    settled: HashMap<(String, String), Settled>,
    staged_bytes: usize,
    /// 已 fault 的 route（fail-closed 门，真源 topicWireDecoder.ts:83-90）。
    faulted_routes: std::collections::HashSet<(String, String)>,
}

impl WireAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    fn fault(
        &mut self,
        topic: &str,
        subscription_id: &str,
        code: FaultCode,
    ) -> AssemblyEvent {
        self.faulted_routes
            .insert((topic.to_string(), subscription_id.to_string()));
        AssemblyEvent::Fault {
            topic: topic.to_string(),
            subscription_id: subscription_id.to_string(),
            code,
        }
    }

    /// 终止某 route 的组装（真源 `abort`：释放 + **留墓碑**，防旧 replay 复活）。
    pub fn abort(&mut self, topic: &str, subscription_id: &str) {
        if let Some(asm) = self
            .assemblies
            .remove(&(topic.to_string(), subscription_id.to_string()))
        {
            self.staged_bytes = self.staged_bytes.saturating_sub(asm.decoded_bytes);
            self.settled.insert(
                (topic.to_string(), subscription_id.to_string()),
                Settled {
                    ordinal: asm.ordinal,
                    logical_frame_id: asm.logical_frame_id,
                },
            );
        }
    }

    /// 丢弃某 route 的组装（真源 `discard`：释放 + **删墓碑**，订阅取消时用）。
    pub fn discard(&mut self, topic: &str, subscription_id: &str) {
        let key = (topic.to_string(), subscription_id.to_string());
        if let Some(asm) = self.assemblies.remove(&key) {
            self.staged_bytes = self.staged_bytes.saturating_sub(asm.decoded_bytes);
        }
        self.settled.remove(&key);
        self.faulted_routes.remove(&key);
    }

    /// 某 route 是否已被 fail-closed。
    pub fn is_faulted(&self, topic: &str, subscription_id: &str) -> bool {
        self.faulted_routes
            .contains(&(topic.to_string(), subscription_id.to_string()))
    }

    /// 下一个到期时刻（真源 `nextExpiryAt`：扫描全部 assembly 取最小）。
    pub fn next_expiry_ms(&self, now_ms: u64) -> Option<u64> {
        self.assemblies
            .values()
            .map(|a| a.first_seen_ms + TIMEOUT_MS)
            .filter(|t| *t > now_ms)
            .min()
    }

    /// 淘汰超时组装（真源 `expire()`：release + settle + fault）。
    /// `now_ms` 是注入的时钟——保留这个 seam，否则超时/墓碑逻辑无法单测。
    pub fn expire(&mut self, now_ms: u64) -> Vec<AssemblyEvent> {
        let expired: Vec<(String, String)> = self
            .assemblies
            .iter()
            .filter(|(_, a)| now_ms.saturating_sub(a.first_seen_ms) >= TIMEOUT_MS)
            .map(|(k, _)| k.clone())
            .collect();
        expired
            .into_iter()
            .map(|(topic, sub)| {
                let code = FaultCode::TimedOut;
                self.abort(&topic, &sub);
                self.fault(&topic, &sub, code)
            })
            .collect()
    }

    /// 送入一个物理帧，返回产生的事件（可能空）。
    pub fn accept(&mut self, wire: &WireFrame, now_ms: u64) -> Vec<AssemblyEvent> {
        let mut events = self.expire(now_ms);
        let key = (wire.topic.clone(), wire.subscription_id.clone());

        // fail-closed 门：一旦 fault，只有 recovery 能解（真源 topicWireDecoder.ts:83-90）。
        if self.faulted_routes.contains(&key) {
            let is_recovery = wire
                .delivery_kind
                .is_some_and(|k| k == DeliveryKind::Recovery);
            if !is_recovery {
                return events;
            }
            self.faulted_routes.remove(&key);
            self.abort(&wire.topic, &wire.subscription_id);
        }

        // Step 1：ordinal 基础合法性。
        if wire.logical_frame_ordinal == 0 {
            let code = FaultCode::MetadataMismatch;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        }
        let delivery_kind = match wire.delivery_kind {
            Some(k) => k,
            None => {
                let code = FaultCode::MetadataMismatch;
                events.push(self.fault(&wire.topic, &wire.subscription_id, code));
                return events;
            }
        };

        // Step 2/4：墓碑与在组装帧的 ordinal 关系。
        if let Some(settled) = self.settled.get(&key) {
            if wire.logical_frame_ordinal < settled.ordinal {
                return events; // 静默丢弃迟到片
            }
            if wire.logical_frame_ordinal == settled.ordinal {
                if wire.logical_frame_id != settled.logical_frame_id {
                    let code = FaultCode::OrdinalConflict;
                    events.push(self.fault(&wire.topic, &wire.subscription_id, code));
                }
                return events; // 重复片，静默丢弃
            }
        }
        if let Some(current) = self.assemblies.get(&key) {
            if wire.logical_frame_ordinal < current.ordinal {
                return events;
            }
            if wire.logical_frame_ordinal == current.ordinal {
                //同 ordinal 且 id 不同 → 真帧冲突，fault。
                if wire.logical_frame_id != current.logical_frame_id {
                    let code = FaultCode::OrdinalConflict;
                    events.push(self.fault(&wire.topic, &wire.subscription_id, code));
                    return events;
                }
                // 同 ordinal 同 id → **同一逻辑帧的后续片，放行**（不是冲突）。
                // 这正是 fragment 分片能逐片累积的前提。
            } else if delivery_kind != DeliveryKind::Recovery {
                // ordinal 更大 → 旧的在组装帧被取代。**只有 recovery 能无 fault 取代**。
                let code = FaultCode::Superseded;
                events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            }
        }

        match wire.kind {
            WireFrameKind::Complete => {
                // complete 帧不该有在组装中的同 route（真源 :272-278）。
                if self.assemblies.contains_key(&key) {
                    let code = FaultCode::OrdinalConflict;
                    events.push(self.fault(&wire.topic, &wire.subscription_id, code));
                    return events;
                }
                match wire.logical() {
                    Some(frame) => {
                        // 写墓碑，后续更小 ordinal 的片会被静默丢弃。
                        self.settled.insert(
                            key,
                            Settled {
                                ordinal: wire.logical_frame_ordinal,
                                logical_frame_id: wire.logical_frame_id.clone(),
                            },
                        );
                        events.push(AssemblyEvent::Complete(frame.clone()));
                    }
                    None => {
                        let code = FaultCode::MetadataMismatch;
                        events.push(self.fault(&wire.topic, &wire.subscription_id, code));
                    }
                }
                events
            }
            WireFrameKind::Fragment => {
                events.extend(self.accept_fragment(wire, delivery_kind, now_ms));
                events
            }
        }
    }

    fn accept_fragment(
        &mut self,
        wire: &WireFrame,
        delivery_kind: DeliveryKind,
        now_ms: u64,
    ) -> Vec<AssemblyEvent> {
        let mut events = Vec::new();
        let key = (wire.topic.clone(), wire.subscription_id.clone());

        // Step 7：fragment 内层字段校验。
        let (Some(index), Some(count)) = (wire.fragment_index, wire.fragment_count) else {
            let code = FaultCode::MetadataMismatch;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        };
        if count == 0 || count as usize > MAX_FRAGMENTS || index >= count {
            let code = FaultCode::MetadataMismatch;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        }
        let Some(logical_bytes) = wire.logical_bytes else {
            let code = FaultCode::MetadataMismatch;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        };
        if logical_bytes > MAX_LOGICAL_BYTES {
            let code = FaultCode::MetadataMismatch;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        }
        let Some(checksum) = wire.checksum_value.as_deref() else {
            let code = FaultCode::MetadataMismatch;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        };
        let Some(data_b64) = wire.data_base64.as_deref() else {
            let code = FaultCode::MetadataMismatch;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        };
        // fragmentCount 不得大于 logicalBytes（真源 :330）。
        if count as usize > logical_bytes {
            let code = FaultCode::MetadataMismatch;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        }

        // Step 8：base64 解码（真源严格标准字母表，见 decode_base64）。
        let Some(bytes) = decode_base64(data_b64) else {
            let code = FaultCode::InvalidBase64;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        };

        // Step 9：建组或校验一致性。
        if !self.assemblies.contains_key(&key) {
            if self.assemblies.len() >= MAX_CONCURRENT {
                let code = FaultCode::TooManyAssemblies;
                events.push(self.fault(&wire.topic, &wire.subscription_id, code));
                return events;
            }
            self.assemblies.insert(
                key.clone(),
                Assembly {
                    topic: wire.topic.clone(),
                    subscription_id: wire.subscription_id.clone(),
                    logical_frame_id: wire.logical_frame_id.clone(),
                    ordinal: wire.logical_frame_ordinal,
                    delivery_kind,
                    fragment_count: count as usize,
                    logical_bytes,
                    checksum: checksum.to_string(),
                    fragments: vec![None; count as usize],
                    received_count: 0,
                    decoded_bytes: 0,
                    first_seen_ms: now_ms,
                },
            );
        }
        let Some(asm) = self.assemblies.get_mut(&key) else {
            return events;
        };
        // 已有组：五项元数据必须全一致（真源 :348-354，logicalFrameId 不在内）。
        if asm.fragment_count != count as usize
            || asm.delivery_kind != delivery_kind
            || asm.logical_bytes != logical_bytes
            || asm.checksum != checksum
        {
            let code = FaultCode::InconsistentMetadata;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        }

        // Step 10：重复片（同字节静默、异字节 fault）。
        if let Some(existing) = &asm.fragments[index as usize] {
            if existing == &bytes {
                return events;
            }
            let code = FaultCode::FragmentConflict;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        }

        // Step 11：预算与组内长度累加。
        let len = bytes.len();
        if self.staged_bytes + len > MAX_STAGED_BYTES {
            let code = FaultCode::StagedBudgetExceeded;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        }
        if asm.decoded_bytes + len > asm.logical_bytes {
            let code = FaultCode::LengthMismatch;
            events.push(self.fault(&wire.topic, &wire.subscription_id, code));
            return events;
        }
        asm.fragments[index as usize] = Some(bytes);
        asm.received_count += 1;
        asm.decoded_bytes += len;
        self.staged_bytes += len;

        // Step 12：收齐判定（唯一信号）。
        let asm = self.assemblies.get(&key).expect("刚插入的组应存在");
        if asm.received_count != asm.fragment_count {
            return events;
        }

        // Step 13：release（槽位立刻释放，预算回落）→ 长度 → 拼接 → CRC → UTF-8 → JSON。
        let Some(asm) = self.assemblies.remove(&key) else {
            return events;
        };
        self.staged_bytes = self.staged_bytes.saturating_sub(asm.decoded_bytes);
        self.settled.insert(
            key.clone(),
            Settled {
                ordinal: asm.ordinal,
                logical_frame_id: asm.logical_frame_id.clone(),
            },
        );

        // 收齐后精确长度相等（预检之外的第二次）。
        if asm.decoded_bytes != asm.logical_bytes {
            let code = FaultCode::LengthMismatch;
            events.push(self.fault(&asm.topic, &asm.subscription_id, code));
            return events;
        }
        // 拼接：定长数组按 fragmentIndex 升序，天然与到达顺序无关。
        let mut logical = Vec::with_capacity(asm.decoded_bytes);
        for slot in &asm.fragments {
            match slot {
                Some(bytes) => logical.extend_from_slice(bytes),
                None => {
                    // 防御性检查：有洞（理论上收齐判定已排除）。
                    let code = FaultCode::LengthMismatch;
                    events.push(self.fault(&asm.topic, &asm.subscription_id, code));
                    return events;
                }
            }
        }
        // CRC32 在长度之后（省 CPU，顺序不可换）。
        if crc32_hex(&logical) != asm.checksum {
            let code = FaultCode::ChecksumMismatch;
            events.push(self.fault(&asm.topic, &asm.subscription_id, code));
            return events;
        }
        // UTF-8 必须严格（不能用 from_utf8_lossy，否则坏帧会带 U+FFFD 混过去）。
        let Ok(text) = String::from_utf8(logical) else {
            let code = FaultCode::InvalidUtf8;
            events.push(self.fault(&asm.topic, &asm.subscription_id, code));
            return events;
        };
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            let code = FaultCode::InvalidJson;
            events.push(self.fault(&asm.topic, &asm.subscription_id, code));
            return events;
        };
        // envelope 交叉校验：逻辑帧的 topic/subscriptionId 必须与外层一致。
        let logical_frame: TopicFrame = match serde_json::from_value(value) {
            Ok(f) => f,
            Err(_) => {
                let code = FaultCode::InvalidPayload;
                events.push(self.fault(&asm.topic, &asm.subscription_id, code));
                return events;
            }
        };
        if logical_frame.topic != asm.topic || logical_frame.subscription_id != asm.subscription_id
        {
            let code = FaultCode::InvalidPayload;
            events.push(self.fault(&asm.topic, &asm.subscription_id, code));
            return events;
        }
        events.push(AssemblyEvent::Complete(logical_frame));
        events
    }
}

/// CRC-32/IEEE，输出 8 位小写 hex（真源 `wire-binary.ts:10-19`）。
pub fn crc32_hex(bytes: &[u8]) -> String {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(bytes);
    format!("{:08x}", hasher.finalize())
}

/// 严格标准 base64 解码（真源 `wire-binary.ts:46-65` + `topicWireBase64Schema`）。
///
/// 真源正则要求：长度 ≥4、是 4 的倍数、只标准字母表（**无 URL-safe**）、
/// padding 只在末尾。Rust 的 STANDARD engine 容忍非规范 padding，
/// 故先做长度与字符集预检，让 fault 分类与真源一致。
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    if input.len() < 4 || input.len() % 4 != 0 {
        return None;
    }
    // 只允许标准字母表与末尾 padding。
    let body = input.trim_end_matches('=');
    if !body
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
    {
        return None;
    }
    // padding 只能在末尾，且至多两个。
    let pad = input.len() - body.len();
    if pad > 2 || input[body.len()..].chars().any(|c| c != '=') {
        return None;
    }
    STANDARD.decode(input).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn b64(data: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(data)
    }

    fn logical_frame_json(seq: f64) -> String {
        serde_json::json!({
            "topic": "conversation/s1",
            "subscriptionId": "sub-1",
            "fromSeq": seq,
            "toSeq": seq + 1.0,
            "payload": { "kind": "deltas", "deltas": [] }
        })
        .to_string()
    }

    fn fragment(
        index: u32,
        count: u32,
        ordinal: u32,
        data: &[u8],
        logical_bytes: usize,
        checksum: &str,
    ) -> WireFrame {
        WireFrame {
            wire_version: 3,
            kind: WireFrameKind::Fragment,
            topic: "conversation/s1".into(),
            subscription_id: "sub-1".into(),
            logical_frame_id: "lf-1".into(),
            logical_frame_ordinal: ordinal,
            delivery_kind: Some(DeliveryKind::Initial),
            frame: None,
            fragment_index: Some(index),
            fragment_count: Some(count),
            logical_bytes: Some(logical_bytes),
            checksum_value: Some(checksum.to_string()),
            data_base64: Some(b64(data)),
        }
    }

    #[test]
    fn crc32_matches_known_vector() {
        // 标准 CRC-32/IEEE："123456789" → 0xCBF43926。
        assert_eq!(crc32_hex(b"123456789"), "cbf43926");
        assert_eq!(crc32_hex(b""), "00000000", "空输入应是 0");
    }

    #[test]
    fn crc32_hex_is_lowercase_8_chars() {
        let hex = crc32_hex(b"hello");
        assert_eq!(hex.len(), 8);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }

    #[test]
    fn base64_rejects_non_canonical_input() {
        // 真源正则要求长度 ≥4 且 4 的倍数。
        assert!(decode_base64("aGk").is_none(), "长度不足 4");
        // URL-safe 字母表被拒（无 -_）。
        assert!(decode_base64("a-b_").is_none(), "URL-safe 应被拒");
        // 中间 padding 非法。
        assert!(decode_base64("ab=cd").is_none(), "padding 只允许末尾");
        // 正常输入可解。
        assert_eq!(decode_base64("aGk=").unwrap(), b"hi");
        assert_eq!(decode_base64("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(decode_base64("").unwrap_or_default(), b"");
    }

    #[test]
    fn single_fragment_completes() {
        let payload = logical_frame_json(0.0);
        let checksum = crc32_hex(payload.as_bytes());
        let mut asm = WireAssembler::new();
        let events = asm.accept(&fragment(0, 1, 1, payload.as_bytes(), payload.len(), &checksum), 0);
        assert_eq!(events.len(), 1, "应产出一个事件");
        match &events[0] {
            AssemblyEvent::Complete(f) => {
                assert_eq!(f.topic, "conversation/s1");
                assert_eq!(f.from_seq, 0.0);
            }
            other => panic!("期望 Complete，实际 {other:?}"),
        }
    }

    #[test]
    fn multi_fragment_joins_in_index_order() {
        // 故意乱序送达，验证拼接按 fragmentIndex 而非到达顺序。
        let payload = logical_frame_json(5.0);
        let bytes = payload.as_bytes();
        let checksum = crc32_hex(bytes);
        let third = bytes.len() / 3;
        let f0 = &bytes[..third];
        let f1 = &bytes[third..2 * third];
        let f2 = &bytes[2 * third..];

        let mut asm = WireAssembler::new();
        // 先送 2，再 0，再 1。
        for idx in [2u32, 0, 1] {
            let piece = match idx {
                0 => f0,
                1 => f1,
                _ => f2,
            };
            let ev =
                asm.accept(&fragment(idx, 3, 1, piece, bytes.len(), &checksum), 0);
            if idx != 1 {
                assert!(ev.is_empty(), "未收齐时不该产出事件（idx={idx}）");
            } else {
                match ev.first() {
                    Some(AssemblyEvent::Complete(f)) => assert_eq!(f.from_seq, 5.0),
                    other => panic!("期望 Complete，实际 {other:?}"),
                }
            }
        }
    }

    #[test]
    fn checksum_mismatch_is_faulted() {
        let payload = logical_frame_json(0.0);
        let mut asm = WireAssembler::new();
        let ev = asm.accept(
            &fragment(0, 1, 1, payload.as_bytes(), payload.len(), "deadbeef"),
            0,
        );
        match ev.first() {
            Some(AssemblyEvent::Fault { code, .. }) => {
                assert_eq!(*code, FaultCode::ChecksumMismatch)
            }
            other => panic!("期望 ChecksumMismatch，实际 {other:?}"),
        }
        assert!(asm.is_faulted("conversation/s1", "sub-1"));
    }

    #[test]
    fn length_mismatch_is_caught_before_crc() {
        // logical_bytes 声明得比实际大 → 预检就拦住，不会走到 CRC。
        let payload = b"{}";
        let mut asm = WireAssembler::new();
        let ev = asm.accept(
            &fragment(0, 1, 1, payload, 9999, &crc32_hex(payload)),
            0,
        );
        match ev.first() {
            Some(AssemblyEvent::Fault { code, .. }) => assert_eq!(*code, FaultCode::LengthMismatch),
            other => panic!("期望 LengthMismatch，实际 {other:?}"),
        }
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        // 0xFF 不是合法 UTF-8 起始字节。
        let payload = [0xffu8, 0xfe, 0xfd];
        let checksum = crc32_hex(&payload);
        let mut asm = WireAssembler::new();
        let ev = asm.accept(&fragment(0, 1, 1, &payload, 3, &checksum), 0);
        match ev.first() {
            Some(AssemblyEvent::Fault { code, .. }) => assert_eq!(*code, FaultCode::InvalidUtf8),
            other => panic!("期望 InvalidUtf8，实际 {other:?}"),
        }
    }

    #[test]
    fn valid_multibyte_utf8_survives_fragment_boundary() {
        // ★核心场景：分片边界切断 3 字节中文字符，逐片解码会乱码，
        // 必须拼接后一次性解码才正确。
        // 只把 payload 的 deltas 里放中文，topic/subId 保持与外层一致
        // （否则会触发 envelope 交叉校验失败，测不到分片边界这个点）。
        let payload = serde_json::json!({
            "topic": "conversation/s1",
            "subscriptionId": "sub-1",
            "fromSeq": 0.0,
            "toSeq": 1.0,
            "payload": { "kind": "deltas", "deltas": [
                { "op": "row.delta", "rowId": 1, "path": "text", "append": "中文会话内容" }
            ] }
        })
        .to_string();
        let bytes = payload.as_bytes();
        let checksum = crc32_hex(bytes);
        // 按任意字节数切（中文 UTF-8 是 3 字节，必然被切断）。
        let mid = bytes.len() / 2;
        let (a, b) = bytes.split_at(mid);
        let mut asm = WireAssembler::new();
        assert!(asm
            .accept(&fragment(0, 2, 1, a, bytes.len(), &checksum), 0)
            .is_empty());
        let ev = asm.accept(&fragment(1, 2, 1, b, bytes.len(), &checksum), 0);
        match ev.first() {
            Some(AssemblyEvent::Complete(f)) => {
                // 分片边界切断 3 字节中文字符，拼接后一次性解码才正确。
                let text = serde_json::to_string(&f.payload).unwrap();
                assert!(
                    text.contains("中文会话内容"),
                    "中文应完整保留（重组后一次解码）：{text}"
                );
            }
            other => panic!("期望 Complete，实际 {other:?}"),
        }
    }

    #[test]
    fn invalid_json_is_faulted() {
        let payload = b"{not json";
        let checksum = crc32_hex(payload);
        let mut asm = WireAssembler::new();
        let ev = asm.accept(&fragment(0, 1, 1, payload, payload.len(), &checksum), 0);
        match ev.first() {
            Some(AssemblyEvent::Fault { code, .. }) => assert_eq!(*code, FaultCode::InvalidJson),
            other => panic!("期望 InvalidJson，实际 {other:?}"),
        }
    }

    #[test]
    fn envelope_topic_mismatch_is_invalid_payload() {
        // 逻辑帧 topic 与外层不一致 → InvalidPayload（确定性内容失败）。
        let payload = serde_json::json!({
            "topic": "conversation/OTHER",
            "subscriptionId": "sub-1",
            "fromSeq": 0.0,
            "toSeq": 1.0,
            "payload": { "kind": "deltas", "deltas": [] }
        })
        .to_string();
        let checksum = crc32_hex(payload.as_bytes());
        let mut asm = WireAssembler::new();
        let ev = asm.accept(
            &fragment(0, 1, 1, payload.as_bytes(), payload.len(), &checksum),
            0,
        );
        match ev.first() {
            Some(AssemblyEvent::Fault { code, .. }) => {
                assert_eq!(*code, FaultCode::InvalidPayload);
                assert!(code.is_deterministic_content(), "InvalidPayload 是确定性内容失败");
            }
            other => panic!("期望 InvalidPayload，实际 {other:?}"),
        }
    }

    #[test]
    fn duplicate_fragment_same_bytes_is_silent() {
        let payload = logical_frame_json(0.0);
        let bytes = payload.as_bytes();
        let checksum = crc32_hex(bytes);
        let mut asm = WireAssembler::new();
        let f = fragment(0, 2, 1, bytes, bytes.len() + 10, &checksum);
        assert!(asm.accept(&f, 0).is_empty());
        // 同字节重复片：静默，不算重复计数。
        assert!(asm.accept(&f, 0).is_empty());
        // 组仍在组装中（received_count=1 ≠ 2）。
        assert_eq!(asm.next_expiry_ms(0), Some(TIMEOUT_MS));
    }

    #[test]
    fn duplicate_fragment_conflicting_bytes_is_faulted() {
        let checksum = crc32_hex(b"x");
        let mut asm = WireAssembler::new();
        assert!(asm
            .accept(&fragment(0, 2, 1, b"aaa", 100, &checksum), 0)
            .is_empty());
        let ev = asm.accept(&fragment(0, 2, 1, b"bbb", 100, &checksum), 0);
        match ev.first() {
            Some(AssemblyEvent::Fault { code, .. }) => assert_eq!(*code, FaultCode::FragmentConflict),
            other => panic!("期望 FragmentConflict，实际 {other:?}"),
        }
    }

    #[test]
    fn late_fragment_after_settle_is_silently_dropped() {
        let payload = logical_frame_json(0.0);
        let bytes = payload.as_bytes();
        let checksum = crc32_hex(bytes);
        let mut asm = WireAssembler::new();
        // 完成 ordinal=2 的帧。
        asm.accept(&fragment(0, 1, 2, bytes, bytes.len(), &checksum), 0);
        // 再来ordinal=1 的旧片 → 静默丢弃（无 fault）。
        let ev = asm.accept(&fragment(0, 1, 1, bytes, bytes.len(), &checksum), 1);
        assert!(ev.is_empty(), "迟到旧片应静默丢弃，实际 {:?}", ev);
    }

    #[test]
    fn inconsistent_metadata_across_fragments_is_faulted() {
        let checksum = crc32_hex(b"aa");
        let mut asm = WireAssembler::new();
        assert!(asm
            .accept(&fragment(0, 3, 1, b"aa", 100, &checksum), 0)
            .is_empty());
        // 同组但 fragmentCount 不一致。
        let mut f = fragment(1, 4, 1, b"bb", 100, &checksum);
        f.fragment_count = Some(4);
        let ev = asm.accept(&f, 0);
        match ev.first() {
            Some(AssemblyEvent::Fault { code, .. }) => {
                assert_eq!(*code, FaultCode::InconsistentMetadata)
            }
            other => panic!("期望 InconsistentMetadata，实际 {other:?}"),
        }
    }

    #[test]
    fn expiry_faults_and_frees_route() {
        let payload = logical_frame_json(0.0);
        let bytes = payload.as_bytes();
        let checksum = crc32_hex(bytes);
        let mut asm = WireAssembler::new();
        asm.accept(&fragment(0, 2, 1, bytes, bytes.len() + 10, &checksum), 1_000);
        assert_eq!(asm.next_expiry_ms(1_000), Some(1_000 + TIMEOUT_MS));

        // 未到时间不淘汰。
        assert!(asm.expire(1_000 + TIMEOUT_MS - 1).is_empty());
        // 到时间淘汰并 fault。
        let ev = asm.expire(1_000 + TIMEOUT_MS);
        match ev.first() {
            Some(AssemblyEvent::Fault { code, .. }) => assert_eq!(*code, FaultCode::TimedOut),
            other => panic!("期望 TimedOut，实际 {other:?}"),
        }
        assert_eq!(asm.next_expiry_ms(0), None, "淘汰后不应残留组装");
    }

    #[test]
    fn faulted_route_drops_until_recovery() {
        let payload = logical_frame_json(0.0);
        let bytes = payload.as_bytes();
        let mut asm = WireAssembler::new();
        // 制造 fault。
        asm.accept(&fragment(0, 1, 1, bytes, bytes.len(), "deadbeef"), 0);
        assert!(asm.is_faulted("conversation/s1", "sub-1"));

        // 普通片被门挡住。
        let good = fragment(0, 1, 5, bytes, bytes.len(), &crc32_hex(bytes));
        assert!(asm.accept(&good, 0).is_empty());

        // recovery 解门并放行。
        let mut recovery = good.clone();
        recovery.delivery_kind = Some(DeliveryKind::Recovery);
        recovery.logical_frame_ordinal = 6;
        let ev = asm.accept(&recovery, 0);
        assert!(
            matches!(ev.first(), Some(AssemblyEvent::Complete(_))),
            "recovery 应解开fail-closed 门，实际 {ev:?}"
        );
    }

    #[test]
    fn supersede_requires_recovery_to_avoid_fault() {
        let a = b"aaaa";
        let checksum = crc32_hex(a);
        let mut asm = WireAssembler::new();
        // ordinal=1 在组装中（2 片只送 1 片）。
        assert!(asm
            .accept(&fragment(0, 2, 1, a, 100, &checksum), 0)
            .is_empty());
        // ordinal=2 的首片 → 取代旧的，非 recovery 应 fault Superseded。
        let ev = asm.accept(&fragment(0, 2, 2, a, 100, &checksum), 0);
        assert!(
            ev.iter().any(|e| matches!(e, AssemblyEvent::Fault { code, .. }
                if *code == FaultCode::Superseded)),
            "非 recovery 取代应 fault Superseded，实际 {ev:?}"
        );
    }

    #[test]
    fn discard_clears_tombstone_so_replay_can_work() {
        let payload = logical_frame_json(0.0);
        let bytes = payload.as_bytes();
        let checksum = crc32_hex(bytes);
        let mut asm = WireAssembler::new();
        asm.accept(&fragment(0, 1, 5, bytes, bytes.len(), &checksum), 0);
        // discard 后同一 ordinal 可以重来（墓碑被清）。
        asm.discard("conversation/s1", "sub-1");
        let ev = asm.accept(&fragment(0, 1, 5, bytes, bytes.len(), &checksum), 0);
        assert!(matches!(ev.first(), Some(AssemblyEvent::Complete(_))));
    }

    #[test]
    fn abort_keeps_tombstone_blocking_replay() {
        let payload = logical_frame_json(0.0);
        let bytes = payload.as_bytes();
        let checksum = crc32_hex(bytes);
        let mut asm = WireAssembler::new();
        asm.accept(&fragment(0, 2, 5, bytes, 100, &checksum), 0);
        asm.abort("conversation/s1", "sub-1");
        // 墓碑还在：同 ordinal 重来被静默丢弃（防旧 replay 复活）。
        let ev = asm.accept(&fragment(0, 2, 5, bytes, 100, &checksum), 0);
        assert!(ev.is_empty());
    }

    #[test]
    fn complete_frame_settles_and_blocks_lower_ordinal_fragments() {
        let payload = logical_frame_json(9.0);
        let bytes = payload.as_bytes();
        let mut asm = WireAssembler::new();
        let complete = WireFrame {
            wire_version: 3,
            kind: WireFrameKind::Complete,
            topic: "conversation/s1".into(),
            subscription_id: "sub-1".into(),
            logical_frame_id: "lf-9".into(),
            logical_frame_ordinal: 9,
            delivery_kind: Some(DeliveryKind::Online),
            frame: Some(serde_json::from_str(&payload).unwrap()),
            fragment_index: None,
            fragment_count: None,
            logical_bytes: None,
            checksum_value: None,
            data_base64: None,
        };
        assert!(matches!(
            asm.accept(&complete, 0).first(),
            Some(AssemblyEvent::Complete(_))
        ));
        // ordinal=3 的 fragment 应被墓碑静默丢弃。
        let ev = asm.accept(&fragment(0, 1, 3, bytes, bytes.len(), &crc32_hex(bytes)), 0);
        assert!(ev.is_empty());
    }

    #[test]
    fn fault_reason_codes_match_source_strings() {
        // reasonCode 是排障时对照日志的依据，值不能错。
        assert_eq!(
            FaultCode::ChecksumMismatch.reason_code(),
            "proto.frameAssemblyChecksumMismatch"
        );
        assert_eq!(
            FaultCode::TimedOut.reason_code(),
            "proto.frameAssemblyTimedOut"
        );
        // 只有 InvalidPayload 是确定性内容失败。
        assert!(FaultCode::InvalidPayload.is_deterministic_content());
        assert!(!FaultCode::InvalidJson.is_deterministic_content());
        assert!(!FaultCode::ChecksumMismatch.is_deterministic_content());
    }

    #[test]
    fn delivery_kind_parses_strictly() {
        assert_eq!(DeliveryKind::parse("initial"), Some(DeliveryKind::Initial));
        assert_eq!(DeliveryKind::parse("online"), Some(DeliveryKind::Online));
        assert_eq!(DeliveryKind::parse("recovery"), Some(DeliveryKind::Recovery));
        assert_eq!(DeliveryKind::parse("bogus"), None);
    }
}