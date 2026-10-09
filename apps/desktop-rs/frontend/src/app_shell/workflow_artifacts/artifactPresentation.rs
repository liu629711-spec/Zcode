//! 1:1 翻译 `packages/ui/src/app-shell/workflow-artifacts/artifactPresentation.tsx`（248 行）。
//!
//! 用户面产物在**几个表面**（run 侧板瓦片、完成卡瓦片、`workflow-artifact` tab、通知行药丸、
//! 中枢）共用的呈现规则。
//!
//! ⚠ 术语：这里的 artifact 是脚本经 `artifact.*` 发布给用户的产出，不是引擎内部
//! 「脚本顶层返回值」的同名词。
//!
//! 抽出来的理由只有一条：**同一个产物在四处必须长得一样**。图标或 kind 词各写一遍，
//! 通知行的 chip 与它点开的 tab 迟早会用两枚不同的图标指同一件东西。

use leptos::prelude::*;

use crate::components::workflow_graph::run_state::WorkflowRunArtifactKind;

use super::presets::PresetLabels;

/// `ArtifactKindIcon`（真源 :46-58）的 Rust 视图：kind 图标。
/// `className` 由调用方给尺寸（卡片 size-4、chip size-3.5、tab 头部 size-4）——
/// 尺寸是各表面的密度决定的，图形本身不是。markdown 用文件展示描述符的图标
/// （真源 `FileDisplayIcon`）；Rust 侧 fileIcons 的 markdown 图与 file 同形，视觉一致。
pub fn artifact_kind_icon_view(kind: WorkflowRunArtifactKind) -> AnyView {
    use crate::components::workflowIcons;
    match kind {
        WorkflowRunArtifactKind::File | WorkflowRunArtifactKind::Markdown => {
            workflowIcons::icon_file().into_any()
        }
        WorkflowRunArtifactKind::Chart => workflowIcons::icon_chart_line().into_any(),
        WorkflowRunArtifactKind::Table => workflowIcons::icon_table().into_any(),
        WorkflowRunArtifactKind::Metrics => workflowIcons::icon_gauge().into_any(),
        WorkflowRunArtifactKind::Board => workflowIcons::icon_square_kanban().into_any(),
    }
}

/// 四个预置看板成员（真源 `PRESET_KINDS`）。内容成员（file / markdown）有字节与版本，它们没有。
pub fn is_artifact_preset_kind(kind: WorkflowRunArtifactKind) -> bool {
    matches!(
        kind,
        WorkflowRunArtifactKind::Chart
            | WorkflowRunArtifactKind::Table
            | WorkflowRunArtifactKind::Metrics
            | WorkflowRunArtifactKind::Board
    )
}

/// `artifactKindMessageId`（真源 :29-31）：六个成员各一个词（文件 / 文档 / 图表 / 表格 / 指标 / 看板）。
pub fn artifact_kind_message_id(kind: WorkflowRunArtifactKind) -> &'static str {
    match kind {
        WorkflowRunArtifactKind::File => "chat.toolCall.workflow.run.artifacts.kind.file",
        WorkflowRunArtifactKind::Markdown => "chat.toolCall.workflow.run.artifacts.kind.markdown",
        WorkflowRunArtifactKind::Chart => "chat.toolCall.workflow.run.artifacts.kind.chart",
        WorkflowRunArtifactKind::Table => "chat.toolCall.workflow.run.artifacts.kind.table",
        WorkflowRunArtifactKind::Metrics => "chat.toolCall.workflow.run.artifacts.kind.metrics",
        WorkflowRunArtifactKind::Board => "chat.toolCall.workflow.run.artifacts.kind.board",
    }
}

/// `formatArtifactBytes`（真源 :67-72）：字节数的展示写法。
///
/// 刻意在本模块另写一份而不是 import feedback 上传作业里那个同名函数：
/// 那是反馈上传作业模块，为了五行算术把整条上传链路拖进 app-shell 的依赖图不划算，
/// 而两者若漂移也不会有人受害（一个说文件多大，一个说传了多少）。
pub fn format_artifact_bytes(bytes: i64) -> String {
    if bytes < 0 {
        return String::new();
    }
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    if bytes < 1024 * 1024 {
        // JS 的 toFixed(1)：一位小数，舍入到最近。
        let kb = bytes as f64 / 1024.0;
        return format!("{:.1} KB", kb);
    }
    let mb = bytes as f64 / 1024.0 / 1024.0;
    format!("{:.2} MB", mb)
}

/// `isTextArtifactContentType`（真源 :82-85）：这个 contentType 的正文是不是**人能读的文本**
/// ——「复制」动作的门。`application/json` 单独列出来：IANA 归 application/，但脚本交出来的
/// JSON 用户想要的就是把它复制走。
pub fn is_text_artifact_content_type(content_type: Option<&str>) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };
    content_type.starts_with("text/") || content_type == "application/json"
}

