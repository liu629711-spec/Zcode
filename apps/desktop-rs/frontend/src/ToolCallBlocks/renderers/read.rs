//! 1:1 翻译 `packages/ui/src/ToolCallBlocks/renderers/read.tsx`（316 行）。
//!
//! read 卡片。真源要点：
//! - **`canToggle={false}`**（:287）—— read 卡**不可折叠**（无 chevron、无点击区），
//!   与 execute/edit 的默认行为不同
//! - `prioritizePrimaryText`（:294）——窄屏优先保文件 chip
//! - `content={null}`（:299）—— 没有折叠主体
//! - **已接线**：快照提示（:302-311，refs 空/回调缺省时不渲染）
//! - 主体是 `ToolSummaryRow` 里的一段自定义渲染（ReadFileChip），不是纯文本
//!
//! 路径来源有**三级兜底**（真源 :152-206注释与代码）：
//! 1. `input.parsed_cmd[]` 里 `type==="read"` 的项（结构化优先）
//! 2. `input.filePath/file_path/path/filename`（某些 provider 直接放input）
//! 3. `toolCall.raw` 的 `<path>` XML 标签（真源 :196-200 注释：之前只兼容
//!    parsed_cmd，导致其他模型 read 卡片退回 kind/title）
//!
//! 行号注释均指真源文件。

use leptos::prelude::*;
use serde_json::Value;

use super::super::ToolSnapshotFieldNotice::{
    SnapshotFieldRef, ToolSnapshotFieldNoticeComponent, ToolSnapshotFieldNoticeProps,
};
use std::sync::Arc;

use crate::ToolCallBlocks::fileSummaryTypes::is_plain_record;
use crate::file_icons::icon_src;

// ---------------------------------------------------------------------------
// ReadSummary（真源 :16-22）
// ---------------------------------------------------------------------------

/// read 卡的摘要（真源 `ReadSummary`，:16-22）。
#[derive(Debug, Clone, PartialEq)]
pub struct ReadSummary {
    pub path: String,
    pub file_name: String,
    pub file_path: Option<String>,
    pub file_icon_src: String,
    /// "file" | "directory"（:21）。
    pub entry_type: EntryType,
}

/// 条目类型（真源 :48-56 `normalizeReadEntryType`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    File,
    Directory,
}

impl EntryType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
        }
    }
}

/// `normalizeReadEntryType`（真源 :48-56）：`dir`/`directory`/`folder` → Directory，
/// 其余（含未定义）→ File。
pub fn normalize_entry_type(value: Option<&str>) -> EntryType {
    match value.map(str::trim).map(str::to_lowercase).as_deref() {
        Some("dir") | Some("directory") | Some("folder") => EntryType::Directory,
        _ => EntryType::File,
    }
}

