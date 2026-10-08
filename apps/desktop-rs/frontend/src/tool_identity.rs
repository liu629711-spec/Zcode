//! 工具身份与渲染分流（1:1 翻译）。
//!
//! 真源三处：
//! - `packages/shared/src/tool-identity.ts:57-92`：`TOOL_FAMILY_BY_NAME` 静态表，
//!   查表大小写不敏感（`TOOL_NAME_BY_LOWER`，:94-107）
//! - `packages/ui/src/ToolCallBlocks/resolveRenderer.ts:57-173`：三层分流
//! - `packages/ui/src/lib/workflowToolNames.ts`：工作流工具**按名字**判定
//!
//! **分层顺序不可换**（真源注释反复强调）：
//! 1. `toolCall.kind` 聚合类（changesGroup / executeGroup / cuaGroup）
//! 2. 工作流系列**按工具名**——必须先于 family，否则这些名字不在已知工具表里，
//!    identity 回 unknown 会掉进 raw JSON 兜底卡；哪天它们被登记进 `workflow` family，
//!    family 兜底（CreateWorkflow 卡）会把「保存工作流」静默渲染成「创建工作流」
//! 3. `identity.family` switch
//!
//! **第一版不复刻 legacy 兼容正则**（`lib/toolIdentity.ts:162-187` 的
//! `resolveLegacyKindFamily`，按 `read|view|cat` / `edit|patch|write` 等 kind 前缀匹配）。
//! 真源那层是给旧数据兜底的，新数据一律走 toolName 精确查表。

use std::collections::HashMap;
use std::sync::OnceLock;

/// 工具家族（真源 `ZCodeToolFamily`，tool-identity.ts:44-57）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolFamily {
    FileRead,
    FileWrite,
    Shell,
    Search,
    Todo,
    AskUserQuestion,
    Agent,
    Skill,
    Goal,
    SessionContext,
    Message,
    TaskControl,
    NodeRepl,
    Workflow,
    /// 不在已知工具表里（真源 identity 回 unknown）。
    Unknown,
}

impl ToolFamily {
    /// 真源字符串值（部分 UI 文案与逻辑按字符串判别）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FileRead => "file-read",
            Self::FileWrite => "file-write",
            Self::Shell => "shell",
            Self::Search => "search",
            Self::Todo => "todo",
            Self::AskUserQuestion => "ask-user-question",
            Self::Agent => "agent",
            Self::Skill => "skill",
            Self::Goal => "goal",
            Self::SessionContext => "session-context",
            Self::Message => "message",
            Self::TaskControl => "task-control",
            Self::NodeRepl => "node-repl",
            Self::Workflow => "workflow",
            Self::Unknown => "unknown",
        }
    }

    /// 是否是文件写入类——**rawFileSummaries 的门控**（fileSummaries.ts:107-112）。
    /// 非 file-write 必须返回空数组，否则 search/explore 会被误判成 edit/delete。
    pub fn writes_files(self) -> bool {
        matches!(self, Self::FileWrite)
    }
}

/// 工具身份（真源 `resolveToolCallIdentity` 的返回形态，取我们需要的三项）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolIdentity {
    /// 归一化后的标准工具名（未知时为空）。
    pub tool_name: String,
    pub family: ToolFamily,
    /// 是否是靠 legacy 正则兜出来的（Rust 侧第一版恒 false）。
    pub legacy: bool,
}

/// 归一化：抹掉大小写与分隔符（真源 `normalizeToolToken`，workflowToolNames.ts:23-26）。
/// `SaveWorkflow` / `save_workflow` / `save-workflow` 归一后都是 `saveworkflow`。
pub fn normalize_tool_token(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect()
}

