//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/resolveRenderer.ts`（173 行）。
//!
//! 本文件**只做纯分流**，不含 JSX、不碰 context 装配（真源 :5-6 明确如此）。
//!
//! **分流顺序不可换**（真源注释反复强调）：
//! 1. `:58-68` `toolCall.kind` 聚合类（changesGroup / executeGroup / cuaGroup）+ `isCuaToolCall`
//! 2. `:73-113` 工作流系列**按工具名**——它们不在已知工具表里（identity 回 unknown，
//!    会掉进 raw JSON 兜底卡）；而 `workflow` family 的兜底是 CreateWorkflow 卡，
//!    一旦有人把这些名字登记进 workflow family，保存/列举会被静默渲染成「创建工作流」
//! 3. `:121-124` node-repl 优先于通用 MCP
//! 4. `:130-170` family switch
//!
//! 另注（:126-129 真源注释）：当前工具名已是固定集合，继续用正则扫 kind/title 会把
//! `TodoWrite` 里的 `Write` 当成文件写入，所以先解析固定 tool identity 再按 family 分流。

use serde_json::Value;

/// 工具家族（真源 `ZCodeToolFamily`，`packages/shared/src/tool-identity.ts:44-57`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolFamily {
    PlanGuidance,
    Agent,
    Todo,
    AskUserQuestion,
    Message,
    TaskControl,
    Skill,
    Workflow,
    SessionContext,
    FileRead,
    FileWrite,
    Explore,
    SwitchMode,
    Search,
    Shell,
    Goal,
    NodeRepl,
    /// 不在已知工具表里（identity 回 unknown）。
    Unknown,
}

impl ToolFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PlanGuidance => "plan-guidance",
            Self::Agent => "agent",
            Self::Todo => "todo",
            Self::AskUserQuestion => "ask-user-question",
            Self::Message => "message",
            Self::TaskControl => "task-control",
            Self::Skill => "skill",
            Self::Workflow => "workflow",
            Self::SessionContext => "session-context",
            Self::FileRead => "file-read",
            Self::FileWrite => "file-write",
            Self::Explore => "explore",
            Self::SwitchMode => "switch-mode",
            Self::Search => "search",
            Self::Shell => "shell",
            Self::Goal => "goal",
            Self::NodeRepl => "node-repl",
            Self::Unknown => "unknown",
        }
    }
}

/// 工具名 → family 静态表（真源 `tool-identity.ts:57-92`，30 条）。
///
/// 查表大小写不敏感（真源 `TOOL_NAME_BY_LOWER`，:94-107）。
pub fn family_by_lower(name: &str) -> ToolFamily {
    match name.to_lowercase().as_str() {
        "read" => ToolFamily::FileRead,
        "write" | "edit" | "applypatch" => ToolFamily::FileWrite,
        "bash" => ToolFamily::Shell,
        "glob" | "grep" | "webfetch" | "websearch" | "web_search" => ToolFamily::Search,
        "todoread" | "todowrite" => ToolFamily::Todo,
        "goalread" => ToolFamily::Goal,
        "readsessioncontext" => ToolFamily::SessionContext,
        "askuserquestion" => ToolFamily::AskUserQuestion,
        "sendmessage" | "respondtocoordinator" => ToolFamily::Message,
        "taskoutput" | "taskstop" => ToolFamily::TaskControl,
        "js" | "js_reset" | "js_add_node_module_dir" | "mcp__node_repl__js"
        | "mcp__node_repl__js_reset" | "mcp__node_repl__js_add_node_module_dir" => ToolFamily::NodeRepl,
        "agent" | "task" => ToolFamily::Agent,
        "skill" => ToolFamily::Skill,
        "createworkflow" | "amendworkflow" | "submit_result" => ToolFamily::Workflow,
        _ => ToolFamily::Unknown,
    }
}

