//! 1:1 翻译 `packages/ui/src/lib/toolIdentity.ts`（343 行）
//! + `packages/shared/src/tool-identity.ts`（125 行，工具名表随迁）。
//!
//! 工具身份解析：从 toolName / kind / title / raw 多路候选里解析出
//! 「已知工具名 + 展示 family + 来源 + 是否 legacy」，供 display model /
//! file summary / 判定层使用。
//!
//! **与 resolveRenderer.rs 的关系**：`resolveRenderer.rs` 已有 `ToolFamily`
//! 枚举（超集：含 explore/switch-mode/plan-guidance/unknown）与
//! `family_by_lower` 静态表——本文件复用该枚举，并补齐 shared 表所需的
//! **规范拼写归一**（`normalize_zcode_tool_name`，返回 "Read"/"web_search"
//! 这类规范形式的原值）与完整 identity 链（resolveRenderer 的
//! `normalize_tool_token` 是更狠的归一，不能替代本文件的表查询）。
//!
//! **顺序敏感**（真源注释反复强调）：legacy payload 判定在 title 之前——
//! `Task` 是现役工具名，但历史投影用 kind="think" + title="Task" 表示
//! 子 agent 活动；先看 title 会把旧会话误升级成非 legacy 身份。

use std::sync::OnceLock;

use regex_lite::Regex;
use serde_json::Value;

use super::renderers::ask_question::{
    normalize_ask_user_question_input, read_ask_user_question_input,
};
use super::renderers::todo::is_todo_plan_tool_name;
use super::resolveRenderer::ToolFamily;

// ---------------------------------------------------------------------------
// packages/shared/src/tool-identity.ts（工具名表）
// ---------------------------------------------------------------------------

/// `ZCODE_KNOWN_TOOL_NAMES`（shared :1-33，31 项）。
pub const ZCODE_KNOWN_TOOL_NAMES: [&str; 31] = [
    "Read",
    "Write",
    "Edit",
    "ApplyPatch",
    "Bash",
    "Glob",
    "Grep",
    "WebFetch",
    "WebSearch",
    "web_search",
    "TodoRead",
    "TodoWrite",
    "GoalRead",
    "ReadSessionContext",
    "AskUserQuestion",
    "SendMessage",
    "RespondToCoordinator",
    "TaskOutput",
    "TaskStop",
    "js",
    "js_reset",
    "js_add_node_module_dir",
    "mcp__node_repl__js",
    "mcp__node_repl__js_reset",
    "mcp__node_repl__js_add_node_module_dir",
    "Agent",
    "Task",
    "Skill",
    "CreateWorkflow",
    // 修订入口：登记进 workflow family 让确认窗按 family 选中运行确认块。
    "AmendWorkflow",
    // wire 名就是 snake_case 的 submit_result（仓库里唯一一个）。
    "submit_result",
];

/// `normalizeZCodeToolName`（shared :98-107）：大小写不敏感查表，
/// 返回**规范拼写**（`"read"` → `"Read"`、`"web_search"` 原样保留）。
pub fn normalize_zcode_tool_name(value: Option<&str>) -> Option<&'static str> {
    let normalized = value?.trim();
    if normalized.is_empty() {
        return None;
    }
    match normalized.to_lowercase().as_str() {
        "read" => Some("Read"),
        "write" => Some("Write"),
        "edit" => Some("Edit"),
        "applypatch" => Some("ApplyPatch"),
        "bash" => Some("Bash"),
        "glob" => Some("Glob"),
        "grep" => Some("Grep"),
        "webfetch" => Some("WebFetch"),
        "websearch" => Some("WebSearch"),
        "web_search" => Some("web_search"),
        "todoread" => Some("TodoRead"),
        "todowrite" => Some("TodoWrite"),
        "goalread" => Some("GoalRead"),
        "readsessioncontext" => Some("ReadSessionContext"),
        "askuserquestion" => Some("AskUserQuestion"),
        "sendmessage" => Some("SendMessage"),
        "respondtocoordinator" => Some("RespondToCoordinator"),
        "taskoutput" => Some("TaskOutput"),
        "taskstop" => Some("TaskStop"),
        "js" => Some("js"),
        "js_reset" => Some("js_reset"),
        "js_add_node_module_dir" => Some("js_add_node_module_dir"),
        "mcp__node_repl__js" => Some("mcp__node_repl__js"),
        "mcp__node_repl__js_reset" => Some("mcp__node_repl__js_reset"),
        "mcp__node_repl__js_add_node_module_dir" => Some("mcp__node_repl__js_add_node_module_dir"),
        "agent" => Some("Agent"),
        "task" => Some("Task"),
        "skill" => Some("Skill"),
        "createworkflow" => Some("CreateWorkflow"),
        "amendworkflow" => Some("AmendWorkflow"),
        "submit_result" => Some("submit_result"),
        _ => None,
    }
}

