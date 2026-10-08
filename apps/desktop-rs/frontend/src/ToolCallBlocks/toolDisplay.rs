//! 1:1 翻译 `packages/ui/src/lib/toolDisplay.ts`（335 行）。
//!
//! 工具展示模型：把「预览源 + 工具身份 + 错误」收敛成通用模型
//! （inlinePreview / planResult / viewerSource / showInput / showOutput /
//! showKind），再由 **strategy 表**按 family 增强。
//!
//! 真源注释（:305-308）：tool 展示之前靠 `kind === "edit"` 直接分叉，
//! 预览提取层已能识别 read/replace/image 但渲染层吃不到。这里收敛成
//! 「通用模型 + kind 策略增强」——新增 execute/search/fetch 专用展示
//! 只需追加策略，不重写主渲染骨架。

use serde_json::Value;

use super::codeViewer::{
    CodeViewerSource, CodeViewerToolCall, ImageCodeViewerSource, PatchCodeViewerSource,
    TextCodeViewerSource, get_tool_call_code_content_preview, get_tool_call_code_preview,
};
use super::renderers::planToolCall::{is_absolute_file_path, join_file_path};
use super::resolveRenderer::ToolFamily;
use super::toolError::get_tool_call_error_text;
use super::toolIdentity::{
    ToolCallIdentity, is_file_content_write_tool_call, is_file_diff_tool_call,
    resolve_tool_call_identity,
};

/// `ToolInlinePreview`（真源 :19-23）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolInlinePreview {
    None,
    Text(TextCodeViewerSource),
    Patch(PatchCodeViewerSource),
    Image(ImageCodeViewerSource),
}

impl ToolInlinePreview {
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// `ToolPlanResult`（真源 :25-28）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPlanResult {
    pub plan: String,
    pub plan_file_path: Option<String>,
}

/// `viewerLabelId`（真源 :34）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerLabelId {
    ViewDiff,
    ViewCode,
}

/// `ToolDisplayModel`（真源 :30-38）。
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDisplayModel {
    pub inline_preview: ToolInlinePreview,
    pub plan_result: Option<ToolPlanResult>,
    pub viewer_source: Option<CodeViewerSource>,
    pub viewer_label_id: ViewerLabelId,
    pub show_summary_file_link: bool,
    pub show_input: bool,
    pub show_output: bool,
    pub show_kind: bool,
}

/// `ToolDisplayContext`（真源 :40-46）。
#[derive(Debug, Clone)]
pub struct ToolDisplayContext {
    pub tool_call: CodeViewerToolCall,
    pub identity: ToolCallIdentity,
    pub preview: Option<CodeViewerSource>,
    pub content_preview: Option<TextCodeViewerSource>,
    pub error_text: Option<String>,
}

/// `Partial<ToolDisplayModel>`（真源 strategy `build` 的返回）。
#[derive(Debug, Clone, Default)]
struct PartialToolDisplayModel {
    inline_preview: Option<ToolInlinePreview>,
    show_summary_file_link: Option<bool>,
    show_input: Option<bool>,
    show_output: Option<bool>,
    show_kind: Option<bool>,
}

impl ToolDisplayModel {
    fn apply(&mut self, partial: PartialToolDisplayModel) {
        if let Some(v) = partial.inline_preview {
            self.inline_preview = v;
        }
        if let Some(v) = partial.show_summary_file_link {
            self.show_summary_file_link = v;
        }
        if let Some(v) = partial.show_input {
            self.show_input = v;
        }
        if let Some(v) = partial.show_output {
            self.show_output = v;
        }
        if let Some(v) = partial.show_kind {
            self.show_kind = v;
        }
    }
}

/// `ToolDisplayStrategy`（真源 :48-51）。
struct ToolDisplayStrategy {
    matches: fn(&ToolDisplayContext) -> bool,
    build: fn(&ToolDisplayContext) -> PartialToolDisplayModel,
}

/// `isRecord`（真源 :54-56）——严格版（排除数组）。
fn is_record(value: &Value) -> bool {
    value.is_object()
}