/// 已知工具 → family 的静态表（真源 tool-identity.ts:57-92，30 条）。
fn family_table() -> &'static HashMap<&'static str, ToolFamily> {
    static TABLE: OnceLock<HashMap<&'static str, ToolFamily>> = OnceLock::new();
    TABLE.get_or_init(|| {
        use ToolFamily::*;
        [
            // 文件读
            ("Read", FileRead),
            // 文件写：Write/Edit/ApplyPatch 同family
            ("Write", FileWrite),
            ("Edit", FileWrite),
            ("ApplyPatch", FileWrite),
            // 命令执行
            ("Bash", Shell),
            // 检索：Glob/Grep/WebFetch/WebSearch/web_search 同 family
            ("Glob", Search),
            ("Grep", Search),
            ("WebFetch", Search),
            ("WebSearch", Search),
            ("web_search", Search),
            // 待办
            ("TodoRead", Todo),
            ("TodoWrite", Todo),
            // 目标
            ("GoalRead", Goal),
            // 会话上下文
            ("ReadSessionContext", SessionContext),
            // 向用户提问
            ("AskUserQuestion", AskUserQuestion),
            // 消息：SendMessage / RespondToCoordinator
            ("SendMessage", Message),
            ("RespondToCoordinator", Message),
            // 任务控制
            ("TaskOutput", TaskControl),
            // 真源注释：TaskStop 未登记时 UI identity 会退回 unknown，最终落到 raw fallback。
            // 这里仍然登记（与真源表一致）。
            ("TaskStop", TaskControl),
            // Node REPL：built-in 名 + MCP 前缀名（MCP 暴露会带前缀，
            // 只登记旧名会让专用 REPL 卡退回 fallback）。
            ("js", NodeRepl),
            ("js_reset", NodeRepl),
            ("js_add_node_module_dir", NodeRepl),
            ("mcp__node_repl__js", NodeRepl),
            ("mcp__node_repl__js_reset", NodeRepl),
            ("mcp__node_repl__js_add_node_module_dir", NodeRepl),
            // 智能体
            ("Agent", Agent),
            ("Task", Agent),
            // 技能
            ("Skill", Skill),
            // 工作流
            ("CreateWorkflow", Workflow),
            ("AmendWorkflow", Workflow),
            ("submit_result", Workflow),
        ]
        .into_iter()
        .collect()
    })
}

/// 小写名 → 家族（真源 `TOOL_NAME_BY_LOWER` 的反向效果）。
fn family_by_lower() -> &'static HashMap<String, ToolFamily> {
    static TABLE: OnceLock<HashMap<String, ToolFamily>> = OnceLock::new();
    TABLE.get_or_init(|| {
        family_table()
            .iter()
            .map(|(name, family)| (name.to_lowercase(), *family))
            .collect()
    })
}

/// 解析工具身份（真源 `resolveToolCallIdentity` 的核心）。
///
/// 查表大小写不敏感；未命中回 `Unknown`（真源不回退 legacy 正则，见模块文档）。
pub fn resolve_tool_identity(tool_name: &str) -> ToolIdentity {
    let family = family_by_lower()
        .get(&tool_name.to_lowercase())
        .copied()
        .unwrap_or(ToolFamily::Unknown);
    ToolIdentity {
        tool_name: if family == ToolFamily::Unknown {
            String::new()
        } else {
            tool_name.to_string()
        },
        family,
        legacy: false,
    }
}

// ---------------------------------------------------------------------------
// 渲染分流（真源 resolveRenderer.ts）
// ---------------------------------------------------------------------------

/// 渲染器种类。真源是 React 组件，这里用枚举表达同一张分流表。
///
/// 顺序与真源 `resolveRenderer.ts` 的判定顺序对应，见文档分层说明。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renderer {
    // ── ① kind 聚合类（最高优先，不看工具名）──
    ChangesGroup,
    ExecuteGroup,
    CuaGroup,

    // ── ② 工作流系列按工具名（必须先于 family）──
    SaveWorkflow,
    ListSavedWorkflows,
    GetWorkflowRun,
    ListWorkflowRuns,
    EvalWorkflowSnippet,
    ResumeWorkflowRun,
    ListModels,
    Escalate,
    ResolveWorkflowQuestion,

    // ── ③ family switch ──
    Read,
    Edit,
    Execute,
    Search,
    /// 探索卡。真源 `ToolFamily` 类型里其实**没有** explore 这个 family
    /// （`resolveRenderer.ts:161` 引用了它，属于类型漏洞）——它只靠 legacy 正则
    /// 命中 `kind` 前缀才会走到。Rust 第一版无 legacy 层，故该卡暂不可达，
    /// 但渲染器本身完整保留，等 legacy 层补上即生效。
    Explore,
    Todo,
    Agent,
    Skill,
    Goal,
    /// message family 内按工具名再分（真源 :141）。
    SendMessage,
    RespondToCoordinator,
    /// task-control family 内按工具名再分（真源 :145）。
    TaskOutput,
    TaskStop,
    NodeRepl,
    /// workflow family 内按工具名再分（真源 :151）。
    CreateWorkflow,
    SubmitResult,
    ReadSessionContext,
    AskQuestion,
    PlanGuidance,
    SwitchMode,
    /// MCP 通用卡（真源 resolveRenderer 的 MCP 分支，在 node-repl 之后）。
    Mcp,
    /// family 兜底（真源 :171）。
    Fallback,
}

