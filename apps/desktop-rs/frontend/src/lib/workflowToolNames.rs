//! 1:1 翻译 `packages/ui/src/lib/workflowToolNames.ts`（99 行）。
//!
//! 工作流一族工具的**按名字**识别，三处原本各自一份同款匹配器的合并（真源 :1-15）：
//! - 可复用工作流：`SaveWorkflow` / `ListSavedWorkflows`；
//! - 升级问答：`escalate` / `ResolveWorkflowQuestion`；
//! - 观察与恢复：`GetWorkflowRun` / `ListWorkflowRuns` / `EvalWorkflowSnippet` / `ResumeWorkflowRun`；
//! - 模型目录：`ListModels`；创建与修订：`CreateWorkflow` / `AmendWorkflow`。
//!
//! 为什么不走 `resolveToolCallIdentity`（真源 :8-9）：这些工具名都不在
//! `packages/shared` 的 `ZCODE_KNOWN_TOOL_NAMES` 里，identity 对它们只会回 `unknown`，
//! 分流会掉进 raw JSON 兜底卡。
//!
//! 而且这些判定必须排在 family 分流**之前**（真源 :11-14）：`workflow` family 的兜底分支是
//! CreateWorkflow 卡与运行确认块，一旦有人把这些名字登记进 workflow family，保存与列举就会
//! 静默渲染成「创建工作流」。按名字先判定让登记前后两种世界都成立。
//!
//! ★入参形态偏离：真源四个判定入参是一个 `WorkflowToolNameSource` 对象（字段全 optional），
//! Rust 侧沿用本仓库 `is_cua_tool_call` 的既有形状 `(tool_name, kind, title, raw)`，
//! 缺省字段以空串表示。`normalize_tool_token("")` 得 `""`，而所有 token 都非空，
//! 故与真源的 `undefined` 字段行为等价。

use serde_json::Value;

/// `normalizeToolToken`（真源 :21-25）：照 cron-create.tsx 的同款归一——
/// 抹掉大小写与分隔符，`SaveWorkflow` / `save_workflow` 两种 wire 写法都命中。
///
/// 真源 `value.toLowerCase().replace(/[^a-z0-9]/gu, "")`，非字符串入参回 `""`。
pub fn normalize_tool_token(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        .collect()
}

/// `isPlainRecord`（真源 :17-19）：对象、非 null、非数组。
/// serde_json 里 `Number`/`Bool`/`Null`/`Array` 都不是 object，判定一致。
fn is_plain_record(value: &Value) -> bool {
    value.is_object()
}

/// 真源把非字符串的 raw 字段交给 `normalizeToolToken` 后得 `""`，
/// 这里读不出字符串时同样给 `""`，结果等价。
fn raw_token(raw: &Value, key: &str) -> String {
    raw.get(key)
        .and_then(|v| v.as_str())
        .map(normalize_tool_token)
        .unwrap_or_default()
}

/// `matchesToolName`（真源 :34-41）：`[toolName, kind, title, raw.toolName, raw.tool_name,
/// raw.name]` 六个候选里任一归一后等于 token。`raw` 不是 plain record 时后三个不参与
/// （真源 :35-37 的三元）。
pub fn matches_tool_name(tool_name: &str, kind: &str, title: &str, raw: &Value, token: &str) -> bool {
    if normalize_tool_token(tool_name) == token
        || normalize_tool_token(kind) == token
        || normalize_tool_token(title) == token
    {
        return true;
    }
    if !is_plain_record(raw) {
        return false;
    }
    ["toolName", "tool_name", "name"]
        .iter()
        .any(|key| raw_token(raw, key) == token)
}

macro_rules! workflow_name_predicate {
    ($fname:ident, $token:literal, $doc:literal) => {
        #[doc = $doc]
        pub fn $fname(tool_name: &str, kind: &str, title: &str, raw: &Value) -> bool {
            matches_tool_name(tool_name, kind, title, raw, $token)
        }
    };
}