/// `extractToolPlanResultFromValue`（真源 :58-80）。
pub fn extract_tool_plan_result_from_value(
    value: &Value,
    workspace_path: &str,
) -> Option<ToolPlanResult> {
    if !is_record(value) {
        return None;
    }
    let raw_plan = value.get("plan").and_then(|v| v.as_str())?;
    if raw_plan.trim().is_empty() {
        return None;
    }
    let raw_plan_file_path = value.get("planFilePath").and_then(|v| v.as_str());
    let plan_file_path = raw_plan_file_path
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            if is_absolute_file_path(p) {
                p.to_string()
            } else {
                join_file_path(workspace_path, p)
            }
        });

    Some(ToolPlanResult {
        plan: raw_plan.trim().to_string(),
        plan_file_path,
    })
}

/// `toInlinePreview`（真源 :82-112）。
fn to_inline_preview(context: &ToolDisplayContext, prefer_patch: bool) -> ToolInlinePreview {
    if prefer_patch {
        if let Some(CodeViewerSource::Patch(p)) = &context.preview {
            return ToolInlinePreview::Patch(p.clone());
        }
    }
    if let Some(CodeViewerSource::Image(i)) = &context.preview {
        return ToolInlinePreview::Image(i.clone());
    }
    if let Some(CodeViewerSource::Text(t)) = &context.preview {
        return ToolInlinePreview::Text(t.clone());
    }
    if let Some(content_preview) = &context.content_preview {
        return ToolInlinePreview::Text(content_preview.clone());
    }
    ToolInlinePreview::None
}

/// `diffToolStrategy`（真源 :114-128）。
fn diff_tool_matches(context: &ToolDisplayContext) -> bool {
    let like = context.tool_call.identity_like();
    is_file_diff_tool_call(&like, Some(&context.identity))
}

fn diff_tool_build(context: &ToolDisplayContext) -> PartialToolDisplayModel {
    let inline_preview = to_inline_preview(context, true);
    let has_inline_preview = !inline_preview.is_none();
    PartialToolDisplayModel {
        inline_preview: Some(inline_preview),
        show_input: Some(!has_inline_preview),
        show_output: Some(context.error_text.is_some()),
        show_kind: Some(!has_inline_preview),
        ..Default::default()
    }
}

/// `readToolStrategy`（真源 :130-151）。
fn read_tool_matches(context: &ToolDisplayContext) -> bool {
    context.identity.family == ToolFamily::FileRead
}

fn read_tool_build(context: &ToolDisplayContext) -> PartialToolDisplayModel {
    let inline_preview = to_inline_preview(context, false);
    let has_inline_preview = !inline_preview.is_none();
    PartialToolDisplayModel {
        inline_preview: Some(inline_preview),
        // 真源 :143-145 —— 读类工具标题已含目标文件，摘要行再补文件名会重复；
        // 只保留标题和正文预览。
        show_summary_file_link: Some(false),
        show_input: Some(!has_inline_preview),
        show_output: Some(context.error_text.is_some()),
        show_kind: Some(!has_inline_preview),
        ..Default::default()
    }
}

/// `writeToolStrategy`（真源 :153-172）。
fn write_tool_matches(context: &ToolDisplayContext) -> bool {
    let like = context.tool_call.identity_like();
    is_file_content_write_tool_call(&like, Some(&context.identity))
}

fn write_tool_build(context: &ToolDisplayContext) -> PartialToolDisplayModel {
    let inline_preview = to_inline_preview(context, false);
    let has_inline_preview = !inline_preview.is_none();
    PartialToolDisplayModel {
        inline_preview: Some(inline_preview),
        show_input: Some(!has_inline_preview),
        // 真源 :164-166 —— Write 的成功 output 常只是结构化确认结果；
        // 写入内容已由 inlinePreview / 文件摘要承载，只在失败时保留错误。
        show_output: Some(context.error_text.is_some()),
        show_kind: Some(!has_inline_preview),
        ..Default::default()
    }
}

/// `genericImageStrategy`（真源 :174-190）。
fn generic_image_matches(context: &ToolDisplayContext) -> bool {
    matches!(context.preview, Some(CodeViewerSource::Image(_)))
}

fn generic_image_build(context: &ToolDisplayContext) -> PartialToolDisplayModel {
    let inline_preview = to_inline_preview(context, false);
    let has_inline_preview = !inline_preview.is_none();
    PartialToolDisplayModel {
        inline_preview: Some(inline_preview),
        show_input: Some(!has_inline_preview),
        show_output: Some(context.error_text.is_some()),
        show_kind: Some(!has_inline_preview),
        ..Default::default()
    }
}