/// `getZCodeToolFamilyForName`（shared :109-114）。
pub fn get_zcode_tool_family_for_name(value: Option<&str>) -> Option<ToolFamily> {
    let tool_name = normalize_zcode_tool_name(value)?;
    Some(match tool_name {
        "Read" => ToolFamily::FileRead,
        "Write" | "Edit" | "ApplyPatch" => ToolFamily::FileWrite,
        "Bash" => ToolFamily::Shell,
        "Glob" | "Grep" | "WebFetch" | "WebSearch" | "web_search" => ToolFamily::Search,
        "TodoRead" | "TodoWrite" => ToolFamily::Todo,
        "GoalRead" => ToolFamily::Goal,
        "ReadSessionContext" => ToolFamily::SessionContext,
        "AskUserQuestion" => ToolFamily::AskUserQuestion,
        "SendMessage" | "RespondToCoordinator" => ToolFamily::Message,
        "TaskOutput" | "TaskStop" => ToolFamily::TaskControl,
        "js"
        | "js_reset"
        | "js_add_node_module_dir"
        | "mcp__node_repl__js"
        | "mcp__node_repl__js_reset"
        | "mcp__node_repl__js_add_node_module_dir" => ToolFamily::NodeRepl,
        "Agent" | "Task" => ToolFamily::Agent,
        "Skill" => ToolFamily::Skill,
        "CreateWorkflow" | "AmendWorkflow" | "submit_result" => ToolFamily::Workflow,
        _ => return None,
    })
}

/// `isZCodeFileContentWriteToolName`（shared :123-125）：只有 `Write` 是。
pub fn is_zcode_file_content_write_tool_name(value: Option<&str>) -> bool {
    normalize_zcode_tool_name(value) == Some("Write")
}

// ---------------------------------------------------------------------------
// packages/ui/src/lib/toolIdentity.ts
// ---------------------------------------------------------------------------

/// `ToolCallIdentitySource`（真源 :21-30）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolIdentitySource {
    ToolName,
    Kind,
    Title,
    Raw,
    RawZcodeMeta,
    LegacyKind,
    LegacyTitle,
    LegacyPayload,
    Unknown,
}

/// `ToolCallIdentity`（真源 :32-37）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallIdentity {
    pub tool_name: Option<String>,
    pub family: ToolFamily,
    pub source: ToolIdentitySource,
    pub is_legacy: bool,
}

/// `ToolIdentityLike`（真源 :39-45）：判定与显示消费的身份来源子集。
#[derive(Debug, Clone)]
pub struct ToolIdentityLike<'a> {
    pub tool_name: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub title: Option<&'a str>,
    pub input: &'a Value,
    pub raw: &'a Value,
}

/// `UNKNOWN_TOOL_IDENTITY`（真源 :47-52）。
fn unknown_tool_identity() -> ToolCallIdentity {
    ToolCallIdentity {
        tool_name: None,
        family: ToolFamily::Unknown,
        source: ToolIdentitySource::Unknown,
        is_legacy: false,
    }
}

fn is_record(value: &Value) -> bool {
    value.is_object()
}

/// `readString`（真源 :58-60）：非空白字符串，返回 **trim 后**值。
fn read_string(value: &Value) -> Option<String> {
    let s = value.as_str()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `readNestedString`（真源 :62-71）：逐级下钻，每级须是 record。
fn read_nested_string(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        if !is_record(current) {
            return None;
        }
        current = current.get(*key)?;
    }
    read_string(current)
}

/// `normalizeLegacyToken`（真源 :73-79）：trim + 小写 + 空白/连字符压成 `_`。
fn normalize_legacy_token(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let mut out = String::new();
    let mut in_separator = false;
    for c in value.trim().to_lowercase().chars() {
        if c.is_whitespace() || c == '-' {
            if !in_separator {
                out.push('_');
                in_separator = true;
            }
        } else {
            out.push(c);
            in_separator = false;
        }
    }
    out
}