workflow_name_predicate!(is_save_workflow_tool_call, "saveworkflow", "`isSaveWorkflowToolCall`（真源 :43-45）。");
workflow_name_predicate!(
    is_list_saved_workflows_tool_call,
    "listsavedworkflows",
    "`isListSavedWorkflowsToolCall`（真源 :47-49）。"
);
workflow_name_predicate!(
    is_escalate_tool_call,
    "escalate",
    "`isEscalateToolCall`（真源 :51-53）：子代理提问。与 `ResolveWorkflowQuestion`（主代理作答）
     卡面完全不同，各认自己的名字。"
);
workflow_name_predicate!(
    is_resolve_workflow_question_tool_call,
    "resolveworkflowquestion",
    "`isResolveWorkflowQuestionToolCall`（真源 :55-57）。"
);
workflow_name_predicate!(is_get_workflow_run_tool_call, "getworkflowrun", "`isGetWorkflowRunToolCall`（真源 :59-61）。");
workflow_name_predicate!(
    is_list_workflow_runs_tool_call,
    "listworkflowruns",
    "`isListWorkflowRunsToolCall`（真源 :63-65）。"
);
workflow_name_predicate!(
    is_eval_workflow_snippet_tool_call,
    "evalworkflowsnippet",
    "`isEvalWorkflowSnippetToolCall`（真源 :67-69）。"
);
workflow_name_predicate!(
    is_resume_workflow_run_tool_call,
    "resumeworkflowrun",
    "`isResumeWorkflowRunToolCall`（真源 :71-73）。"
);
workflow_name_predicate!(
    is_list_models_tool_call,
    "listmodels",
    "`isListModelsToolCall`（真源 :75-81）：模型目录。同款按名判定——`ListModels` 也不在已知
     工具表里，兜底卡会把那段以 providerId 开头的模型面文本原样摊开。"
);
workflow_name_predicate!(
    is_create_workflow_tool_call,
    "createworkflow",
    "`isCreateWorkflowToolCall`（真源 :83-90）：创建入口。渲染分流仍走 `workflow` family
     （它的兜底就是创建卡）；这个按名判定只给不经过分流的读者用——编译反馈的稿号联接要在
     行窗口里认出每一次创建，而 family 会把修订也算进来。"
);
workflow_name_predicate!(
    is_amend_workflow_tool_call,
    "amendworkflow",
    "`isAmendWorkflowToolCall`（真源 :92-99）：修订入口。它**已登记**进 workflow family
     （确认窗按 family 选运行确认块），但工具行仍按名先判：同一个 create-workflow 渲染器换
     一套修订词汇，而不是让 family 兜底把它画成一张普通的「创建工作流」卡。"
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn empty_raw() -> Value {
        json!({})
    }

    #[test]
    fn token_normalization_drops_case_and_separators() {
        // 真源 :24 —— 大小写与所有非 a-z0-9 分隔符一起抹掉。
        assert_eq!(normalize_tool_token("SaveWorkflow"), "saveworkflow");
        assert_eq!(normalize_tool_token("save_workflow"), "saveworkflow");
        assert_eq!(normalize_tool_token("mcp__save-workflow__v2"), "mcpsaveworkflowv2");
        assert_eq!(normalize_tool_token(""), "");
    }

    #[test]
    fn matches_any_of_the_six_positions() {
        // 真源 :38-40 —— toolName / kind / title / raw 的三个键，任一命中即可。
        assert!(matches_tool_name("SaveWorkflow", "", "", &empty_raw(), "saveworkflow"));
        assert!(matches_tool_name("", "save_workflow", "", &empty_raw(), "saveworkflow"));
        assert!(matches_tool_name("", "", "SaveWorkflow", &empty_raw(), "saveworkflow"));
        assert!(matches_tool_name(
            "Unknown",
            "",
            "",
            &json!({ "toolName": "SaveWorkflow" }),
            "saveworkflow"
        ));
        assert!(matches_tool_name(
            "Unknown",
            "",
            "",
            &json!({ "tool_name": "SaveWorkflow" }),
            "saveworkflow"
        ));
        assert!(matches_tool_name("Unknown", "", "", &json!({ "name": "SaveWorkflow" }), "saveworkflow"));
        // 都不命中。
        assert!(!matches_tool_name("Bash", "shell", "ls", &json!({ "toolName": "Bash" }), "saveworkflow"));
    }

    #[test]
    fn non_record_raw_contributes_nothing() {
        // 真源 :35-37 —— raw 不是 plain record 时 rawNames 是空数组。
        // 数组与数字都可能带可索引的 name，这里必须一律忽略。
        assert!(!matches_tool_name("x", "", "", &json!(["SaveWorkflow"]), "saveworkflow"));
        assert!(!matches_tool_name("x", "", "", &json!("SaveWorkflow"), "saveworkflow"));
        assert!(!matches_tool_name("x", "", "", &Value::Null, "saveworkflow"));
        // 数组里的 name 字段形状（真源同样不读）。
        assert!(!matches_tool_name("x", "", "", &json!([{ "name": "SaveWorkflow" }]), "saveworkflow"));
    }

    #[test]
    fn non_string_raw_field_is_not_a_match() {
        // 真源把非字符串交给 normalizeToolToken 得 ""，不会误命中。
        assert!(!matches_tool_name("x", "", "", &json!({ "name": 42 }), "saveworkflow"));
        assert!(!matches_tool_name("x", "", "", &json!({ "name": null }), "saveworkflow"));
        // ★空字符串同样不命中（token 恒非空）。
        assert!(!matches_tool_name("", "", "", &empty_raw(), "saveworkflow"));
    }

    #[test]
    fn every_predicate_has_its_own_token() {
        // 真源 :43-99 的 11 个谓词，各自只认自己的名字。
        let cases: &[(&str, fn(&str, &str, &str, &Value) -> bool)] = &[
            ("SaveWorkflow", is_save_workflow_tool_call),
            ("ListSavedWorkflows", is_list_saved_workflows_tool_call),
            ("escalate", is_escalate_tool_call),
            ("ResolveWorkflowQuestion", is_resolve_workflow_question_tool_call),
            ("GetWorkflowRun", is_get_workflow_run_tool_call),
            ("ListWorkflowRuns", is_list_workflow_runs_tool_call),
            ("EvalWorkflowSnippet", is_eval_workflow_snippet_tool_call),
            ("ResumeWorkflowRun", is_resume_workflow_run_tool_call),
            ("ListModels", is_list_models_tool_call),
            ("CreateWorkflow", is_create_workflow_tool_call),
            ("AmendWorkflow", is_amend_workflow_tool_call),
        ];
        for (name, predicate) in cases {
            assert!(
                predicate(name, "", "", &empty_raw()),
                "{name} 应被自己的谓词认领"
            );
        }
        // 交叉不误判：ListWorkflowRuns 不是 GetWorkflowRun。
        assert!(!is_get_workflow_run_tool_call("ListWorkflowRuns", "", "", &empty_raw()));
        assert!(!is_list_workflow_runs_tool_call("GetWorkflowRun", "", "", &empty_raw()));
        // ResumeWorkflowRun 不是 SaveWorkflow。
        assert!(!is_save_workflow_tool_call("ResumeWorkflowRun", "", "", &empty_raw()));
        // AmendWorkflow 不是 CreateWorkflow（family 会把两者都算进 workflow，按名必须分得开）。
        assert!(is_amend_workflow_tool_call("AmendWorkflow", "", "", &empty_raw()));
        assert!(!is_create_workflow_tool_call("AmendWorkflow", "", "", &empty_raw()));
    }

    #[test]
    fn workflow_names_are_not_known_tool_identity_names() {
        // 真源 :8-9 的理由：这些名字不在已知工具表里，所以 identity 只会回 unknown，
        // 才必须按名判定。已知表里出现的（CreateWorkflow / AmendWorkflow 已登记）另说。
        for name in [
            "SaveWorkflow",
            "ListSavedWorkflows",
            "GetWorkflowRun",
            "ListWorkflowRuns",
            "EvalWorkflowSnippet",
            "ResumeWorkflowRun",
            "ListModels",
            "escalate",
            "ResolveWorkflowQuestion",
        ] {
            assert_eq!(
                crate::ToolCallBlocks::resolveRenderer::family_by_lower(name),
                crate::ToolCallBlocks::resolveRenderer::ToolFamily::Unknown,
                "{name} 不应在已知工具表里（否则按名判定的理由就变了）"
            );
        }
    }
}