/// `toolCall.kind` 的聚合类判定（真源 resolveRenderer.ts:58-66）。
///
/// v4 下 `kind` 直接等于 `toolName`（toolCallRowAdapter.ts:88），所以这里同一个值两用。
pub fn aggregate_renderer(kind: &str) -> Option<Renderer> {
    match kind {
        "changesGroup" => Some(Renderer::ChangesGroup),
        "executeGroup" => Some(Renderer::ExecuteGroup),
        "cuaGroup" => Some(Renderer::CuaGroup),
        _ => None,
    }
}

/// 工作流工具的按名判定（真源 `workflowToolNames.ts`）。
///
/// 归一化后比对 token：抹掉大小写与分隔符，所以
/// `SaveWorkflow` / `save_workflow` / `SAVE-WORKFLOW` 都命中 `saveworkflow`。
pub fn workflow_renderer(tool_name: &str) -> Option<Renderer> {
    let token = normalize_tool_token(tool_name);
    match token.as_str() {
        "saveworkflow" => Some(Renderer::SaveWorkflow),
        "listsavedworkflows" => Some(Renderer::ListSavedWorkflows),
        "getworkflowrun" => Some(Renderer::GetWorkflowRun),
        "listworkflowruns" => Some(Renderer::ListWorkflowRuns),
        "evalworkflowsnippet" => Some(Renderer::EvalWorkflowSnippet),
        "resumeworkflowrun" => Some(Renderer::ResumeWorkflowRun),
        "listmodels" => Some(Renderer::ListModels),
        "escalate" => Some(Renderer::Escalate),
        "resolveworkflowquestion" => Some(Renderer::ResolveWorkflowQuestion),
        _ => None,
    }
}