/// `identityFromKnownToolName`（真源 :81-98）。
fn identity_from_known_tool_name(
    value: Option<&str>,
    source: ToolIdentitySource,
) -> Option<ToolCallIdentity> {
    let tool_name = normalize_zcode_tool_name(value)?;
    let family = get_zcode_tool_family_for_name(Some(tool_name))?;
    Some(ToolCallIdentity {
        tool_name: Some(tool_name.to_string()),
        family,
        source,
        is_legacy: false,
    })
}

/// `identityFromLegacyFamily`（真源 :100-110）。
fn identity_from_legacy_family(
    tool_name: Option<String>,
    family: ToolFamily,
    source: ToolIdentitySource,
) -> ToolCallIdentity {
    ToolCallIdentity {
        tool_name,
        family,
        source,
        is_legacy: true,
    }
}

/// `readRawToolNameCandidates` 的返回（真源 :112-121）。
struct RawToolNameCandidates {
    direct: Option<String>,
    zcode: Option<String>,
    raw_kind: Option<String>,
    raw_title: Option<String>,
}

fn read_raw_tool_name_candidates(raw: &Value) -> RawToolNameCandidates {
    RawToolNameCandidates {
        direct: read_nested_string(raw, &["toolName"])
            .or_else(|| read_nested_string(raw, &["tool_name"]))
            .or_else(|| read_nested_string(raw, &["name"])),
        zcode: read_nested_string(raw, &["_meta", "zcode", "toolName"]),
        raw_kind: read_nested_string(raw, &["kind"]),
        raw_title: read_nested_string(raw, &["title"]),
    }
}

/// `isLegacyAgentTool`（真源 :123-137）。
fn is_legacy_agent_tool(tool_call: &ToolIdentityLike, raw_names: &RawToolNameCandidates) -> bool {
    let kind = normalize_legacy_token(tool_call.kind);
    let title = normalize_legacy_token(tool_call.title);
    let raw_direct = normalize_legacy_token(raw_names.direct.as_deref());
    let raw_zcode = normalize_legacy_token(raw_names.zcode.as_deref());

    raw_zcode == "agent"
        || raw_direct == "agent"
        || kind == "agent"
        || (kind == "think" && (title == "agent" || title == "task"))
        || (kind == "think"
            && is_record(tool_call.input)
            && tool_call
                .input
                .get("subagent_type")
                .is_some_and(Value::is_string))
}

/// `isLegacySkillTool`（真源 :139-146）。
fn is_legacy_skill_tool(tool_call: &ToolIdentityLike) -> bool {
    let kind = normalize_legacy_token(tool_call.kind);
    let title = normalize_legacy_token(tool_call.title);
    title == "skill"
        || kind == "skill"
        || (kind == "other"
            && is_record(tool_call.input)
            && tool_call.input.get("skill").is_some_and(Value::is_string))
}

/// `hasAskUserQuestionPayload`（真源 :148-150）。
fn has_ask_user_question_payload(tool_call: &ToolIdentityLike) -> bool {
    let read = read_ask_user_question_input(tool_call.input, tool_call.raw);
    !normalize_ask_user_question_input(&read)
        .questions
        .is_empty()
}

/// `isLegacyGoalToolToken`（真源 :152-154）。
fn is_legacy_goal_tool_token(value: &str) -> bool {
    value == "goalcreate" || value == "goalupdate"
}

fn legacy_kind_family_re() -> &'static [Regex; 5] {
    static RES: OnceLock<[Regex; 5]> = OnceLock::new();
    RES.get_or_init(|| {
        [
            // 真源 :157 —— `^(?:read|view|open|cat|head|tail|read_file)(?:_|$)` /i
            Regex::new(r"(?i)^(?:read|view|open|cat|head|tail|read_file)(?:_|$)")
                .expect("file-read 正则"),
            // 真源 :162-165 —— `(?:^|_)(?:edit|patch|...|apply_patch)(?:_|$)` /i
            Regex::new(
                r"(?i)(?:^|_)(?:edit|patch|replace|multi_edit|multiedit|write|create|save|apply_patch)(?:_|$)",
            )
            .expect("file-write 正则"),
            // 真源 :169 —— `^(?:execute|run|exec|bash|shell|command|terminal)(?:_|$)` /i
            Regex::new(r"(?i)^(?:execute|run|exec|bash|shell|command|terminal)(?:_|$)")
                .expect("shell 正则"),
            // 真源 :174-177 —— `^(?:search|grep|...|tree)(?:_|$)` /i
            Regex::new(
                r"(?i)^(?:search|grep|find|fetch|web_search|web_fetch|webfetch|query|lookup|glob|list|ls|dir|tree)(?:_|$)",
            )
            .expect("search 正则"),
            // 真源 :181 —— `^(?:explore|inspect)(?:_|$)` /i
            Regex::new(r"(?i)^(?:explore|inspect)(?:_|$)").expect("explore 正则"),
        ]
    })
}