/// `artifactFileBadge`（真源 :88-106）：文件的类型徽字。工作区原路径的扩展名优先
/// （`out/book.pdf` → `PDF`），没有路径时退回 MIME 子类型。
pub fn artifact_file_badge(
    source_path: Option<&str>,
    content_type: Option<&str>,
) -> Option<String> {
    // 真源 :92-94 —— /\.([a-z0-9]{1,5})$/iu：1 到 5 位的字母数字扩展名。
    // 扩展名不成立时要落到 MIME 子类型，不能把整个函数短路成 None。
    if let Some(path) = source_path {
        let lower = path.to_lowercase();
        if let Some(dot) = lower.rfind('.') {
            let ext = &lower[dot + 1..];
            if !ext.is_empty() && ext.len() <= 5 && ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
                return Some(ext.to_uppercase());
            }
        }
    }
    let subtype = content_type?
        .split('/')
        .nth(1)?
        .split(';')
        .next()?
        .trim();
    if subtype.is_empty() {
        return None;
    }
    const KNOWN: &[(&str, &str)] = &[
        ("x-markdown", "MD"),
        ("markdown", "MD"),
        ("plain", "TXT"),
        ("json", "JSON"),
        ("csv", "CSV"),
        ("html", "HTML"),
        ("pdf", "PDF"),
    ];
    if let Some((_, badge)) = KNOWN.iter().find(|(key, _)| *key == subtype) {
        return Some(badge.to_string());
    }
    if subtype.len() <= 5 {
        return Some(subtype.to_uppercase());
    }
    None
}

/// `ArtifactDetail` 的输入视图（真源 :113-124 的结构参数）。
pub struct ArtifactDetailInput<'a> {
    pub kind: WorkflowRunArtifactKind,
    pub bytes: Option<i64>,
    pub item_count: Option<i64>,
    pub source_path: Option<&'a str>,
    pub content_type: Option<&'a str>,
}

/// `artifactDetailText`（真源 :150-167）：说明行细节的**纯文本**写法（`PDF · 4.0 KB` /
/// `4 items`）：迷你瓦片把细节挪进 tooltip，交付物行把它写进 `kind · size` 那一行。
/// 与 `ArtifactDetail` 同一套规则，只是没有 testid。返回 `None` 即调用方不画细节槽。
pub fn artifact_detail_text(
    artifact: &ArtifactDetailInput<'_>,
    labels: &PresetLabels,
) -> Option<String> {
    if is_artifact_preset_kind(artifact.kind) {
        return artifact.item_count.map(|count| labels.items_count(count));
    }
    let bytes = artifact.bytes?;
    let badge = if artifact.kind == WorkflowRunArtifactKind::File {
        artifact_file_badge(artifact.source_path, artifact.content_type)
    } else {
        None
    };
    let size = format_artifact_bytes(bytes);
    Some(match badge {
        Some(badge) => format!("{badge} · {size}"),
        None => size,
    })
}

/// `resolvePrimaryArtifact`（真源 :174-180）：交付物 = 打了 `primary` 旗子的那一件；没有旗子
/// 而清单只有一件时，那一件就是交付物——**单件规则只在 UI 上成立**，协议与引擎从不推断旗子。
/// 其余情形没有交付物：发布失败的 primary 不由别的产物顶替，两件以上无旗子就是今天的画法。
pub fn resolve_primary_index(primaries: &[Option<bool>]) -> Option<usize> {
    if let Some((index, _)) = primaries
        .iter()
        .enumerate()
        .find(|(_, flag)| **flag == Some(true))
    {
        return Some(index);
    }
    if primaries.len() == 1 {
        return Some(0);
    }
    None
}

/// `orderArtifactsPrimaryFirst`（真源 :186-192）：交付物带头，其余保持原顺序。
/// CLI 的三处投影已经这样排过；这里是**活投影那条路**唯一的排序点。返回重排后的下标序列。
pub fn order_artifacts_primary_first(primaries: &[Option<bool>]) -> Vec<usize> {
    let Some(index) = primaries.iter().position(|flag| *flag == Some(true)) else {
        return (0..primaries.len()).collect();
    };
    if index == 0 {
        return (0..primaries.len()).collect();
    }
    let mut order: Vec<usize> = Vec::with_capacity(primaries.len());
    order.push(index);
    order.extend(0..index);
    order.extend(index + 1..primaries.len());
    order
}