/// `executeToolStrategy`（真源 :192-203）。
fn execute_tool_matches(context: &ToolDisplayContext) -> bool {
    context.identity.family == ToolFamily::Shell
}

fn execute_tool_build(context: &ToolDisplayContext) -> PartialToolDisplayModel {
    PartialToolDisplayModel {
        inline_preview: Some(ToolInlinePreview::None),
        show_input: Some(false),
        show_output: Some(
            !matches!(context.tool_call.output, Value::Null) || context.error_text.is_some(),
        ),
        show_kind: Some(false),
        ..Default::default()
    }
}

/// `searchToolStrategy`（真源 :205-222）。
fn search_tool_matches(context: &ToolDisplayContext) -> bool {
    context.identity.family == ToolFamily::Search
}

fn search_tool_build(context: &ToolDisplayContext) -> PartialToolDisplayModel {
    // 真源 :211-216 —— search/fetch 的输入只是 query/路径/过滤条件，
    // 用户只关心结果；统一只保留 result/error，且摘要行不再补文件链接。
    PartialToolDisplayModel {
        inline_preview: Some(ToolInlinePreview::None),
        show_summary_file_link: Some(false),
        show_input: Some(false),
        show_output: Some(
            !matches!(context.tool_call.output, Value::Null) || context.error_text.is_some(),
        ),
        show_kind: Some(false),
        ..Default::default()
    }
}

/// `goalToolStrategy`（真源 :224-240）。
fn goal_tool_matches(context: &ToolDisplayContext) -> bool {
    context.identity.family == ToolFamily::Goal
}

fn goal_tool_build(context: &ToolDisplayContext) -> PartialToolDisplayModel {
    // 真源 :229-231 —— Goal 的 input 是模型给 runtime 的状态变更参数，
    // 不是用户要读的结果；通用 fallback 会把 goal 状态淹没在噪音里。
    PartialToolDisplayModel {
        inline_preview: Some(ToolInlinePreview::None),
        show_summary_file_link: Some(false),
        show_input: Some(false),
        show_output: Some(
            !matches!(context.tool_call.output, Value::Null) || context.error_text.is_some(),
        ),
        show_kind: Some(false),
        ..Default::default()
    }
}

/// `nodeReplToolStrategy`（真源 :242-258）。
fn node_repl_tool_matches(context: &ToolDisplayContext) -> bool {
    context.identity.family == ToolFamily::NodeRepl
}

fn node_repl_tool_build(_context: &ToolDisplayContext) -> PartialToolDisplayModel {
    // 真源 :247-249 —— 展示语义由专用 renderer 归一化；通用
    // Parameters/Result/kind 会暴露实现细节并重复，故全部关闭。
    PartialToolDisplayModel {
        inline_preview: Some(ToolInlinePreview::None),
        show_summary_file_link: Some(false),
        show_input: Some(false),
        show_output: Some(false),
        show_kind: Some(false),
        ..Default::default()
    }
}

/// `TOOL_DISPLAY_STRATEGIES`（真源 :260-269）——**顺序即匹配优先级**。
const TOOL_DISPLAY_STRATEGIES: [ToolDisplayStrategy; 8] = [
    ToolDisplayStrategy {
        matches: diff_tool_matches,
        build: diff_tool_build,
    },
    ToolDisplayStrategy {
        matches: read_tool_matches,
        build: read_tool_build,
    },
    ToolDisplayStrategy {
        matches: write_tool_matches,
        build: write_tool_build,
    },
    ToolDisplayStrategy {
        matches: execute_tool_matches,
        build: execute_tool_build,
    },
    ToolDisplayStrategy {
        matches: search_tool_matches,
        build: search_tool_build,
    },
    ToolDisplayStrategy {
        matches: goal_tool_matches,
        build: goal_tool_build,
    },
    ToolDisplayStrategy {
        matches: node_repl_tool_matches,
        build: node_repl_tool_build,
    },
    ToolDisplayStrategy {
        matches: generic_image_matches,
        build: generic_image_build,
    },
];