/// `resolveLegacyKindFamily`（真源 :156-183）。
fn resolve_legacy_kind_family(kind: &str) -> Option<ToolFamily> {
    let res = legacy_kind_family_re();
    if res[0].is_match(kind) {
        return Some(ToolFamily::FileRead);
    }
    if res[1].is_match(kind) {
        return Some(ToolFamily::FileWrite);
    }
    if res[2].is_match(kind) {
        return Some(ToolFamily::Shell);
    }
    if res[3].is_match(kind) {
        return Some(ToolFamily::Search);
    }
    if res[4].is_match(kind) {
        return Some(ToolFamily::Explore);
    }
    None
}

/// `isPlanModeExitToken`（真源 :185-194）。
fn is_plan_mode_exit_token(value: &str) -> bool {
    matches!(
        value,
        "switch_mode"
            | "switchmode"
            | "exited_plan_mode"
            | "exitedplanmode"
            | "exit_plan_mode"
            | "exitplanmode"
    )
}

/// `resolveToolCallIdentity`（真源 :196-283）。
pub fn resolve_tool_call_identity(tool_call: &ToolIdentityLike) -> ToolCallIdentity {
    let raw_names = read_raw_tool_name_candidates(tool_call.raw);
    let normalized_kind = normalize_legacy_token(tool_call.kind);
    let normalized_title = normalize_legacy_token(tool_call.title);

    if normalized_kind == "explore" {
        return identity_from_legacy_family(
            Some("Explore".to_string()),
            ToolFamily::Explore,
            ToolIdentitySource::LegacyKind,
        );
    }

    for (value, source) in [
        (tool_call.tool_name, ToolIdentitySource::ToolName),
        (tool_call.kind, ToolIdentitySource::Kind),
        (raw_names.direct.as_deref(), ToolIdentitySource::Raw),
        (raw_names.zcode.as_deref(), ToolIdentitySource::RawZcodeMeta),
    ] {
        if let Some(identity) = identity_from_known_tool_name(value, source) {
            return identity;
        }
    }

    if is_legacy_agent_tool(tool_call, &raw_names) {
        return identity_from_legacy_family(
            raw_names
                .direct
                .clone()
                .or_else(|| raw_names.zcode.clone())
                .or_else(|| Some("Agent".to_string())),
            ToolFamily::Agent,
            ToolIdentitySource::LegacyPayload,
        );
    }

    // 真源 :218-221 —— `Task` 是现役工具名，但历史投影用
    // kind="think" + title="Task" 表示子 agent 活动。title 判断必须在
    // legacy payload 之后，避免把旧会话误升级。
    if let Some(title_identity) =
        identity_from_known_tool_name(tool_call.title, ToolIdentitySource::Title)
    {
        return title_identity;
    }

    // 真源 :223-241 —— todo 计划工具（6 候选，不含 raw zcode）。
    let todo_candidates = [
        tool_call.tool_name,
        tool_call.kind,
        tool_call.title,
        raw_names.direct.as_deref(),
        raw_names.raw_kind.as_deref(),
        raw_names.raw_title.as_deref(),
    ];
    if todo_candidates
        .into_iter()
        .flatten()
        .any(is_todo_plan_tool_name)
    {
        return identity_from_legacy_family(
            Some("TodoWrite".to_string()),
            ToolFamily::Todo,
            ToolIdentitySource::LegacyTitle,
        );
    }

    // 真源 :243-265 —— legacy Goal（normalize 后判 token）。
    if todo_candidates
        .into_iter()
        .flatten()
        .map(|v| normalize_legacy_token(Some(v)))
        .any(|t| is_legacy_goal_tool_token(&t))
    {
        return identity_from_legacy_family(
            tool_call
                .tool_name
                .map(|v| v.trim().to_string())
                .or_else(|| tool_call.kind.map(|v| v.trim().to_string()))
                .or_else(|| Some("GoalUpdate".to_string())),
            ToolFamily::Goal,
            ToolIdentitySource::LegacyTitle,
        );
    }

    if normalized_title == "enterplanmode" {
        return identity_from_legacy_family(
            Some("EnterPlanMode".to_string()),
            ToolFamily::PlanGuidance,
            ToolIdentitySource::LegacyTitle,
        );
    }

    // 真源 :271-289 —— plan mode 退出（7 候选，含 raw zcode）。
    if {
        let plan_exit_candidates = [
            tool_call.tool_name,
            tool_call.kind,
            tool_call.title,
            raw_names.direct.as_deref(),
            raw_names.zcode.as_deref(),
            raw_names.raw_kind.as_deref(),
            raw_names.raw_title.as_deref(),
        ];
        plan_exit_candidates
            .into_iter()
            .flatten()
            .map(|v| normalize_legacy_token(Some(v)))
            .any(|t| is_plan_mode_exit_token(&t))
    } {
        return identity_from_legacy_family(
            Some("switch_mode".to_string()),
            ToolFamily::SwitchMode,
            ToolIdentitySource::LegacyKind,
        );
    }

    if is_legacy_skill_tool(tool_call) {
        return identity_from_legacy_family(
            Some("Skill".to_string()),
            ToolFamily::Skill,
            ToolIdentitySource::LegacyPayload,
        );
    }

    if normalized_kind == "ask_question"
        || normalized_title == "askuserquestion"
        || has_ask_user_question_payload(tool_call)
    {
        return identity_from_legacy_family(
            Some("AskUserQuestion".to_string()),
            ToolFamily::AskUserQuestion,
            ToolIdentitySource::LegacyPayload,
        );
    }

    if let Some(legacy_kind_family) = resolve_legacy_kind_family(&normalized_kind) {
        return identity_from_legacy_family(
            tool_call.kind.map(|v| v.trim().to_string()),
            legacy_kind_family,
            ToolIdentitySource::LegacyKind,
        );
    }

    let title_prefix_family =
        resolve_legacy_kind_family(normalized_title.split('_').next().unwrap_or(""));
    if title_prefix_family == Some(ToolFamily::FileRead) {
        return identity_from_legacy_family(
            tool_call.title.map(|v| v.trim().to_string()),
            ToolFamily::FileRead,
            ToolIdentitySource::LegacyTitle,
        );
    }

    unknown_tool_identity()
}