/// 归一化工具名（真源 `workflowToolNames.ts:23-26` `normalizeToolToken`）：
/// 抹掉大小写与分隔符，`SaveWorkflow` / `save_workflow` 都归一为 `saveworkflow`。
fn normalize_tool_token(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect()
}

/// `normalizeToolName`（真源 `cuaPermissionAction.ts:3-5`）：
/// trim + 小写 + 下划线转连字符。与 `normalizeToolToken` **不同**，别混用。
fn normalize_cua_tool_name(value: &str) -> String {
    value.trim().to_lowercase().replace('_', "-")
}

/// `isZCodeCuaToolName`（真源 `cuaPermissionAction.ts:7-16`）。
///
/// 真源注释：server key 段是 `computer-use`，两种形态都包含该串——
/// `mcp__computer-use__*` 与 `mcp__plugin_zcode-cua_computer-use__*`（plugin 命名空间
/// 给 server 加前缀，归一化后 server 段前是单连字符）。用 `includes` 兼容两者，
/// 否则 main 的 namespace 前缀会让 CUA 识别失败、ToolCallBlock 退化成 fallback。
/// `computer-use` 足够特异（不会误判 android-emulator / browser-use）。
pub fn is_cua_tool_name(value: &str) -> bool {
    let n = normalize_cua_tool_name(value);
    n == "computer-use" || n.contains("computer-use")
}

/// 分流结果（真源返回的是 React 组件，这里用枚举表达同一张表）。
///
/// 顺序与真源 `resolveRenderer.ts` 的判定顺序对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renderer {
    // ── ① kind 聚合类（:58-68）──
    ChangesGroup,
    ExecuteGroup,
    CuaGroup,
    Cua,
    // ── ② 工作流按名（:73-113）──
    SaveWorkflow,
    ListSavedWorkflows,
    GetWorkflowRun,
    ListWorkflowRuns,
    EvalWorkflowSnippet,
    ResumeWorkflowRun,
    ListModels,
    Escalate,
    ResolveWorkflowQuestion,
    // ── ③ MCP 通用（:125-126）──
    Mcp,
    // ── ④ family switch（:130-170）──
    PlanGuidance,
    Agent,
    Todo,
    AskQuestion,
    /// `message` family 内按工具名分（:141-143）。
    RespondToCoordinator,
    SendMessage,
    /// `task-control` family 内按工具名分（:145）。
    TaskOutput,
    TaskStop,
    Skill,
    /// `workflow` family 内按工具名分（:151-156）。
    SubmitResult,
    CreateWorkflow,
    ReadSessionContext,
    Read,
    Edit,
    Explore,
    SwitchMode,
    Search,
    Execute,
    Goal,
    /// node-repl 优先于通用 MCP（:121-124）。
    NodeRepl,
    /// 兜底（:170）。
    Fallback,
}