/// `buildToolDisplayModel`（真源 :268-335）。
///
/// `tool_error` / `status` 对应真源 `getToolCallErrorText(toolCall)` 的
/// `toolCall.error` 与 `toolCall.status`（v4 载荷分列传入）。
pub fn build_tool_display_model(
    tool_call: &CodeViewerToolCall,
    tool_error: Option<&str>,
    status: &str,
    workspace_path: &str,
) -> ToolDisplayModel {
    let preview = get_tool_call_code_preview(tool_call, workspace_path);
    let content_preview = get_tool_call_code_content_preview(tool_call, workspace_path);
    let like = tool_call.identity_like();
    let error_text =
        get_tool_call_error_text(tool_error, &tool_call.output, &tool_call.raw, status);
    let identity = resolve_tool_call_identity(&like);
    // 真源 :274-277 —— 用户要看的 plan 来自 tool result，不是 tool input；
    // EnterPlanMode 输入里的 plan/todo 结构不能兜底读，避免与顶部 plan 事件职责混淆。
    let plan_result = extract_tool_plan_result_from_value(&tool_call.output, workspace_path);
    let context = ToolDisplayContext {
        tool_call: tool_call.clone(),
        identity,
        preview,
        content_preview,
        error_text,
    };

    let viewer_label_id = if matches!(context.preview, Some(CodeViewerSource::Patch(_))) {
        ViewerLabelId::ViewDiff
    } else {
        ViewerLabelId::ViewCode
    };
    // 真源 :287 —— `showSummaryFileLink: Boolean(preview?.path)`（不分类型）。
    let show_summary_file_link = preview_path_present(&context.preview);

    let mut model = ToolDisplayModel {
        inline_preview: ToolInlinePreview::None,
        plan_result: plan_result.clone(),
        viewer_source: context.preview.clone(),
        viewer_label_id,
        show_summary_file_link,
        show_input: if plan_result.is_some() {
            false
        } else {
            !matches!(tool_call.input, Value::Null)
        },
        show_output: !matches!(tool_call.output, Value::Null) || context.error_text.is_some(),
        show_kind: true,
    };

    if let Some(strategy) = TOOL_DISPLAY_STRATEGIES
        .iter()
        .find(|strategy| (strategy.matches)(&context))
    {
        model.apply((strategy.build)(&context));
    }

    // 真源 :310-318 —— 退出计划模式的 result 同时带 markdown plan 与
    // allowedPrompts 等结构化字段；优先把 plan 当专用结果块，只在出错时回退通用输出区。
    if plan_result.is_some() {
        let mut adjusted = model.clone();
        adjusted.show_input = false;
        adjusted.show_output = context.error_text.is_some();
        return adjusted;
    }

    if context.error_text.is_some() {
        // 真源 :320-331 —— edit/write 失败时继续展示 Parameters 会把
        // oldString/newString 整坨 JSON 顶上来，报错被挤下去；失败态收敛成错误视图。
        let mut adjusted = model;
        adjusted.inline_preview = ToolInlinePreview::None;
        adjusted.show_input = false;
        adjusted.show_output = true;
        adjusted.show_kind = false;
        return adjusted;
    }

    model
}