/// `isFileContentWriteToolCall`（真源 :285-297）。
pub fn is_file_content_write_tool_call(
    tool_call: &ToolIdentityLike,
    identity: Option<&ToolCallIdentity>,
) -> bool {
    let fallback;
    let identity = match identity {
        Some(i) => i,
        None => {
            fallback = resolve_tool_call_identity(tool_call);
            &fallback
        }
    };
    if identity.family != ToolFamily::FileWrite {
        return false;
    }
    if is_zcode_file_content_write_tool_name(identity.tool_name.as_deref()) {
        return true;
    }
    static CONTENT_WRITE_RE: OnceLock<Regex> = OnceLock::new();
    let legacy_kind = normalize_legacy_token(tool_call.kind);
    CONTENT_WRITE_RE
        .get_or_init(|| {
            // 真源 :295 —— `^(?:write|create|save)(?:_|$)` /i
            Regex::new(r"(?i)^(?:write|create|save)(?:_|$)").expect("content-write 正则")
        })
        .is_match(&legacy_kind)
}

/// `isFileDiffToolCall`（真源 :299-306）。
pub fn is_file_diff_tool_call(
    tool_call: &ToolIdentityLike,
    identity: Option<&ToolCallIdentity>,
) -> bool {
    let owned;
    let identity = match identity {
        Some(i) => i,
        None => {
            owned = resolve_tool_call_identity(tool_call);
            &owned
        }
    };
    identity.family == ToolFamily::FileWrite
        && !is_file_content_write_tool_call(tool_call, Some(identity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn like<'a>(
        tool_name: Option<&'a str>,
        kind: Option<&'a str>,
        title: Option<&'a str>,
        input: &'a Value,
        raw: &'a Value,
    ) -> ToolIdentityLike<'a> {
        ToolIdentityLike {
            tool_name,
            kind,
            title,
            input,
            raw,
        }
    }

    #[test]
    fn normalize_known_tool_names() {
        assert_eq!(normalize_zcode_tool_name(Some("read")), Some("Read"));
        assert_eq!(normalize_zcode_tool_name(Some(" READ ")), Some("Read"));
        // snake_case 的 web_search 与 submit_result 保持字面。
        assert_eq!(
            normalize_zcode_tool_name(Some("web_search")),
            Some("web_search")
        );
        assert_eq!(
            normalize_zcode_tool_name(Some("SUBMIT_RESULT")),
            Some("submit_result")
        );
        assert_eq!(normalize_zcode_tool_name(Some("unknown_tool")), None);
        assert_eq!(normalize_zcode_tool_name(Some("  ")), None);
        assert_eq!(normalize_zcode_tool_name(None), None);
    }

    #[test]
    fn known_name_source_priority() {
        let input = json!({});
        let raw = json!({});
        // toolName 优先。
        let id = resolve_tool_call_identity(&like(Some("Read"), Some("Bash"), None, &input, &raw));
        assert_eq!(id.family, ToolFamily::FileRead);
        assert_eq!(id.source, ToolIdentitySource::ToolName);
        assert_eq!(id.tool_name.as_deref(), Some("Read"));
        assert!(!id.is_legacy);
        // toolName 未知名 → kind 命中。
        let id =
            resolve_tool_call_identity(&like(Some("mcp__foo"), Some("Bash"), None, &input, &raw));
        assert_eq!(id.family, ToolFamily::Shell);
        assert_eq!(id.source, ToolIdentitySource::Kind);
        // → raw.toolName。
        let raw_direct = json!({"toolName": "Grep"});
        let id = resolve_tool_call_identity(&like(None, None, None, &input, &raw_direct));
        assert_eq!(id.family, ToolFamily::Search);
        assert_eq!(id.source, ToolIdentitySource::Raw);
        // → raw._meta.zcode.toolName。
        let raw_zcode = json!({"_meta": {"zcode": {"toolName": "WebFetch"}}});
        let id = resolve_tool_call_identity(&like(None, None, None, &input, &raw_zcode));
        assert_eq!(id.family, ToolFamily::Search);
        assert_eq!(id.source, ToolIdentitySource::RawZcodeMeta);
    }

    #[test]
    fn explore_kind_wins_first() {
        let input = json!({});
        let raw = json!({});
        let id =
            resolve_tool_call_identity(&like(Some("Bash"), Some("Explore"), None, &input, &raw));
        assert_eq!(id.family, ToolFamily::Explore);
        assert_eq!(id.source, ToolIdentitySource::LegacyKind);
        assert!(id.is_legacy);
    }

    #[test]
    fn legacy_agent_payload() {
        let input = json!({});
        let raw = json!({});
        // kind="think" + title="Task" → legacy agent（在 title 命中之前）。
        let id = resolve_tool_call_identity(&like(None, Some("think"), Some("Task"), &input, &raw));
        assert_eq!(id.family, ToolFamily::Agent);
        assert_eq!(id.source, ToolIdentitySource::LegacyPayload);
        assert!(id.is_legacy);
        // kind="think" + input.subagent_type 字符串。
        let input_sub = json!({"subagent_type": "explorer"});
        let id =
            resolve_tool_call_identity(&like(None, Some("think"), Some("随便"), &input_sub, &raw));
        assert_eq!(id.family, ToolFamily::Agent);
    }

    #[test]
    fn title_identity_after_legacy_agent() {
        let input = json!({});
        let raw = json!({});
        // 单纯 title="Bash"（kind 不命中）→ title 来源。
        let id = resolve_tool_call_identity(&like(
            None,
            Some("custom_kind"),
            Some("Bash"),
            &input,
            &raw,
        ));
        assert_eq!(id.family, ToolFamily::Shell);
        assert_eq!(id.source, ToolIdentitySource::Title);
    }

    #[test]
    fn todo_and_goal_legacy_tokens() {
        let input = json!({});
        let raw = json!({});
        let id = resolve_tool_call_identity(&like(None, Some("update_plan"), None, &input, &raw));
        assert_eq!(id.family, ToolFamily::Todo);
        assert_eq!(id.tool_name.as_deref(), Some("TodoWrite"));
        let id = resolve_tool_call_identity(&like(None, Some("GoalUpdate"), None, &input, &raw));
        assert_eq!(id.family, ToolFamily::Goal);
        assert_eq!(id.tool_name.as_deref(), Some("GoalUpdate"));
    }

    #[test]
    fn plan_guidance_and_switch_mode() {
        let input = json!({});
        let raw = json!({});
        let id =
            resolve_tool_call_identity(&like(None, Some("x"), Some("EnterPlanMode"), &input, &raw));
        assert_eq!(id.family, ToolFamily::PlanGuidance);
        assert!(id.is_legacy);
        // 旧 token：Exited Plan Mode（空白折下划线后在表里）。
        let id =
            resolve_tool_call_identity(&like(None, Some("Exited Plan Mode"), None, &input, &raw));
        assert_eq!(id.family, ToolFamily::SwitchMode);
        assert_eq!(id.tool_name.as_deref(), Some("switch_mode"));
    }

    #[test]
    fn skill_and_ask_question_payloads() {
        let raw = json!({});
        // kind="other" + input.skill。
        let input_skill = json!({"skill": "commit"});
        let id = resolve_tool_call_identity(&like(None, Some("other"), None, &input_skill, &raw));
        assert_eq!(id.family, ToolFamily::Skill);
        // ask_question kind。
        let input = json!({});
        let id = resolve_tool_call_identity(&like(None, Some("ask_question"), None, &input, &raw));
        assert_eq!(id.family, ToolFamily::AskUserQuestion);
        // 无 kind 但 input 带问题载荷。
        let input_q = json!({"questions": [{"question": "Q", "options": []}]});
        let id = resolve_tool_call_identity(&like(None, Some("misc"), None, &input_q, &raw));
        assert_eq!(id.family, ToolFamily::AskUserQuestion);
    }

    #[test]
    fn legacy_kind_family_and_title_prefix() {
        let input = json!({});
        let raw = json!({});
        // kind 前缀 read_file → file-read。
        let id = resolve_tool_call_identity(&like(None, Some("read_file"), None, &input, &raw));
        assert_eq!(id.family, ToolFamily::FileRead);
        assert_eq!(id.source, ToolIdentitySource::LegacyKind);
        // kind 中缀 edit（xx_edit_yy）。
        let id =
            resolve_tool_call_identity(&like(None, Some("multi_edit_file"), None, &input, &raw));
        assert_eq!(id.family, ToolFamily::FileWrite);
        // title 前缀（kind 不命中）→ title 来源 file-read。
        let id = resolve_tool_call_identity(&like(
            None,
            Some("zzz"),
            Some("read_file_extra"),
            &input,
            &raw,
        ));
        assert_eq!(id.family, ToolFamily::FileRead);
        assert_eq!(id.source, ToolIdentitySource::LegacyTitle);
        // 全不命中 → unknown。
        let id = resolve_tool_call_identity(&like(None, Some("zzz"), Some("qqq"), &input, &raw));
        assert_eq!(id.family, ToolFamily::Unknown);
        assert_eq!(id.source, ToolIdentitySource::Unknown);
    }

    #[test]
    fn normalize_legacy_token_folds_separators() {
        assert_eq!(normalize_legacy_token(Some("Read File")), "read_file");
        assert_eq!(normalize_legacy_token(Some("read-file")), "read_file");
        assert_eq!(normalize_legacy_token(Some("read -  file")), "read_file");
        assert_eq!(normalize_legacy_token(None), "");
    }

    #[test]
    fn file_content_write_vs_diff() {
        let input = json!({});
        let raw = json!({});
        // Write → 内容写入。
        let tc = like(Some("Write"), None, None, &input, &raw);
        assert!(is_file_content_write_tool_call(&tc, None));
        assert!(!is_file_diff_tool_call(&tc, None));
        // Edit → diff。
        let tc = like(Some("Edit"), None, None, &input, &raw);
        assert!(!is_file_content_write_tool_call(&tc, None));
        assert!(is_file_diff_tool_call(&tc, None));
        // legacy kind="write_file" → 内容写入。
        let tc = like(None, Some("write_file"), None, &input, &raw);
        assert!(is_file_content_write_tool_call(&tc, None));
        // 非 file-write family → 都 false。
        let tc = like(Some("Bash"), None, None, &input, &raw);
        assert!(!is_file_content_write_tool_call(&tc, None));
        assert!(!is_file_diff_tool_call(&tc, None));
    }
}