/// `artifactDisplayTitle`（真源 :195-197）：作者写的 title 优先，缺席退回 id
/// （facade 的缺省 title 本来就是 id）。
pub fn artifact_display_title(id: &str, title: Option<&str>) -> String {
    match title.map(str::trim).filter(|t| !t.is_empty()) {
        Some(trimmed) => trimmed.to_string(),
        None => id.to_string(),
    }
}

/// chip 上的标题截断长度（真源 :200）。
const ARTIFACT_CHIP_TITLE_MAX_LENGTH: usize = 24;

/// `ARTIFACT_CHIP_MAX_VISIBLE`（真源 :203）：通知行 / 中枢行一次最多摆几枚 chip，其余折进「+N」。
pub const ARTIFACT_CHIP_MAX_VISIBLE: usize = 3;

/// `truncateArtifactChipTitle`（真源 :205-209）。
pub fn truncate_artifact_chip_title(title: &str) -> String {
    if title.chars().count() > ARTIFACT_CHIP_TITLE_MAX_LENGTH {
        // JS 的 slice 按码元；标题是 UI 文本，Rust 侧按字符截，视觉等价。
        let cut: String = title.chars().take(ARTIFACT_CHIP_TITLE_MAX_LENGTH).collect();
        format!("{cut}…")
    } else {
        title.to_string()
    }
}