/// 工作流工具的按名判定（真源 `workflowToolNames.ts:43-81`，九个判定）。
fn workflow_renderer(tool_name: &str) -> Option<Renderer> {
    match normalize_tool_token(tool_name).as_str() {
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

/// 从 `toolCall.raw` 读工具名（真源 `readRawToolName`，被 `collectToolNames` 用）。
///
/// `toolCall.raw` 是透传的原始载荷，工具名可能落在 `toolName` / `tool_name` / `name`。
fn read_raw_tool_name(raw: &Value) -> Option<String> {
    for key in ["toolName", "tool_name", "name"] {
        if let Some(v) = raw.get(key).and_then(|v| v.as_str()) {
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// `collectToolNames`（真源 `cua.tsx:85-89`）：
/// `[toolCall.toolName, toolCall.kind, toolCall.title, readRawToolName(toolCall.raw)]`。
pub fn collect_tool_names(tool_name: &str, kind: &str, title: &str, raw: &Value) -> Vec<String> {
    let mut names = Vec::new();
    for candidate in [Some(tool_name), Some(kind), Some(title)] {
        if let Some(c) = candidate {
            if !c.is_empty() {
                names.push(c.to_string());
            }
        }
    }
    if let Some(n) = read_raw_tool_name(raw) {
        names.push(n);
    }
    names
}

/// `isCuaToolCall`（真源 `cua.tsx:332-336`）：四个名字里任一命中 CUA 工具名集合。
pub fn is_cua_tool_call(
    tool_name: &str,
    kind: &str,
    title: &str,
    raw: &Value,
) -> bool {
    collect_tool_names(tool_name, kind, title, raw)
        .iter()
        .any(|n| is_cua_tool_name(n))
}

/// 分流入口（真源 `resolveToolCallRenderer`，:57-173）。
///
/// 参数对应 `ToolCallBlockRenderContext` 里本函数真正用到的字段。
pub fn resolve_tool_call_renderer(
    tool_name: &str,
    kind: &str,
    title: &str,
    raw: &Value,
    has_mcp_presentation: bool,
) -> Renderer {
    // ── ① kind 聚合类（:58-68）──
    if kind == "changesGroup" {
        return Renderer::ChangesGroup;
    }
    if kind == "executeGroup" {
        return Renderer::ExecuteGroup;
    }
    if kind == "cuaGroup" {
        return Renderer::CuaGroup;
    }
    if is_cua_tool_call(tool_name, kind, title, raw) {
        return Renderer::Cua;
    }

    // ── ② 工作流按名（:73-113），必须先于 family ──
    if let Some(r) = workflow_renderer(tool_name) {
        return r;
    }

    let identity_family = family_by_lower(tool_name);

    // ── ③ node-repl 优先于通用 MCP（:117-124）──
    // 真源注释：宿主 Node REPL 也通过 MCP 注册，通用 MCP 分流会吞掉
    // 代码、错误栈和 artifact 等专用交互，所以先保住 node-repl。
    if identity_family == ToolFamily::NodeRepl {
        return Renderer::NodeRepl;
    }
    if has_mcp_presentation {
        return Renderer::Mcp;
    }

    // ── ④ family switch（:130-170）──
    match identity_family {
        ToolFamily::PlanGuidance => Renderer::PlanGuidance,
        ToolFamily::Agent => Renderer::Agent,
        ToolFamily::Todo => Renderer::Todo,
        ToolFamily::AskUserQuestion => Renderer::AskQuestion,
        // 真源 :141-143 —— message family 里 RespondToCoordinator 卡面不同。
        ToolFamily::Message => {
            if tool_name == "RespondToCoordinator" {
                Renderer::RespondToCoordinator
            } else {
                Renderer::SendMessage
            }
        }
        // 真源 :145
        ToolFamily::TaskControl => {
            if tool_name == "TaskOutput" {
                Renderer::TaskOutput
            } else {
                Renderer::TaskStop
            }
        }
        ToolFamily::Skill => Renderer::Skill,
        // 真源 :151-156 —— workflow family 内按工具名分派，
        // 工作流两个工具卡面完全不同（一个是脚本/图，一个是 actor 提交的结果）。
        ToolFamily::Workflow => {
            if tool_name == "submit_result" {
                Renderer::SubmitResult
            } else {
                Renderer::CreateWorkflow
            }
        }
        ToolFamily::SessionContext => Renderer::ReadSessionContext,
        ToolFamily::FileRead => Renderer::Read,
        ToolFamily::FileWrite => Renderer::Edit,
        ToolFamily::Explore => Renderer::Explore,
        ToolFamily::SwitchMode => Renderer::SwitchMode,
        ToolFamily::Search => Renderer::Search,
        ToolFamily::Shell => Renderer::Execute,
        ToolFamily::Goal => Renderer::Goal,
        ToolFamily::NodeRepl => Renderer::NodeRepl,
        ToolFamily::Unknown => Renderer::Fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn empty_raw() -> Value {
        json!({})
    }

    #[test]
    fn family_table_matches_source() {
        // 真源 tool-identity.ts:57-92 逐条对齐。
        use super::ToolFamily::*;
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
        for (name, family) in cases {
            assert_eq!(family_by_lower(name), *family, "{name} 应映射到 {family:?}");
        }
    }

    #[test]
    fn family_lookup_is_case_insensitive() {
        // 真源 TOOL_NAME_BY_LOWER（:94-107）。
        for n in ["read", "READ", "ReAd"] {
            assert_eq!(family_by_lower(n), ToolFamily::FileRead);
        }
    }

    #[test]
    fn workflow_dispatch_precedes_family() {
        // ★关键顺序：这些名字不在已知工具表里（→unknown→fallback），
        // 按名先判定让它们拿到专用卡，而不是被 workflow family 兜底成 CreateWorkflow。
        assert_eq!(
            resolve_tool_call_renderer("SaveWorkflow", "SaveWorkflow", "", &empty_raw(), false),
            Renderer::SaveWorkflow
        );
        assert_eq!(
            resolve_tool_call_renderer("ListModels", "ListModels", "", &empty_raw(), false),
            Renderer::ListModels
        );
        // 反证：CreateWorkflow 走 family 兜底。
        assert_eq!(
            resolve_tool_call_renderer("CreateWorkflow", "CreateWorkflow", "", &empty_raw(), false),
            Renderer::CreateWorkflow
        );
    }

    #[test]
    fn kind_aggregation_wins_first() {
        // 真源 :58-68 —— kind 聚合类不看工具名。
        assert_eq!(
            resolve_tool_call_renderer("Bash", "executeGroup", "", &empty_raw(), false),
            Renderer::ExecuteGroup
        );
        assert_eq!(
            resolve_tool_call_renderer("Read", "changesGroup", "", &empty_raw(), false),
            Renderer::ChangesGroup
        );
        assert_eq!(
            resolve_tool_call_renderer("x", "cuaGroup", "", &empty_raw(), false),
            Renderer::CuaGroup
        );
    }

    #[test]
    fn node_repl_survives_mcp_dispatch() {
        // 真源 :117-124 —— 通用 MCP 分流会吞掉 node-repl 的代码/错误栈/artifact交互。
        assert_eq!(
            resolve_tool_call_renderer("mcp__node_repl__js", "js", "", &empty_raw(), true),
            Renderer::NodeRepl,
            "node-repl 不能被通用 MCP 卡吞掉"
        );
        // 其余 MCP 走通用展示。
        assert_eq!(
            resolve_tool_call_renderer("mcp__other", "other", "", &empty_raw(), true),
            Renderer::Mcp
        );
    }

    #[test]
    fn cua_tool_name_matches_both_namespaces() {
        // 真源 cuaPermissionAction.ts:7-16 —— 两种命名空间都要命中。
        assert!(is_cua_tool_name("computer-use"));
        assert!(is_cua_tool_name("mcp__computer-use__click"));
        assert!(is_cua_tool_name("mcp__plugin_zcode-cua_computer-use__screenshot"));
        assert!(is_cua_tool_name("COMPUTER_USE"), "下划线归一为连字符");
        // 不会误判 android-emulator / browser-use。
        assert!(!is_cua_tool_name("android-emulator"));
        assert!(!is_cua_tool_name("browser-use"));
        assert!(!is_cua_tool_name("Bash"));
    }

    #[test]
    fn cua_detected_from_any_of_four_names() {
        // 真源 collectToolNames（cua.tsx:85-89）看四个位置：toolName/kind/title/raw。
        // raw 里的工具名也要能命中。
        let raw = json!({ "toolName": "mcp__computer-use__click" });
        assert!(is_cua_tool_call("Unknown", "Unknown", "", &raw));
        assert!(is_cua_tool_call("Unknown", "computer-use", "", &empty_raw()));
        assert!(is_cua_tool_call("Unknown", "Unknown", "computer-use", &empty_raw()));
        // 无 CUA 特征时不误判。
        assert!(!is_cua_tool_call("Bash", "shell", "", &empty_raw()));
    }

    #[test]
    fn normalize_functions_differ_intentionally() {
        // 两个归一化刻意不同，别混用：
        // - workflowToolNames 抹掉**所有**非字母数字（含连字符）
        // - cuaPermissionAction 只把下划线换成连字符，连字符保留
        assert_eq!(normalize_tool_token("save_workflow"), "saveworkflow");
        assert_eq!(normalize_cua_tool_name("COMPUTER_USE"), "computer-use");
        // 关键差异：连字符在前者被抹掉、后者保留。
        assert_eq!(normalize_tool_token("computer-use"), "computeruse");
        assert_eq!(normalize_cua_tool_name("computer-use"), "computer-use");
        // 正因为后者保留连字符，CUA 判定才能用 includes 兼容 plugin namespace 前缀。
        assert!(is_cua_tool_name("mcp__plugin_zcode-cua_computer-use__x"));
    }

    #[test]
    fn message_family_splits_by_tool_name() {
        // 真源 :141-143
        assert_eq!(
            resolve_tool_call_renderer("RespondToCoordinator", "x", "", &empty_raw(), false),
            Renderer::RespondToCoordinator
        );
        assert_eq!(
            resolve_tool_call_renderer("SendMessage", "x", "", &empty_raw(), false),
            Renderer::SendMessage
        );
    }

    #[test]
    fn task_control_family_splits_by_tool_name() {
        // 真源 :145
        assert_eq!(
            resolve_tool_call_renderer("TaskOutput", "x", "", &empty_raw(), false),
            Renderer::TaskOutput
        );
        assert_eq!(
            resolve_tool_call_renderer("TaskStop", "x", "", &empty_raw(), false),
            Renderer::TaskStop
        );
    }

    #[test]
    fn workflow_family_splits_by_submit_result() {
        // 真源 :151-156
        assert_eq!(
            resolve_tool_call_renderer("submit_result", "x", "", &empty_raw(), false),
            Renderer::SubmitResult
        );
        assert_eq!(
            resolve_tool_call_renderer("CreateWorkflow", "x", "", &empty_raw(), false),
            Renderer::CreateWorkflow
        );
    }

    #[test]
    fn todo_write_is_not_misread_as_file_write() {
        // 真源 :126-129 注释：继续用正则扫 kind/title 会把 TodoWrite 里的 Write
        // 当成文件写入。查表法应正确归到 todo。
        assert_eq!(
            resolve_tool_call_renderer("TodoWrite", "TodoWrite", "", &empty_raw(), false),
            Renderer::Todo
        );
    }

    #[test]
    fn unknown_tool_falls_back() {
        // 真源 :170 default
        assert_eq!(
            resolve_tool_call_renderer("SomeRandomTool", "x", "", &empty_raw(), false),
            Renderer::Fallback
        );
    }

    #[test]
    fn every_registered_family_has_a_renderer() {
        // 真源 :130-170 每个 family 分支都要有落点，漏一个就会掉进 default。
        let cases: &[(&str, Renderer)] = &[
            ("Read", Renderer::Read),
            ("Edit", Renderer::Edit),
            ("Write", Renderer::Edit),
            ("Bash", Renderer::Execute),
            ("Grep", Renderer::Search),
            ("TodoWrite", Renderer::Todo),
            ("Agent", Renderer::Agent),
            ("Skill", Renderer::Skill),
            ("GoalRead", Renderer::Goal),
            ("ReadSessionContext", Renderer::ReadSessionContext),
            ("AskUserQuestion", Renderer::AskQuestion),
            ("SendMessage", Renderer::SendMessage),
            ("TaskOutput", Renderer::TaskOutput),
            ("js", Renderer::NodeRepl),
            ("CreateWorkflow", Renderer::CreateWorkflow),
        ];
        for (tool, expected) in cases {
            assert_eq!(
                resolve_tool_call_renderer(tool, tool, "", &empty_raw(), false),
                *expected,
                "{tool} 应落到 {expected:?}"
            );
        }
    }
}