/// `readStringField`（真源 :24-34）：按候选键顺序取第一个非空字符串（trim 后）。
pub fn read_string_field(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(s) = value.get(key).and_then(|v| v.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

/// `extractTaggedValue`（真源 :36-40）：从文本里取 `<tag>...</tag>` 的内容（大小写不敏感）。
pub fn extract_tagged_value(text: &str, tag: &str) -> Option<String> {
    let lower_text = text.to_lowercase();
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = lower_text.find(&open)? + open.len();
    let rest = &text[start..];
    let end = rest.to_lowercase().find(&close)?;
    let value = rest[..end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

// ---------------------------------------------------------------------------
// raw 元数据（真源 :58-123）
// ---------------------------------------------------------------------------

/// `readReadMetadataFromRaw`（真源 :58-123）的结果。
#[derive(Debug, Clone, PartialEq)]
pub struct ReadMetadata {
    pub path: Option<String>,
    pub entry_type: EntryType,
}

/// `readReadMetadataFromRaw`（真源 :58-123）——从原始载荷读路径与类型。
///
/// 真源注释（:112-115，**重要**）：
/// 有些模型只在 rawInput 里给 filePath，但会在 rawOutput/content 里补充目录类型。
/// 之前一旦先读到 rawInput.filePath 就提前返回成 file，导致目录卡片图标错误。
/// 现在改成「类型优先看显式输出，路径再回退 rawInput」。
pub fn read_metadata_from_raw(raw: &Value) -> ReadMetadata {
    if !is_plain_record(raw) {
        return ReadMetadata {
            path: None,
            entry_type: EntryType::File,
        };
    }
    let raw_input = raw.get("rawInput").filter(|v| is_plain_record(v));
    let direct_path = raw_input
        .and_then(|r| read_string_field(r, &["filePath", "file_path", "path", "filename"]));
    let direct_type =
        raw_input.and_then(|r| read_string_field(r, &["type", "fileType", "entryType"]));

    let raw_output = raw.get("rawOutput").filter(|v| is_plain_record(v));
    let output_text = raw_output.and_then(|r| read_string_field(r, &["output", "text", "content"]));

    // ① rawOutput 的 <path>/<type> 标签优先。
    if let Some(text) = output_text.as_deref() {
        if let Some(tagged_path) = extract_tagged_value(text, "path") {
            let tagged_type = extract_tagged_value(text, "type");
            return ReadMetadata {
                path: Some(tagged_path),
                entry_type: normalize_entry_type(tagged_type.as_deref()),
            };
        }
    }

    // ② raw.content[] 里逐项找 <path> 标签（真源 :90-108）。
    if let Some(items) = raw.get("content").and_then(|c| c.as_array()) {
        for item in items {
            if !is_plain_record(item) {
                continue;
            }
            let nested = item.get("content").filter(|v| is_plain_record(v));
            let text = nested
                .and_then(|c| read_string_field(c, &["text"]))
                .or_else(|| read_string_field(item, &["text"]));
            if let Some(text) = text.as_deref() {
                if let Some(tagged_path) = extract_tagged_value(text, "path") {
                    let tagged_type = extract_tagged_value(text, "type");
                    return ReadMetadata {
                        path: Some(tagged_path),
                        entry_type: normalize_entry_type(tagged_type.as_deref()),
                    };
                }
            }
        }
    }

    // ③ 回退 rawInput（真源 :110-120的注释说明了为什么不能在这里就定死 file）。
    if let Some(path) = direct_path {
        return ReadMetadata {
            path: Some(path),
            entry_type: normalize_entry_type(direct_type.as_deref()),
        };
    }

    ReadMetadata {
        path: None,
        entry_type: EntryType::File,
    }
}

/// `readReadMetadataFromValue`（真源 :125-136）：从 input 直接读。
pub fn read_metadata_from_value(value: &Value) -> ReadMetadata {
    if !is_plain_record(value) {
        return ReadMetadata {
            path: None,
            entry_type: EntryType::File,
        };
    }
    ReadMetadata {
        path: read_string_field(value, &["filePath", "file_path", "path", "filename"]),
        entry_type: normalize_entry_type(
            read_string_field(value, &["type", "fileType", "entryType"]).as_deref(),
        ),
    }
}

// ---------------------------------------------------------------------------
// summary 构造（真源 :138-207）
// ---------------------------------------------------------------------------

/// `createReadSummary`（真源 :138-150）。
pub fn create_read_summary(
    path: &str,
    file_name: Option<&str>,
    entry_type: EntryType,
) -> ReadSummary {
    let derived_name = crate::ToolCallBlocks::renderers::get_path_leaf(path).to_string();
    ReadSummary {
        path: path.to_string(),
        file_name: file_name.map(|s| s.to_string()).unwrap_or(derived_name),
        file_path: Some(crate::ToolCallBlocks::renderers::get_path_leaf(path).to_string()),
        // 目录用 folder 图标，文件按扩展名解析（真源 :147）。
        file_icon_src: if entry_type == EntryType::Directory {
            icon_src("folder")
        } else {
            icon_src(&crate::file_icons::resolve_icon_name(path))
        },
        entry_type,
    }
}

/// `buildReadSummary`（真源 :152-207）——三级兜底解析路径。
///
/// 真实源注释（:188-190）：read 卡片之前只兼容 `parsed_cmd` 结构，
/// 其他模型常把路径放在 rawInput/filePath 或 rawOutput 的 `<path>` 标签里，
/// 导致 UI 退回 kind/title。这里保留结构化优先，再补原始输出兜底。
pub fn build_read_summary(input: &Value, raw: &Value) -> Option<ReadSummary> {
    // ── ① input.parsed_cmd[]（真源 :155-186）──
    if is_plain_record(input) {
        if let Some(parsed) = input.get("parsed_cmd").and_then(|v| v.as_array()) {
            let cwd = input
                .get("cwd")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.trim_end_matches('/').to_string());
            for item in parsed {
                if !is_plain_record(item)
                    || item.get("type").and_then(|v| v.as_str()) != Some("read")
                {
                    continue;
                }
                let relative_path = item
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let file_name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        relative_path.as_ref().and_then(|p| {
                            p.split('/')
                                .filter(|seg| !seg.is_empty())
                                .next_back()
                                .map(|s| s.to_string())
                        })
                    });
                if relative_path.is_none() && file_name.is_none() {
                    continue;
                }
                // 相对路径 + cwd → 绝对路径（真源 :177-179）。
                let absolute = match (&relative_path, &cwd) {
                    (Some(rel), Some(c)) if !rel.starts_with('/') => format!("{c}/{rel}"),
                    _ => relative_path
                        .clone()
                        .or_else(|| file_name.clone())
                        .unwrap_or_else(|| "read".to_string()),
                };
                return Some(create_read_summary(
                    &absolute,
                    file_name.as_deref(),
                    EntryType::File,
                ));
            }
        }
    }

    // ── ② input 直接字段（真源 :190-197）──
    let direct = read_metadata_from_value(input);
    if let Some(path) = direct.path {
        return Some(create_read_summary(&path, None, direct.entry_type));
    }

    // ── ③ raw 兜底（真源 :199-206）──
    let raw_meta = read_metadata_from_raw(raw);
    raw_meta
        .path
        .map(|path| create_read_summary(&path, None, raw_meta.entry_type))
}

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

/// `READ_TOOL_ICON`（真源 :14）——SearchIcon，size-4 shrink-0 text-foreground-subtle。
pub const READ_TOOL_ICON_CLASS: &str = "size-4 flex-none text-foreground-subtle";

/// `ReadFileChip`（真源 :208-252）的静态分支类名（:244）。
pub const READ_CHIP_STATIC_CLASS: &str =
    "inline-flex min-w-0 max-w-full items-center gap-1.5 text-foreground-subtle";

/// ReadToolCallBlock 的 props（对应真源从 context 解构的五个字段）。
#[derive(Debug, Clone)]
pub struct ReadBlockProps {
    pub tool_id: String,
    pub snapshot_refs: Vec<SnapshotFieldRef>,
    pub on_load_full_tool_call_fields: Option<Callback<String, bool>>,
    pub input: Value,
    pub raw: Value,
    pub title: Option<String>,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub is_running: bool,
    pub status_label: Option<String>,
    pub error_text: Option<String>,
    pub source_label: Option<String>,
    pub show_icon: bool,
    /// 摘要构造结果（提前算好，避免在 view 闭包里重算）。
    pub summary: Option<ReadSummary>,
}

/// `ReadToolCallBlock`（真源 :254-316）。
///
/// 真源关键 props：
/// - `canToggle={false}`（:287）—— **read 卡不可折叠**
/// - `prioritizePrimaryText`（:294）
/// - `content={null}`（:299）—— 无折叠主体
#[component]
pub fn ReadToolCallBlock(props: ReadBlockProps) -> impl IntoView {
    let summary = props.summary.clone();
    // canOpenPreview：只有文件类+ 有打开回调才可点（真源 :259）。
    let can_open_preview = summary
        .as_ref()
        .is_some_and(|s| s.entry_type == EntryType::File);

    // primaryText：有summary 用 chip，否则退回 title/kind/固定文案（真源 :273-281）。
    // chip 以**数据**形式传给 ToolLayout（View 类型非 Send，不能放进 Arc 传递）。
    let chip_data = summary
        .as_ref()
        .map(|s| crate::ToolCallBlocks::ToolLayout::FileChipData {
            path: s.path.clone(),
            file_name: s.file_name.clone(),
            icon_src: s.file_icon_src.clone(),
            clickable: can_open_preview,
        });

    let fallback_text = props
        .title
        .clone()
        .or_else(|| props.kind.clone())
        .unwrap_or_else(|| "读取".to_string());

    // kindLabel：运行态「正在读取」，否则「读取」（真源 :288-291）。
    let kind_label = if props.is_running {
        "正在读取"
    } else {
        "读取"
    };

    let status_label = props.status_label.clone();
    let status_tooltip = if props.status.as_deref() == Some("failed") {
        props.error_text.clone()
    } else {
        None
    };
    let show_failure_status = props.status.as_deref() == Some("failed");

    // secondaryText：只有 summary 才有 filePath（真源 :295）。
    let secondary_text = summary.as_ref().and_then(|s| s.file_path.clone());

    view! {
        <crate::ToolCallBlocks::ToolLayout::ToolLayoutComponent
            props=crate::ToolCallBlocks::ToolLayout::ToolLayoutProps {
                tool_id: props.tool_id.clone(),
                // 真源 :287 —— read 卡不可折叠。
                can_toggle: Some(false),
                prioritize_primary_text: Some(true),
                kind_label: Some(kind_label.to_string()),
                source_label: props.source_label.clone(),
                // 真源 :273-281 —— 有 summary 用 ReadFileChip，否则退回
                // title/kind/固定文案。chip 走 primary_text_view（ReactNode 形态）。
                primary_text: Some(fallback_text.clone()),
                primary_file_chip: chip_data,
                secondary_text: secondary_text.clone(),
                status_label: status_label.clone(),
                status_tooltip: status_tooltip.clone(),
                show_failure_status: Some(show_failure_status),
                is_running: Some(props.is_running),
                title: props.title.clone(),
                // 真源 :299 —— 无折叠主体。
                ..Default::default()
            }
            icon_view=Some(Arc::new(|| {
                view! {
                    <span class=READ_TOOL_ICON_CLASS>
                        <crate::app::Icon
                            paths=vec!["m21 21-4.34-4.34"]
                            circles=vec![("11", "11", "8")]
                        />
                    </span>
                }
                .into_any()
            }))
            // 真源 :299 —— content={null}，read 卡无折叠主体。
            render_content=None
        />
        // 真源 :302-311 —— 快照提示（Fragment 尾部）。
        <ToolSnapshotFieldNoticeComponent
            props=ToolSnapshotFieldNoticeProps {
                refs: props.snapshot_refs.clone(),
                tool_id: props.tool_id.clone(),
                on_load_full_tool_call_fields: props.on_load_full_tool_call_fields.clone(),
            }
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_entry_type_maps_directory_aliases() {
        // 真源 :48-56
        for v in ["dir", "directory", "folder", "Directory", " FOLDER "] {
            assert_eq!(
                normalize_entry_type(Some(v)),
                EntryType::Directory,
                "{v} 应映射为 directory"
            );
        }
        for v in ["file", "something", ""] {
            assert_eq!(normalize_entry_type(Some(v)), EntryType::File);
        }
        // 未定义 → file
        assert_eq!(normalize_entry_type(None), EntryType::File);
    }

    #[test]
    fn read_string_field_takes_first_non_empty() {
        // 真源 :24-34
        let v = json!({ "file_path": "  a.rs ", "path": "b.rs" });
        assert_eq!(
            read_string_field(&v, &["filePath", "file_path", "path"]),
            Some("a.rs".to_string())
        );
        // 空串跳过，继续找下一个。
        let v2 = json!({ "filePath": "  ", "path": "b.rs" });
        assert_eq!(
            read_string_field(&v2, &["filePath", "path"]),
            Some("b.rs".to_string())
        );
        // 都没有 → None。
        assert_eq!(read_string_field(&json!({}), &["path"]), None);
    }

    #[test]
    fn extract_tagged_value_is_case_insensitive() {
        // 真源 :36-40（正则带 i 标志）
        assert_eq!(
            extract_tagged_value("prefix <path>/a/b.rs</path> suffix", "path").as_deref(),
            Some("/a/b.rs")
        );
        assert_eq!(
            extract_tagged_value("<PATH>x.rs</PATH>", "path").as_deref(),
            Some("x.rs"),
            "大写标签也要命中"
        );
        assert_eq!(extract_tagged_value("no tag here", "path"), None);
        assert_eq!(
            extract_tagged_value("<path>  </path>", "path"),
            None,
            "空内容返回 None"
        );
    }

    #[test]
    fn raw_output_tag_wins_over_raw_input() {
        // ★真源 :112-115 的坑：类型优先看显式输出，路径再回退 rawInput。
        // 之前先读到 rawInput.filePath 就定成 file，目录卡片图标会错。
        let raw = json!({
            "rawInput": { "filePath": "/a/dir" },
            "rawOutput": { "text": "<path>/a/dir</path><type>directory</type>" }
        });
        let meta = read_metadata_from_raw(&raw);
        assert_eq!(meta.path.as_deref(), Some("/a/dir"));
        assert_eq!(
            meta.entry_type,
            EntryType::Directory,
            "目录类型应从 output 标签读到"
        );
    }

    #[test]
    fn raw_input_used_when_no_output_tag() {
        // 真源 :110-120 —— 没有输出标签时才回退 rawInput。
        let raw = json!({ "rawInput": { "filePath": "/a/f.rs" } });
        let meta = read_metadata_from_raw(&raw);
        assert_eq!(meta.path.as_deref(), Some("/a/f.rs"));
        assert_eq!(meta.entry_type, EntryType::File);
    }

    #[test]
    fn raw_content_items_scanned_for_tags() {
        // 真源 :90-108 —— raw.content[] 逐项找 <path>。
        let raw = json!({
            "rawInput": { "filePath": "/fallback" },
            "content": [
                { "type": "text", "content": { "text": "<path>/from/content.rs</path>" } }
            ]
        });
        let meta = read_metadata_from_raw(&raw);
        assert_eq!(meta.path.as_deref(), Some("/from/content.rs"));
    }

    #[test]
    fn build_summary_from_parsed_cmd() {
        // 真源 :155-186 —— 结构化优先。
        let input = json!({
            "cwd": "/work/dir/",
            "parsed_cmd": [{ "type": "read", "path": "src/main.rs" }]
        });
        let s = build_read_summary(&input, &json!({})).unwrap();
        assert_eq!(
            s.path, "/work/dir/src/main.rs",
            "相对路径要拼 cwd（尾部斜杠已裁）"
        );
        assert_eq!(s.file_name, "main.rs");
        assert_eq!(s.entry_type, EntryType::File);
    }

    #[test]
    fn build_summary_skips_non_read_items() {
        // 真源 :167-169 —— type 不是 read 的项要跳过。
        let input = json!({
            "cwd": "/w",
            "parsed_cmd": [
                { "type": "write", "path": "other.txt" },
                { "type": "read", "path": "target.rs" }
            ]
        });
        let s = build_read_summary(&input, &json!({})).unwrap();
        assert_eq!(s.path, "/w/target.rs");
    }

    #[test]
    fn build_summary_absolute_path_not_joined_with_cwd() {
        // 绝对路径不该再拼 cwd（真源 :177-179 的 startsWith("/") 判断）。
        let input = json!({
            "cwd": "/work",
            "parsed_cmd": [{ "type": "read", "path": "/abs/x.rs" }]
        });
        let s = build_read_summary(&input, &json!({})).unwrap();
        assert_eq!(s.path, "/abs/x.rs");
    }

    #[test]
    fn build_summary_falls_back_to_input_direct_fields() {
        // 真源 :190-197 —— 某些 provider 直接放 input.file_path。
        let input = json!({ "file_path": "/direct/file.rs" });
        let s = build_read_summary(&input, &json!({})).unwrap();
        assert_eq!(s.path, "/direct/file.rs");
    }

    #[test]
    fn build_summary_falls_back_to_raw() {
        // 真源 :199-206 —— 最后兜底 raw。
        let input = json!({});
        let raw = json!({ "rawInput": { "filePath": "/from/raw.rs" } });
        let s = build_read_summary(&input, &raw).unwrap();
        assert_eq!(s.path, "/from/raw.rs");
    }

    #[test]
    fn build_summary_returns_none_when_nothing_found() {
        assert_eq!(build_read_summary(&json!({}), &json!({})), None);
    }

    #[test]
    fn directory_summary_uses_folder_icon() {
        // 真源 :147 —— 目录用 folder 图标。
        let s = create_read_summary("/a/dir", None, EntryType::Directory);
        assert!(s.file_icon_src.contains("folder"));
        let f = create_read_summary("/a/b.rs", None, EntryType::File);
        assert!(
            f.file_icon_src.contains("rust"),
            "文件按扩展名解析：{}",
            f.file_icon_src
        );
    }
}