/// `Boolean(preview?.path)`（真源 :287）：preview 存在且有 path。
fn preview_path_present(preview: &Option<CodeViewerSource>) -> bool {
    match preview {
        Some(CodeViewerSource::File(f)) => !f.path.is_empty(),
        Some(CodeViewerSource::CodeReview(c)) => !c.path.is_empty(),
        Some(CodeViewerSource::Text(t)) => t.path.as_deref().is_some_and(|p| !p.is_empty()),
        Some(CodeViewerSource::Patch(p)) => p.path.as_deref().is_some_and(|p| !p.is_empty()),
        Some(CodeViewerSource::MultiFileDiff(m)) => {
            m.path.as_deref().is_some_and(|p| !p.is_empty())
        }
        Some(CodeViewerSource::Image(i)) => !i.path.is_empty(),
        Some(CodeViewerSource::Media(m)) => !m.path.is_empty(),
        Some(CodeViewerSource::Pdf(p)) => !p.path.is_empty(),
        Some(CodeViewerSource::Pptx(p)) => !p.path.is_empty(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tc(tool_name: &str, input: Value, output: Value, raw: Value) -> CodeViewerToolCall {
        CodeViewerToolCall {
            tool_name: Some(tool_name.to_string()),
            kind: Some(tool_name.to_string()),
            title: None,
            input,
            output,
            raw,
        }
    }

    #[test]
    fn plan_result_from_output_only() {
        // 绝对路径直用。
        let plan = extract_tool_plan_result_from_value(
            &json!({"plan": "  # 计划\n- 步骤 ", "planFilePath": "/abs/p.md"}),
            "/ws",
        )
        .unwrap();
        assert_eq!(plan.plan, "# 计划\n- 步骤", "plan trim");
        assert_eq!(plan.plan_file_path.as_deref(), Some("/abs/p.md"));
        // 相对路径 join。
        let plan = extract_tool_plan_result_from_value(
            &json!({"plan": "x", "planFilePath": "docs/p.md"}),
            "/ws",
        )
        .unwrap();
        assert_eq!(plan.plan_file_path.as_deref(), Some("/ws/docs/p.md"));
        // 空白 plan → None。
        assert_eq!(
            extract_tool_plan_result_from_value(&json!({"plan": "  "}), "/ws"),
            None
        );
    }

    #[test]
    fn edit_tool_uses_diff_strategy() {
        let input = json!({"file_path": "a.rs", "old_string": "old", "new_string": "new"});
        let model = build_tool_display_model(
            &tc("Edit", input, json!({}), json!({})),
            None,
            "completed",
            "/ws",
        );
        assert!(matches!(model.inline_preview, ToolInlinePreview::Patch(_)));
        assert_eq!(model.viewer_label_id, ViewerLabelId::ViewDiff);
        assert!(!model.show_input, "有预览 → 不展示参数");
        assert!(!model.show_output);
        assert!(!model.show_kind);
    }

    #[test]
    fn read_tool_uses_read_strategy() {
        let model = build_tool_display_model(
            &tc(
                "Read",
                json!({"file_path": "a.rs"}),
                json!("code"),
                json!({}),
            ),
            None,
            "completed",
            "/ws",
        );
        assert!(matches!(model.inline_preview, ToolInlinePreview::Text(_)));
        assert_eq!(model.viewer_label_id, ViewerLabelId::ViewCode);
        assert!(!model.show_summary_file_link, "读类工具不重复文件名");
        assert!(!model.show_input);
    }

    #[test]
    fn execute_and_search_hide_input() {
        let model = build_tool_display_model(
            &tc("Bash", json!({"command": "ls"}), json!("out"), json!({})),
            None,
            "completed",
            "/ws",
        );
        assert!(model.inline_preview.is_none());
        assert!(!model.show_input);
        assert!(model.show_output, "有 output");
        assert!(!model.show_kind);

        let model = build_tool_display_model(
            &tc("Grep", json!({"pattern": "x"}), json!("hits"), json!({})),
            None,
            "completed",
            "/ws",
        );
        assert!(!model.show_input);
        assert!(!model.show_summary_file_link);
    }

    #[test]
    fn node_repl_hides_output_too() {
        let model = build_tool_display_model(
            &tc("js", json!({}), json!("result"), json!({})),
            None,
            "completed",
            "/ws",
        );
        assert!(!model.show_output, "node-repl 全部关闭");
        assert!(!model.show_input);
        assert!(!model.show_kind);
    }

    #[test]
    fn plan_result_overrides_input_and_output() {
        let output = json!({"plan": "# 计划", "planFilePath": "p.md"});
        let model = build_tool_display_model(
            &tc(
                "ExitPlanMode",
                json!({"plan": "input 里的"}),
                output,
                json!({}),
            ),
            None,
            "completed",
            "/ws",
        );
        assert!(model.plan_result.is_some());
        assert!(!model.show_input, "planResult → 不展示输入");
        assert!(!model.show_output, "无错误 → 不展示通用输出");
    }

    #[test]
    fn error_text_collapses_view() {
        let model = build_tool_display_model(
            &tc(
                "Edit",
                json!({"file_path": "a.rs"}),
                json!({"error": "失败原因"}),
                json!({}),
            ),
            None,
            "completed",
            "/ws",
        );
        assert!(model.inline_preview.is_none(), "失败态收敛预览");
        assert!(!model.show_input);
        assert!(model.show_output);
        assert!(!model.show_kind);
    }
}