/// `canRevealArtifactInWorkspace`（真源 :238-247）：该产物能不能在**本地文件系统**上被定位
/// （「在工作区显示」与 html 的「在浏览器中打开」的门）。远程 workspace（SSH / WSL / Docker）
/// 的路径在本机不存在，手机远控也没有文件树可以跳。`sourcePath` 缺席则连路径都没有——
/// markdown 产物与预置看板天生就没有出处。
pub fn can_reveal_artifact_in_workspace(
    source_path: Option<&str>,
    workspace_identity: Option<&str>,
    remote_session_id: Option<&str>,
) -> bool {
    let has_path = source_path.map(str::trim).is_some_and(|t| !t.is_empty());
    let no_identity = workspace_identity.map(str::trim).unwrap_or("").is_empty();
    has_path && no_identity && remote_session_id.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_message_ids_cover_all_six_members() {
        // 真源 :29-31 —— 六个成员各一个词，前缀一致。
        for kind in [
            WorkflowRunArtifactKind::File,
            WorkflowRunArtifactKind::Markdown,
            WorkflowRunArtifactKind::Chart,
            WorkflowRunArtifactKind::Table,
            WorkflowRunArtifactKind::Metrics,
            WorkflowRunArtifactKind::Board,
        ] {
            assert!(artifact_kind_message_id(kind).starts_with("chat.toolCall.workflow.run.artifacts.kind."));
        }
    }

    #[test]
    fn preset_kinds_are_exactly_four() {
        assert!(is_artifact_preset_kind(WorkflowRunArtifactKind::Chart));
        assert!(is_artifact_preset_kind(WorkflowRunArtifactKind::Table));
        assert!(is_artifact_preset_kind(WorkflowRunArtifactKind::Metrics));
        assert!(is_artifact_preset_kind(WorkflowRunArtifactKind::Board));
        assert!(!is_artifact_preset_kind(WorkflowRunArtifactKind::File));
        assert!(!is_artifact_preset_kind(WorkflowRunArtifactKind::Markdown));
    }

    #[test]
    fn bytes_format_matches_truth_source() {
        // 真源 :67-72 —— B / KB（1 位小数）/ MB（2 位小数），负数与非法值给空串。
        assert_eq!(format_artifact_bytes(0), "0 B");
        assert_eq!(format_artifact_bytes(1023), "1023 B");
        assert_eq!(format_artifact_bytes(1024), "1.0 KB");
        assert_eq!(format_artifact_bytes(4096), "4.0 KB");
        assert_eq!(format_artifact_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_artifact_bytes(-1), "");
    }

    #[test]
    fn text_content_types_include_json() {
        // 真源 :82-85 —— application/json 是「能整份复制」的文本。
        assert!(is_text_artifact_content_type(Some("text/markdown")));
        assert!(is_text_artifact_content_type(Some("application/json")));
        assert!(!is_text_artifact_content_type(Some("application/pdf")));
        assert!(!is_text_artifact_content_type(None));
    }

    #[test]
    fn file_badge_prefers_path_extension() {
        // 真源 :88-106 —— 扩展名优先，MIME 子类型兜底，已知子类型换写法。
        assert_eq!(artifact_file_badge(Some("out/book.PDF"), None).as_deref(), Some("PDF"));
        assert_eq!(artifact_file_badge(None, Some("application/pdf")).as_deref(), Some("PDF"));
        assert_eq!(artifact_file_badge(None, Some("text/markdown; charset=utf-8")).as_deref(), Some("MD"));
        assert_eq!(artifact_file_badge(None, Some("application/x-markdown")).as_deref(), Some("MD"));
        // 未知子类型 ≤ 5 位照写，更长的没有徽字。
        assert_eq!(artifact_file_badge(None, Some("image/webp")).as_deref(), Some("WEBP"));
        assert_eq!(artifact_file_badge(None, Some("x-weird/averylongsubtype")), None);
        assert_eq!(artifact_file_badge(Some("noext"), None), None);
    }

    #[test]
    fn detail_text_covers_preset_file_and_markdown() {
        let labels = super::super::presets::build_preset_labels();
        // 预置看板：只有条数，没有字节。
        let board = ArtifactDetailInput {
            kind: WorkflowRunArtifactKind::Board,
            bytes: Some(2048),
            item_count: Some(4),
            source_path: None,
            content_type: None,
        };
        assert_eq!(artifact_detail_text(&board, &labels), Some(labels.items_count(4)));
        // 预置看板没有条数：不画细节槽。
        let board = ArtifactDetailInput {
            item_count: None,
            ..board
        };
        assert_eq!(artifact_detail_text(&board, &labels), None);
        // 文件：徽字 · 大小。
        let file = ArtifactDetailInput {
            kind: WorkflowRunArtifactKind::File,
            bytes: Some(4096),
            item_count: None,
            source_path: Some("out/report.csv"),
            content_type: None,
        };
        assert_eq!(artifact_detail_text(&file, &labels).as_deref(), Some("CSV · 4.0 KB"));
        // 文档：只有大小，没有徽字。
        let md = ArtifactDetailInput {
            kind: WorkflowRunArtifactKind::Markdown,
            bytes: Some(4096),
            item_count: None,
            source_path: None,
            content_type: None,
        };
        assert_eq!(artifact_detail_text(&md, &labels).as_deref(), Some("4.0 KB"));
        // 没有字节：不画。
        let md = ArtifactDetailInput { bytes: None, ..md };
        assert_eq!(artifact_detail_text(&md, &labels), None);
    }

    #[test]
    fn primary_resolution_is_flag_or_single() {
        // 真源 :174-180 —— 旗子优先；无旗子单件顶上；其余没有交付物。
        assert_eq!(resolve_primary_index(&[None, Some(true), None]), Some(1));
        assert_eq!(resolve_primary_index(&[Some(false)]), Some(0));
        assert_eq!(resolve_primary_index(&[Some(false), Some(false)]), None);
        assert_eq!(resolve_primary_index(&[]), None);
    }

    #[test]
    fn ordering_puts_primary_first_and_keeps_the_rest() {
        // 真源 :186-192 —— 交付物带头，其余保持原顺序。
        assert_eq!(order_artifacts_primary_first(&[None, Some(true), None]), vec![1, 0, 2]);
        assert_eq!(order_artifacts_primary_first(&[Some(true), None]), vec![0, 1]);
        assert_eq!(order_artifacts_primary_first(&[None, None]), vec![0, 1]);
    }

    #[test]
    fn display_title_prefers_author_and_trims() {
        // 真源 :195-197 —— title.trim() || id。
        assert_eq!(artifact_display_title("a1", Some(" 报告 ")), "报告");
        assert_eq!(artifact_display_title("a1", Some("   ")), "a1");
        assert_eq!(artifact_display_title("a1", None), "a1");
    }

    #[test]
    fn chip_title_truncates_at_24_chars() {
        // 真源 :205-209 —— 超过 24 截断加省略号。
        let long = "标".repeat(25);
        let cut = truncate_artifact_chip_title(&long);
        assert_eq!(cut.chars().count(), 25);
        assert!(cut.ends_with('…'));
        assert_eq!(truncate_artifact_chip_title("短标题"), "短标题");
        // 恰好 24 不截。
        let exact = "x".repeat(24);
        assert_eq!(truncate_artifact_chip_title(&exact), exact);
    }

    #[test]
    fn reveal_gate_is_local_workspace_with_path() {
        // 真源 :238-247 —— 本地 workspace + 有路径才可定位。
        assert!(can_reveal_artifact_in_workspace(Some("out/a.html"), None, None));
        assert!(can_reveal_artifact_in_workspace(Some(" out/a.html "), Some("  "), None));
        assert!(!can_reveal_artifact_in_workspace(Some("out/a.html"), Some("ssh://x"), None));
        assert!(!can_reveal_artifact_in_workspace(Some("out/a.html"), None, Some("rs1")));
        assert!(!can_reveal_artifact_in_workspace(None, None, None));
        assert!(!can_reveal_artifact_in_workspace(Some("   "), None, None));
    }
}