/// 完整分流（真源 `resolveRenderer.ts` 主体）。
///
/// 参数 `is_mcp_tool` 对应真源的 mcp_tool presentation——通用 MCP 卡要抢在
/// family 之前，但 **node-repl 必须保住专用卡**（真源 :117-120 注释：
/// 通用 MCP 分流会吞掉代码、错误栈和 artifact 等专用交互）。
pub fn resolve_renderer(kind: &str, tool_name: &str, is_mcp_tool: bool) -> Renderer {
    // ① kind 聚合类
    if let Some(r) = aggregate_renderer(kind) {
        return r;
    }
    // ② 工作流按名（先于 family）
    if let Some(r) = workflow_renderer(tool_name) {
        return r;
    }
    let identity = resolve_tool_identity(tool_name);
    // ③ family switch
    let renderer = match identity.family {
        ToolFamily::NodeRepl => Renderer::NodeRepl,
        ToolFamily::FileRead => Renderer::Read,
        ToolFamily::FileWrite => Renderer::Edit,
        ToolFamily::Shell => Renderer::Execute,
        ToolFamily::Search => Renderer::Search,
        ToolFamily::Todo => Renderer::Todo,
        ToolFamily::Agent => Renderer::Agent,
        ToolFamily::Skill => Renderer::Skill,
        ToolFamily::Goal => Renderer::Goal,
        ToolFamily::Message => {
            // 真源 :141 —— RespondToCoordinator 走独立卡。
            if tool_name == "RespondToCoordinator" {
                Renderer::RespondToCoordinator
            } else {
                Renderer::SendMessage
            }
        }
        ToolFamily::TaskControl => {
            // 真源 :145 —— TaskOutput 与 TaskStop 分卡。
            if tool_name == "TaskOutput" {
                Renderer::TaskOutput
            } else {
                Renderer::TaskStop
            }
        }
        ToolFamily::Workflow => {
            // 真源 :151 —— submit_result 分卡。
            if tool_name == "submit_result" {
                Renderer::SubmitResult
            } else {
                Renderer::CreateWorkflow
            }
        }
        ToolFamily::SessionContext => Renderer::ReadSessionContext,
        ToolFamily::AskUserQuestion => Renderer::AskQuestion,
        // 真源 family 枚举里还有 plan-guidance / switch-mode 两个分支，
        // 但它们不在 TOOL_FAMILY_BY_NAME 表里（只能靠 legacy 正则命中），
        // Rust 第一版无 legacy 层，故这两种工具自然落到 Unknown → Fallback。
        ToolFamily::Unknown => Renderer::Fallback,
    };

    // MCP 通用卡：node-repl 已被上面保住，其余 MCP 走通用展示（真源 :117-120）。
    if is_mcp_tool && renderer != Renderer::NodeRepl {
        return Renderer::Mcp;
    }
    renderer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_tools_map_to_families() {
        // 真源 tool-identity.ts:57-92 的映射，逐条对齐。
        use ToolFamily::*;
        let cases: &[(&str, ToolFamily)] = &[
            ("Read", FileRead),
            ("Write", FileWrite),
            ("Edit", FileWrite),
            ("ApplyPatch", FileWrite),
            ("Bash", Shell),
            ("Glob", Search),
            ("Grep", Search),
            ("WebFetch", Search),
            ("WebSearch", Search),
            ("web_search", Search),
            ("TodoRead", Todo),
            ("TodoWrite", Todo),
            ("GoalRead", Goal),
            ("ReadSessionContext", SessionContext),
            ("AskUserQuestion", AskUserQuestion),
            ("SendMessage", Message),
            ("RespondToCoordinator", Message),
            ("TaskOutput", TaskControl),
            ("TaskStop", TaskControl),
            ("js", NodeRepl),
            ("mcp__node_repl__js", NodeRepl),
            ("Agent", Agent),
            ("Task", Agent),
            ("Skill", Skill),
            ("CreateWorkflow", Workflow),
            ("AmendWorkflow", Workflow),
            ("submit_result", Workflow),
        ];
        for (tool, family) in cases {
            assert_eq!(
                resolve_tool_identity(tool).family,
                *family,
                "{tool} 的 family 应为 {family:?}"
            );
        }
    }

    #[test]
    fn lookup_is_case_insensitive() {
        // 真源 TOOL_NAME_BY_LOWER（:94-107）大小写不敏感。
        for name in ["read", "READ", "ReAd", "rEaD"] {
            assert_eq!(resolve_tool_identity(name).family, ToolFamily::FileRead);
        }
        assert_eq!(
            resolve_tool_identity("bAsh").family,
            ToolFamily::Shell,
            "Bash 的家族是 shell，不是大写敏感"
        );
    }

    #[test]
    fn unknown_tool_returns_unknown_family() {
        let id = resolve_tool_identity("SomeRandomTool");
        assert_eq!(id.family, ToolFamily::Unknown);
        assert!(id.tool_name.is_empty(), "未知工具名应清空");
        assert!(!id.legacy);
    }

    #[test]
    fn file_write_family_is_the_gate_for_file_summaries() {
        // fileSummaries.ts:107-112：非 file-write 必须返回空 rawFileSummaries。
        assert!(ToolFamily::FileWrite.writes_files());
        for f in [
            ToolFamily::FileRead,
            ToolFamily::Search,
            ToolFamily::Shell,
            ToolFamily::Unknown,
        ] {
            assert!(
                !f.writes_files(),
                "{} 不该通过 file-write 门控，否则 search/explore 会被误渲染成 edit",
                f.as_str()
            );
        }
    }

    #[test]
    fn normalize_strips_case_and_separators() {
        assert_eq!(normalize_tool_token("SaveWorkflow"), "saveworkflow");
        assert_eq!(normalize_tool_token("save_workflow"), "saveworkflow");
        assert_eq!(normalize_tool_token("SAVE-WORKFLOW"), "saveworkflow");
        assert_eq!(normalize_tool_token("save workflow"), "saveworkflow");
        assert_eq!(normalize_tool_token("mcp__node_repl__js"), "mcpnoderepljs");
    }

    #[test]
    fn workflow_tools_match_by_normalized_name() {
        // 真源 workflowToolNames.ts 的九个判定（全部归一化后比对 token）。
        assert_eq!(workflow_renderer("SaveWorkflow"), Some(Renderer::SaveWorkflow));
        assert_eq!(
            workflow_renderer("save_workflow"),
            Some(Renderer::SaveWorkflow),
            "下划线写法也要命中"
        );
        assert_eq!(
            workflow_renderer("ListSavedWorkflows"),
            Some(Renderer::ListSavedWorkflows)
        );
        assert_eq!(workflow_renderer("get_workflow_run"), Some(Renderer::GetWorkflowRun));
        assert_eq!(
            workflow_renderer("listWorkflowRuns"),
            Some(Renderer::ListWorkflowRuns)
        );
        assert_eq!(
            workflow_renderer("EvalWorkflowSnippet"),
            Some(Renderer::EvalWorkflowSnippet)
        );
        assert_eq!(
            workflow_renderer("ResumeWorkflowRun"),
            Some(Renderer::ResumeWorkflowRun)
        );
        assert_eq!(workflow_renderer("ListModels"), Some(Renderer::ListModels));
        assert_eq!(workflow_renderer("escalate"), Some(Renderer::Escalate));
        assert_eq!(
            workflow_renderer("ResolveWorkflowQuestion"),
            Some(Renderer::ResolveWorkflowQuestion)
        );
        assert_eq!(workflow_renderer("CreateWorkflow"), None, "创建入口不走按名判定");
    }

    #[test]
    fn family_switch_covers_every_registered_family() {
        // 每个 family 至少一个代表性工具，验证 switch 覆盖完整。
        let cases: &[(&str, Renderer)] = &[
            ("Read", Renderer::Read),
            ("Edit", Renderer::Edit),
            ("Write", Renderer::Edit), // 同 family → 同卡
            ("Bash", Renderer::Execute),
            ("Grep", Renderer::Search),
            ("TodoWrite", Renderer::Todo),
            ("Agent", Renderer::Agent),
            ("Skill", Renderer::Skill),
            ("GoalRead", Renderer::Goal),
            ("ReadSessionContext", Renderer::ReadSessionContext),
            ("AskUserQuestion", Renderer::AskQuestion),
            ("SendMessage", Renderer::SendMessage),
            ("RespondToCoordinator", Renderer::RespondToCoordinator),
            ("TaskOutput", Renderer::TaskOutput),
            ("TaskStop", Renderer::TaskStop),
            ("js", Renderer::NodeRepl),
            ("CreateWorkflow", Renderer::CreateWorkflow),
            ("submit_result", Renderer::SubmitResult),
        ];
        for (tool, expected) in cases {
            assert_eq!(
                resolve_renderer(tool, tool, false),
                *expected,
                "{tool} 的 renderer 应为 {expected:?}"
            );
        }
        // 未登记的 family（真源 legacy 正则覆盖）落到 Fallback。
        assert_eq!(resolve_renderer("Whatever", "Whatever", false), Renderer::Fallback);
    }

    #[test]
    fn kind_aggregation_wins_over_tool_name() {
        // 真源 resolveRenderer.ts:58-66 —— 聚合类不看工具名。
        assert_eq!(resolve_renderer("executeGroup", "Bash", false), Renderer::ExecuteGroup);
        assert_eq!(
            resolve_renderer("changesGroup", "Read", false),
            Renderer::ChangesGroup
        );
        assert_eq!(resolve_renderer("cuaGroup", "Read", false), Renderer::CuaGroup);
    }

    #[test]
    fn workflow_name_dispatch_precedes_family() {
        // ★关键顺序：这些名字不在已知工具表里（→unknown→fallback），
        // 但按名先判定让它们拿到专用卡，而不是被 workflow family 兜底成 CreateWorkflow。
        assert_eq!(
            resolve_renderer("SaveWorkflow", "SaveWorkflow", false),
            Renderer::SaveWorkflow
        );
        assert_eq!(
            resolve_renderer("ListModels", "ListModels", false),
            Renderer::ListModels,
            "ListModels 不在已知工具表，靠按名分流拿专用卡"
        );
        // 反证：真源 CreateWorkflow 走 family 兜底。
        assert_eq!(
            resolve_renderer("CreateWorkflow", "CreateWorkflow", false),
            Renderer::CreateWorkflow
        );
    }

    #[test]
    fn node_repl_survives_mcp_dispatch() {
        // 真源 :117-120：通用 MCP 分流会吞掉 node-repl 的代码/错误栈/artifact 交互，
        // 所以 node-repl 必须保住专用卡。
        assert_eq!(
            resolve_renderer("mcp__node_repl__js", "mcp__node_repl__js", true),
            Renderer::NodeRepl,
            "node-repl 不能被通用 MCP 卡吞掉"
        );
        // 其余 MCP 走通用展示。
        assert_eq!(
            resolve_renderer("mcp__other__tool", "mcp__other__tool", true),
            Renderer::Mcp
        );
        // 非 MCP 工具不受影响。
        assert_eq!(resolve_renderer("Read", "Read", false), Renderer::Read);
    }

    #[test]
    fn unknown_mcp_tool_uses_mcp_card() {
        // 未登记的 MCP 工具：family=unknown→Fallback，但 MCP 标记应转成通用 MCP 卡。
        assert_eq!(
            resolve_renderer("mcp__x__y", "mcp__x__y", true),
            Renderer::Mcp
        );
    }
}