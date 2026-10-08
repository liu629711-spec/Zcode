//! 1:1 翻译 `packages/ui/src/lib/codeViewer.ts`（602 行）
//! + `packages/ui/src/lib/codeViewerWorkspaceScope.ts`（5 行）
//! + `packages/shared/src/media-preview.ts`（27 行，随迁）。
//!
//! 预览源提取：从工具调用提取「代码查看器源」（file / text / patch /
//! image / media / pdf / pptx 等），供 ToolCallBody / 聊天正文的
//! 「查看代码」按钮消费。
//!
//! **BundledLanguage 处理**：真源用 shiki 的 `BundledLanguage` 联合类型，
//! Rust 侧以 `String`（语言标识符，如 "typescript"）承接——高亮器不在
//! 本层。
//!
//! **decodeURI**：真源用 JS 内建（保留字符的转义原样输出）；Rust 侧
//! 手写等价实现（见 `decode_uri`）。

use std::sync::OnceLock;

use regex_lite::Regex;
use serde_json::Value;

use super::renderers::get_path_leaf;
use super::renderers::planToolCall::{is_absolute_file_path, join_file_path};
use super::resolveRenderer::ToolFamily;
use super::toolDiffPreview::{build_unified_diff, extract_before_after, extract_structured_diff};
use super::toolIdentity::{
    ToolIdentityLike, is_file_content_write_tool_call, is_file_diff_tool_call,
    resolve_tool_call_identity,
};

// ---------------------------------------------------------------------------
// packages/shared/src/media-preview.ts（随迁）
// ---------------------------------------------------------------------------

/// `MediaPreviewKind`（media-preview.ts:2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaPreviewKind {
    Audio,
    Video,
}

/// `MediaPreviewFormat`（media-preview.ts:4-8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaPreviewFormat {
    pub extension: &'static str,
    pub kind: MediaPreviewKind,
    pub media_type: &'static str,
}

/// `MEDIA_PREVIEW_FORMATS`（media-preview.ts:10-22，11 项）。
pub const MEDIA_PREVIEW_FORMATS: [MediaPreviewFormat; 11] = [
    MediaPreviewFormat {
        extension: ".mp4",
        kind: MediaPreviewKind::Video,
        media_type: "video/mp4",
    },
    MediaPreviewFormat {
        extension: ".mov",
        kind: MediaPreviewKind::Video,
        media_type: "video/quicktime",
    },
    MediaPreviewFormat {
        extension: ".webm",
        kind: MediaPreviewKind::Video,
        media_type: "video/webm",
    },
    MediaPreviewFormat {
        extension: ".m4v",
        kind: MediaPreviewKind::Video,
        media_type: "video/x-m4v",
    },
    MediaPreviewFormat {
        extension: ".mp3",
        kind: MediaPreviewKind::Audio,
        media_type: "audio/mpeg",
    },
    MediaPreviewFormat {
        extension: ".wav",
        kind: MediaPreviewKind::Audio,
        media_type: "audio/wav",
    },
    MediaPreviewFormat {
        extension: ".m4a",
        kind: MediaPreviewKind::Audio,
        media_type: "audio/mp4",
    },
    MediaPreviewFormat {
        extension: ".ogg",
        kind: MediaPreviewKind::Audio,
        media_type: "audio/ogg",
    },
    MediaPreviewFormat {
        extension: ".opus",
        kind: MediaPreviewKind::Audio,
        media_type: "audio/opus",
    },
    MediaPreviewFormat {
        extension: ".flac",
        kind: MediaPreviewKind::Audio,
        media_type: "audio/flac",
    },
    MediaPreviewFormat {
        extension: ".weba",
        kind: MediaPreviewKind::Audio,
        media_type: "audio/webm",
    },
];

/// `getMediaPreviewFormat`（media-preview.ts:24-27）：反斜杠转正斜杠 + 小写后
/// 按后缀匹配。
pub fn get_media_preview_format(path: &str) -> Option<&'static MediaPreviewFormat> {
    let normalized = path.replace('\\', "/").to_lowercase();
    MEDIA_PREVIEW_FORMATS
        .iter()
        .find(|format| normalized.ends_with(format.extension))
}

// ---------------------------------------------------------------------------
// codeViewerWorkspaceScope.ts
// ---------------------------------------------------------------------------

/// `CodeViewerWorkspaceScope`（codeViewerWorkspaceScope.ts:1-5）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodeViewerWorkspaceScope {
    pub workspace_path: Option<String>,
    pub workspace_identity: Option<String>,
    pub workspace_remote_session_id: Option<String>,
}

// ---------------------------------------------------------------------------
// codeViewer.ts —— 类型
// ---------------------------------------------------------------------------

/// `FILE_VIEWER_MAX_TEXT_BYTES`（真源 :23）。
pub const FILE_VIEWER_MAX_TEXT_BYTES: usize = 256 * 1024;

/// `FileCodeViewerSource`（真源 :24-28）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCodeViewerSource {
    pub scope: CodeViewerWorkspaceScope,
    pub title: String,
    pub path: String,
}

/// `CodeReviewAnchor`（真源 :30-37）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeReviewAnchor {
    pub request_id: String,
    pub title: String,
    pub body: String,
    pub priority: Option<u8>,
    pub start_line: Option<i64>,
    pub end_line: Option<i64>,
}

/// `CodeReviewCodeViewerSource`（真源 :39-44）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeReviewCodeViewerSource {
    pub scope: CodeViewerWorkspaceScope,
    pub title: String,
    pub path: String,
    pub review: CodeReviewAnchor,
}

/// `TextCodeViewerSource`（真源 :46-52）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextCodeViewerSource {
    pub scope: CodeViewerWorkspaceScope,
    pub title: String,
    pub path: Option<String>,
    pub content: String,
    /// 真源 `BundledLanguage`（shiki）；Rust 以标识符承接。
    pub language: String,
}

/// `PatchCodeViewerSource`（真源 :54-58）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchCodeViewerSource {
    pub scope: CodeViewerWorkspaceScope,
    pub title: String,
    pub path: Option<String>,
    pub patch: String,
}

/// `MultiFileDiffCodeViewerSource`（真源 :60-66）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiFileDiffCodeViewerSource {
    pub scope: CodeViewerWorkspaceScope,
    pub title: String,
    pub path: Option<String>,
    pub before_content: String,
    pub after_content: String,
}

/// `ImageCodeViewerSource`（真源 :67-72）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageCodeViewerSource {
    pub scope: CodeViewerWorkspaceScope,
    pub title: String,
    pub path: String,
    pub media_type: String,
}

/// `MediaCodeViewerSource`（真源 :73-80）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaCodeViewerSource {
    pub scope: CodeViewerWorkspaceScope,
    pub title: String,
    pub path: String,
    pub kind: MediaPreviewKind,
    pub media_type: String,
    pub url: Option<String>,
}

/// `PdfCodeViewerSource`（真源 :82-86）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfCodeViewerSource {
    pub scope: CodeViewerWorkspaceScope,
    pub title: String,
    pub path: String,
}

/// `PptxReferencePreviewNavigation`（真源 :92-96）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PptxReferencePreviewNavigation {
    pub request_id: String,
    pub page_index: i64,
    pub expected_source_fingerprint: String,
}

/// `PptxCodeViewerSource`（真源 :87-91）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PptxCodeViewerSource {
    pub scope: CodeViewerWorkspaceScope,
    pub title: String,
    pub path: String,
    pub reference_navigation: Option<PptxReferencePreviewNavigation>,
}

/// `CodeViewerSource`（真源 :98-107）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeViewerSource {
    File(FileCodeViewerSource),
    CodeReview(CodeReviewCodeViewerSource),
    Text(TextCodeViewerSource),
    Patch(PatchCodeViewerSource),
    MultiFileDiff(MultiFileDiffCodeViewerSource),
    Image(ImageCodeViewerSource),
    Media(MediaCodeViewerSource),
    Pdf(PdfCodeViewerSource),
    Pptx(PptxCodeViewerSource),
}

/// 工具调用（真源 `ChatToolCall` 的消费子集）。
///
/// 真源 output 是 `unknown`；v4 投影里是 `output.text` 字符串形态——
/// 用 `Value` 承接两种形态（extractContentText 等对字符串原样、对 record 取字段）。
#[derive(Debug, Clone)]
pub struct CodeViewerToolCall {
    pub tool_name: Option<String>,
    pub kind: Option<String>,
    pub title: Option<String>,
    pub input: Value,
    pub output: Value,
    pub raw: Value,
}

impl CodeViewerToolCall {
    /// 从 legacy 载荷构造（output 字符串包成 Value）。
    pub fn from_legacy(tc: &super::toolCallRowAdapter::LegacyToolCall) -> Self {
        Self {
            tool_name: tc.tool_name.clone(),
            kind: Some(tc.kind.clone()),
            title: tc.title.clone(),
            input: tc.input.clone().unwrap_or(Value::Null),
            output: tc.output.clone().map(Value::String).unwrap_or(Value::Null),
            raw: tc.raw.clone(),
        }
    }

    pub fn identity_like(&self) -> ToolIdentityLike<'_> {
        ToolIdentityLike {
            tool_name: self.tool_name.as_deref(),
            kind: self.kind.as_deref(),
            title: self.title.as_deref(),
            input: &self.input,
            raw: &self.raw,
        }
    }
}

// ---------------------------------------------------------------------------
// 语言 / 媒体类型表
// ---------------------------------------------------------------------------

/// `EXTENSION_TO_LANGUAGE`（真源 :109-146，36 项）。
fn extension_to_language(extension: &str) -> Option<&'static str> {
    Some(match extension {
        "bash" | "sh" => "bash",
        "c" | "h" => "c",
        "cc" | "cpp" => "cpp",
        "css" => "css",
        "diff" => "diff",
        "go" => "go",
        "htm" | "html" => "html",
        "java" => "java",
        "js" | "mjs" => "javascript",
        "json" => "json",
        "jsx" => "jsx",
        "md" => "markdown",
        "mermaid" | "mmd" => "mermaid",
        "py" => "python",
        "rb" => "ruby",
        "rs" => "rust",
        "sql" => "sql",
        "svg" => "xml",
        "text" | "txt" => "log",
        "toml" => "toml",
        "ts" => "typescript",
        "tsx" => "tsx",
        "xml" => "xml",
        "yaml" | "yml" => "yaml",
        _ => return None,
    })
}

/// `IMAGE_EXTENSION_TO_MEDIA_TYPE`（真源 :148-159，10 项）。
fn image_extension_to_media_type(extension: &str) -> Option<&'static str> {
    Some(match extension {
        "apng" => "image/apng",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "jpeg" | "jpg" => "image/jpeg",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// 基础 helper
// ---------------------------------------------------------------------------

/// `isRecord`（真源 :162-164）——**注意**：宽松（不排除数组），照抄。
fn is_record(value: &Value) -> bool {
    matches!(value, Value::Object(_) | Value::Array(_))
}

/// `normalizeToolLabel`（真源 :166-176）：label trim 非空用它；否则路径叶子；
/// 否则 "Tool Preview"。
fn normalize_tool_label(label: Option<&str>, fallback_path: Option<&str>) -> String {
    if let Some(label) = label {
        let trimmed = label.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(path) = fallback_path {
        return get_path_leaf(path).to_string();
    }
    "Tool Preview".to_string()
}

/// `findStringField`（真源 :178-190）：返回**原值**（不 trim）。
fn find_string_field(value: &Value, keys: &[&str]) -> Option<String> {
    if !is_record(value) {
        return None;
    }
    for key in keys {
        if let Some(candidate) = value.get(*key).and_then(|v| v.as_str()) {
            if !candidate.trim().is_empty() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

fn looks_like_diff_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // 真源 :192-194 —— `/^(diff --git|--- |\+\+\+ |@@ )/m`
    RE.get_or_init(|| Regex::new(r"(?m)^(diff --git|--- |\+\+\+ |@@ )").expect("diff 特征正则"))
}

/// `looksLikeDiff`（真源 :192-194）。
pub fn looks_like_diff(text: &str) -> bool {
    looks_like_diff_re().is_match(text)
}

/// `extractDiffText`（真源 :196-203）。
fn extract_diff_text(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        if looks_like_diff(s) {
            return Some(s.to_string());
        }
    }
    find_string_field(value, &["diff", "patch", "unifiedDiff", "unified_diff"])
}

/// `extractContentText`（真源 :205-219）。
fn extract_content_text(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        return Some(s.to_string());
    }
    find_string_field(
        value,
        &[
            "content",
            "text",
            "fileContent",
            "contents",
            "code",
            "result",
            "value",
        ],
    )
}

/// `extractRawPath`（真源 :221-234）。
fn extract_raw_path(value: &Value) -> Option<String> {
    find_string_field(
        value,
        &[
            "path",
            "filePath",
            "file_path",
            "filename",
            "targetPath",
            "target_path",
            "targetFile",
            "target_file",
            "file",
        ],
    )
}

/// `buildTextPreview`（真源 :236-246）。
fn build_text_preview(
    title: String,
    path: Option<String>,
    content: String,
) -> TextCodeViewerSource {
    TextCodeViewerSource {
        scope: CodeViewerWorkspaceScope::default(),
        language: infer_code_language(path.as_deref(), Some(&content)).to_string(),
        title,
        path,
        content,
    }
}

/// `resolveViewerPath`（真源 :248-259）：URI 解码 → 绝对路径直用，否则 join 工作区。
fn resolve_viewer_path(raw_path: Option<&str>, workspace_path: &str) -> Option<String> {
    let raw_path = raw_path.filter(|p| !p.is_empty())?;
    let decoded_path = decode_file_path_uri_escapes(raw_path);
    if is_absolute_file_path(&decoded_path) {
        return Some(decoded_path);
    }
    Some(join_file_path(workspace_path, &decoded_path))
}

// ---------------------------------------------------------------------------
// 语言 / 媒体推断
// ---------------------------------------------------------------------------

/// `inferCodeLanguage`（真源 :261-291）。
pub fn infer_code_language(path: Option<&str>, content_hint: Option<&str>) -> &'static str {
    if let Some(path) = path {
        let leaf = get_path_leaf(path);
        let extension = if leaf.contains('.') {
            leaf.rsplit('.').next().map(str::to_lowercase)
        } else {
            None
        };
        if let Some(extension) = extension {
            if let Some(language) = extension_to_language(&extension) {
                return language;
            }
        }
    }

    if let Some(hint) = content_hint {
        if hint.starts_with("#!/") {
            if hint.contains("python") {
                return "python";
            }
            if hint.contains("bash") || hint.contains("sh") {
                return "bash";
            }
        }
    }

    if let Some(hint) = content_hint {
        if looks_like_diff(hint) {
            return "diff";
        }
    }

    "log"
}

/// `inferImageMediaType`（真源 :293-308）。
pub fn infer_image_media_type(path: Option<&str>) -> Option<&'static str> {
    let path = path.filter(|p| !p.is_empty())?;
    let leaf = get_path_leaf(path);
    if !leaf.contains('.') {
        return None;
    }
    let extension = leaf.rsplit('.').next()?.to_lowercase();
    image_extension_to_media_type(&extension)
}

/// `inferMediaPreview` 的返回（真源 :310-317 的 `Omit<..., "type"|"title"|"path">`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaPreviewDescriptor {
    pub kind: MediaPreviewKind,
    pub media_type: String,
}

/// `inferMediaPreview`（真源 :310-317）。
pub fn infer_media_preview(path: Option<&str>) -> Option<MediaPreviewDescriptor> {
    let path = path.filter(|p| !p.is_empty())?;
    get_media_preview_format(path).map(|format| MediaPreviewDescriptor {
        kind: format.kind,
        media_type: format.media_type.to_string(),
    })
}

/// `isImagePreviewPath`（真源 :319-321）。
pub fn is_image_preview_path(path: Option<&str>) -> bool {
    infer_image_media_type(path).is_some()
}

/// `isPdfPreviewPath`（真源 :323-329）。
pub fn is_pdf_preview_path(path: Option<&str>) -> bool {
    let Some(path) = path.filter(|p| !p.is_empty()) else {
        return false;
    };
    get_path_leaf(path).to_lowercase().ends_with(".pdf")
}

/// `isPptxPreviewPath`（真源 :331-337）。
pub fn is_pptx_preview_path(path: Option<&str>) -> bool {
    let Some(path) = path.filter(|p| !p.is_empty()) else {
        return false;
    };
    get_path_leaf(path).to_lowercase().ends_with(".pptx")
}

// ---------------------------------------------------------------------------
// path.ts 随迁：decodeFilePathUriEscapes（path.ts:39-52）
// ---------------------------------------------------------------------------

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// `URI_ESCAPE_RE`（path.ts:3）的判定：是否存在 `%[0-9A-Fa-f]{2}`。
fn has_uri_escape(path: &str) -> bool {
    let bytes = path.as_bytes();
    if bytes.len() < 3 {
        return false;
    }
    (0..bytes.len() - 2).any(|i| {
        bytes[i] == b'%' && hex_val(bytes[i + 1]).is_some() && hex_val(bytes[i + 2]).is_some()
    })
}

/// `decodeURI`（JS 内建）等价实现。
///
/// 真源注释（path.ts:44-47）：用 decodeURI 只还原路径文本，**保留 %2F
/// 这类分隔符转义**，避免把文件名内容误拆成新的路径层级。等价规则：
/// 百分号编码先按 UTF-8 解码成字符；字符若在保留集（`;/?:@&=+$,#`）
/// 内则原样保留编码文本。无效编码 → None（真源 catch 回退原样）。
fn decode_uri(input: &str) -> Option<String> {
    const RESERVED: &[u8] = b";/?:@&=+$,#";
    let bytes = input.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            // 非 % 段直接拷贝（% 是 ASCII，段边界必为 char boundary）。
            let next = input[i..].find('%').map(|p| i + p).unwrap_or(bytes.len());
            out.push_str(&input[i..next]);
            i = next;
            continue;
        }
        // 收集连续 %XX 的字节序列 + 每字节的原始文本（保留字符要原样输出）。
        let mut seq: Vec<u8> = Vec::new();
        let mut raw_parts: Vec<&str> = Vec::new();
        let mut j = i;
        while j + 2 < bytes.len() {
            if bytes[j] != b'%' {
                break;
            }
            let (Some(high), Some(low)) = (hex_val(bytes[j + 1]), hex_val(bytes[j + 2])) else {
                break;
            };
            seq.push(high * 16 + low);
            raw_parts.push(&input[j..j + 3]);
            j += 3;
        }
        if seq.is_empty() {
            // 孤立 % 或无效 %XY：decodeURI 抛 URIError。
            return None;
        }
        // 逐字符解码（UTF-8）；保留字符原样输出编码文本。
        let mut k = 0;
        while k < seq.len() {
            let b = seq[k];
            if b < 0x80 {
                if RESERVED.contains(&b) {
                    out.push_str(raw_parts[k]);
                } else {
                    out.push(b as char);
                }
                k += 1;
                continue;
            }
            let len = if b >= 0xF0 {
                4
            } else if b >= 0xE0 {
                3
            } else if b >= 0xC0 {
                2
            } else {
                return None;
            };
            if k + len > seq.len() {
                return None;
            }
            let decoded = std::str::from_utf8(&seq[k..k + len]).ok()?;
            out.push_str(decoded);
            k += len;
        }
        i = j;
    }
    Some(out)
}

/// `decodeFilePathUriEscapes`（path.ts:39-52）。
pub fn decode_file_path_uri_escapes(path: &str) -> String {
    if !has_uri_escape(path) {
        return path.to_string();
    }
    decode_uri(path).unwrap_or_else(|| path.to_string())
}

// ---------------------------------------------------------------------------
// diff 头解析（真源 :339-427）
// ---------------------------------------------------------------------------

/// `isUsableDiffFileTarget`（真源 :339-342）。
fn is_usable_diff_file_target(target: Option<&str>) -> bool {
    target
        .map(str::trim)
        .is_some_and(|trimmed| !trimmed.is_empty() && trimmed != "/dev/null")
}

/// `unquoteDiffPath`（真源 :344-351）。
fn unquote_diff_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        return trimmed[1..trimmed.len() - 1].to_string();
    }
    trimmed.to_string()
}

/// `stripDiffPathPrefix`（真源 :353-360）：去 a/ b/ 前缀。
fn strip_diff_path_prefix(path: &str) -> String {
    let unquoted = unquote_diff_path(path);
    if unquoted.starts_with("a/") || unquoted.starts_with("b/") {
        return unquoted[2..].to_string();
    }
    unquoted
}

fn quoted_path_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // 真源 :368 —— /^"((?:\\.|[^"\\])+)"/  （忽略尾随内容）
    RE.get_or_init(|| Regex::new(r#"^"((?:\\.|[^"\\])+)""#).expect("引号路径正则"))
}

/// `parseDiffHeaderPath`（真源 :363-379）：引号路径或去 tab 时间戳路径。
fn parse_diff_header_path(header_value: &str) -> Option<String> {
    let trimmed = header_value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with('"') {
        let caps = quoted_path_re().captures(trimmed)?;
        let inner = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if inner.is_empty() {
            return None;
        }
        return Some(strip_diff_path_prefix(inner));
    }
    // split("\t", 1)[0]?.trim() —— 第一个 tab 之前的内容。
    let path_without_timestamp = trimmed.split('\t').next().unwrap_or("").trim();
    if path_without_timestamp.is_empty() {
        return None;
    }
    Some(strip_diff_path_prefix(path_without_timestamp))
}

fn diff_git_line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // 真源 :383-385 的 diff --git 行正则。
    RE.get_or_init(|| {
        Regex::new(r#"^diff --git (?:"((?:\\.|[^"\\])*)"|(\S+)) (?:"((?:\\.|[^"\\])*)"|(\S+))$"#)
            .expect("diff --git 正则")
    })
}

/// `parseDiffGitLinePath`（真源 :381-397）。
fn parse_diff_git_line_path(line: &str) -> Option<String> {
    let caps = diff_git_line_re().captures(line)?;
    let next_path = caps
        .get(3)
        .or_else(|| caps.get(4))
        .map(|m| m.as_str().to_string());
    let previous_path = caps
        .get(1)
        .or_else(|| caps.get(2))
        .map(|m| m.as_str().to_string());
    let normalized_next = next_path.map(|p| strip_diff_path_prefix(&p));
    if is_usable_diff_file_target(normalized_next.as_deref()) {
        return normalized_next;
    }
    let normalized_previous = previous_path.map(|p| strip_diff_path_prefix(&p));
    if is_usable_diff_file_target(normalized_previous.as_deref()) {
        normalized_previous
    } else {
        None
    }
}

/// `getPatchHeaderFileTarget`（真源 :399-425）：先扫 diff --git / +++ ，
/// 再回扫 ---。
fn get_patch_header_file_target(patch: &str) -> Option<String> {
    let lines: Vec<&str> = patch
        .split('\n')
        .map(|l| l.trim_end_matches('\r'))
        .collect();
    for line in &lines {
        if let Some(target) = parse_diff_git_line_path(line) {
            return Some(target);
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            let next_file_target = parse_diff_header_path(rest);
            if is_usable_diff_file_target(next_file_target.as_deref()) {
                return next_file_target;
            }
        }
    }
    for line in &lines {
        let Some(rest) = line.strip_prefix("--- ") else {
            continue;
        };
        let previous_file_target = parse_diff_header_path(rest);
        if is_usable_diff_file_target(previous_file_target.as_deref()) {
            return previous_file_target;
        }
    }
    None
}

/// `getDiffSourceFileTarget`（真源 :427-441）。
fn get_diff_source_file_target(source: &CodeViewerSource) -> Option<String> {
    let (path, patch) = match source {
        CodeViewerSource::Patch(p) => (p.path.as_deref(), Some(p.patch.as_str())),
        CodeViewerSource::MultiFileDiff(m) => (m.path.as_deref(), None),
        // 真源类型层面排除其他变体。
        _ => return None,
    };
    if is_usable_diff_file_target(path) {
        return path.map(str::to_string);
    }
    if let Some(patch) = patch {
        return get_patch_header_file_target(patch);
    }
    None
}

/// `createDiffSourceFilePreviewSource`（真源 :443-470）。
pub fn create_diff_source_file_preview_source(
    source: &CodeViewerSource,
    fallback_workspace_path: Option<&str>,
) -> Option<FileCodeViewerSource> {
    let file_target = get_diff_source_file_target(source)?;
    let source_scope = match source {
        CodeViewerSource::Patch(p) => &p.scope,
        CodeViewerSource::MultiFileDiff(m) => &m.scope,
        _ => return None,
    };
    let workspace_path = source_scope
        .workspace_path
        .clone()
        .or_else(|| fallback_workspace_path.map(str::to_string));
    if !is_absolute_file_path(&file_target) && workspace_path.is_none() {
        return None;
    }
    let path = resolve_viewer_path(Some(&file_target), workspace_path.as_deref().unwrap_or(""))?;
    // 真源 :462-468 —— 保留 workspace scope（远程 workspace / 远控按本地路径
    // 边界读文件会出错）；空串 falsy 不 spread。
    Some(FileCodeViewerSource {
        scope: CodeViewerWorkspaceScope {
            workspace_path: workspace_path.filter(|w| !w.is_empty()),
            workspace_identity: source_scope
                .workspace_identity
                .clone()
                .filter(|w| !w.is_empty()),
            workspace_remote_session_id: source_scope
                .workspace_remote_session_id
                .clone()
                .filter(|w| !w.is_empty()),
        },
        title: get_path_leaf(&path).to_string(),
        path,
    })
}

// ---------------------------------------------------------------------------
// 主提取（真源 :472-602）
// ---------------------------------------------------------------------------

/// `getToolCallCodePreview`（真源 :472-580）。
pub fn get_tool_call_code_preview(
    tool_call: &CodeViewerToolCall,
    workspace_path: &str,
) -> Option<CodeViewerSource> {
    // 结构化 diff 是工具结果的显式变更事实，优先于 input/output 中的全文预览。
    let structured_diff = extract_structured_diff(&tool_call.raw);
    let path_candidate = structured_diff
        .as_ref()
        .and_then(|d| d.path.clone())
        .or_else(|| extract_raw_path(&tool_call.input))
        .or_else(|| extract_raw_path(&tool_call.output));
    let resolved_path = resolve_viewer_path(path_candidate.as_deref(), workspace_path);
    let viewer_title = normalize_tool_label(
        tool_call.title.as_deref().or(tool_call.kind.as_deref()),
        resolved_path.as_deref(),
    );
    let identity_like = tool_call.identity_like();
    let identity = resolve_tool_call_identity(&identity_like);
    let is_diff_tool = is_file_diff_tool_call(&identity_like, Some(&identity));
    let is_read_tool = identity.family == ToolFamily::FileRead;
    let is_write_tool = is_file_content_write_tool_call(&identity_like, Some(&identity));
    let diff_label = resolved_path
        .as_deref()
        .map(get_path_leaf)
        .unwrap_or("preview");

    if let Some(diff) = &structured_diff {
        let patch = build_unified_diff(&diff.old_text, &diff.new_text, diff_label);
        if !patch.is_empty() {
            return Some(CodeViewerSource::Patch(PatchCodeViewerSource {
                scope: CodeViewerWorkspaceScope::default(),
                title: viewer_title,
                path: resolved_path,
                patch,
            }));
        }
        return Some(CodeViewerSource::Text(build_text_preview(
            viewer_title,
            resolved_path,
            diff.new_text.clone(),
        )));
    }

    let explicit_patch =
        extract_diff_text(&tool_call.output).or_else(|| extract_diff_text(&tool_call.input));
    if let Some(patch) = explicit_patch {
        return Some(CodeViewerSource::Patch(PatchCodeViewerSource {
            scope: CodeViewerWorkspaceScope::default(),
            title: viewer_title,
            path: resolved_path,
            patch,
        }));
    }

    if is_diff_tool {
        let before_after = extract_before_after(&tool_call.input)
            .or_else(|| extract_before_after(&tool_call.output));
        if let Some(ba) = before_after {
            let patch = build_unified_diff(&ba.before, &ba.after, diff_label);
            if !patch.is_empty() {
                return Some(CodeViewerSource::Patch(PatchCodeViewerSource {
                    scope: CodeViewerWorkspaceScope::default(),
                    title: viewer_title,
                    path: resolved_path,
                    patch,
                }));
            }
            return Some(CodeViewerSource::Text(build_text_preview(
                viewer_title,
                resolved_path,
                ba.after,
            )));
        }
    }

    // 真源 :536-543 —— read 优先 output；write 优先 input；其余同 read
    // （三分支的两支代码相同，照抄保持结构）。
    let preferred_content = if is_read_tool {
        extract_content_text(&tool_call.output).or_else(|| extract_content_text(&tool_call.input))
    } else if is_write_tool {
        extract_content_text(&tool_call.input).or_else(|| extract_content_text(&tool_call.output))
    } else {
        extract_content_text(&tool_call.output).or_else(|| extract_content_text(&tool_call.input))
    };

    if let Some(content) = preferred_content {
        return Some(CodeViewerSource::Text(build_text_preview(
            viewer_title,
            resolved_path,
            content,
        )));
    }

    if let Some(path) = &resolved_path {
        if (is_diff_tool || is_read_tool || is_write_tool) && is_image_preview_path(Some(path)) {
            return Some(CodeViewerSource::Image(ImageCodeViewerSource {
                scope: CodeViewerWorkspaceScope::default(),
                title: viewer_title,
                path: path.clone(),
                media_type: infer_image_media_type(Some(path))
                    .unwrap_or("application/octet-stream")
                    .to_string(),
            }));
        }
    }

    if let Some(path) = resolved_path {
        return Some(CodeViewerSource::File(FileCodeViewerSource {
            scope: CodeViewerWorkspaceScope::default(),
            title: viewer_title,
            path,
        }));
    }

    None
}

/// `getToolCallCodeContentPreview`（真源 :582-602）。
pub fn get_tool_call_code_content_preview(
    tool_call: &CodeViewerToolCall,
    workspace_path: &str,
) -> Option<TextCodeViewerSource> {
    let structured_diff = extract_structured_diff(&tool_call.raw);
    let path_candidate = structured_diff
        .as_ref()
        .and_then(|d| d.path.clone())
        .or_else(|| extract_raw_path(&tool_call.input))
        .or_else(|| extract_raw_path(&tool_call.output));
    let resolved_path = resolve_viewer_path(path_candidate.as_deref(), workspace_path);
    let viewer_title = normalize_tool_label(
        tool_call.title.as_deref().or(tool_call.kind.as_deref()),
        resolved_path.as_deref(),
    );
    let identity_like = tool_call.identity_like();
    let identity = resolve_tool_call_identity(&identity_like);
    let is_diff_tool = is_file_diff_tool_call(&identity_like, Some(&identity));
    let is_read_tool = identity.family == ToolFamily::FileRead;
    let is_write_tool = is_file_content_write_tool_call(&identity_like, Some(&identity));

    if let Some(diff) = structured_diff {
        return Some(build_text_preview(
            viewer_title,
            resolved_path,
            diff.new_text,
        ));
    }

    if is_diff_tool {
        let before_after = extract_before_after(&tool_call.input)
            .or_else(|| extract_before_after(&tool_call.output));
        if let Some(ba) = before_after {
            return Some(build_text_preview(viewer_title, resolved_path, ba.after));
        }
    }

    let preferred_content = if is_read_tool {
        extract_content_text(&tool_call.output).or_else(|| extract_content_text(&tool_call.input))
    } else if is_write_tool {
        extract_content_text(&tool_call.input).or_else(|| extract_content_text(&tool_call.output))
    } else {
        extract_content_text(&tool_call.output).or_else(|| extract_content_text(&tool_call.input))
    };

    let content = preferred_content?;
    Some(build_text_preview(viewer_title, resolved_path, content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tc(tool_name: Option<&str>, input: Value, output: Value, raw: Value) -> CodeViewerToolCall {
        CodeViewerToolCall {
            tool_name: tool_name.map(str::to_string),
            kind: tool_name.map(str::to_string),
            title: None,
            input,
            output,
            raw,
        }
    }

    #[test]
    fn media_preview_format_matches_case_and_backslash() {
        assert_eq!(
            get_media_preview_format(r"C:\videos\a.MP4").map(|f| f.kind),
            Some(MediaPreviewKind::Video)
        );
        assert_eq!(
            get_media_preview_format("a.mp3").map(|f| f.media_type),
            Some("audio/mpeg")
        );
        assert_eq!(get_media_preview_format("a.txt"), None);
    }

    #[test]
    fn infer_language_extension_shebang_diff_log() {
        assert_eq!(infer_code_language(Some("a/b.ts"), None), "typescript");
        assert_eq!(infer_code_language(Some("a/b.yaml"), None), "yaml");
        // 无扩展名但 shebang。
        assert_eq!(
            infer_code_language(None, Some("#!/usr/bin/env python\nprint(1)")),
            "python"
        );
        assert_eq!(
            infer_code_language(None, Some("#!/bin/bash\necho hi")),
            "bash"
        );
        // diff 内容特征。
        assert_eq!(
            infer_code_language(None, Some("--- a\n+++ b\n@@ -1 +1 @@")),
            "diff"
        );
        // 默认 log。
        assert_eq!(infer_code_language(None, None), "log");
    }

    #[test]
    fn image_media_type_and_preview_paths() {
        assert_eq!(infer_image_media_type(Some("a/b.PNG")), Some("image/png"));
        assert_eq!(infer_image_media_type(Some("noext")), None);
        assert!(is_image_preview_path(Some("x.png")));
        assert!(!is_image_preview_path(Some("x.txt")));
        assert!(is_pdf_preview_path(Some("docs/A.PDF")));
        assert!(!is_pdf_preview_path(Some("")));
        assert!(is_pptx_preview_path(Some("s.pptx")));
    }

    #[test]
    fn decode_uri_escapes_keeps_reserved() {
        // 空格还原。
        assert_eq!(decode_file_path_uri_escapes("a%20b"), "a b");
        // 中文 UTF-8 编码还原。
        assert_eq!(decode_file_path_uri_escapes("%E4%B8%AD.txt"), "中.txt");
        // %2F（保留字符）原样保留——真源注释：避免误拆路径层级。
        assert_eq!(decode_file_path_uri_escapes("a%2Fb"), "a%2Fb");
        // 无转义原样。
        assert_eq!(
            decode_file_path_uri_escapes("plain/path.rs"),
            "plain/path.rs"
        );
        // 无效编码（非 hex）→ 无 URI 特征，原样。
        assert_eq!(decode_file_path_uri_escapes("a%ZZ"), "a%ZZ");
        // 截断的 UTF-8 序列 → 回退原样（真源 catch）。
        assert_eq!(decode_file_path_uri_escapes("a%E4"), "a%E4");
    }

    #[test]
    fn looks_like_diff_multiline_anchors() {
        assert!(looks_like_diff("diff --git a/b b/b\nmore"));
        assert!(looks_like_diff("some\n@@ -1 +1 @@\n"));
        assert!(looks_like_diff("--- a/x\n+++ b/x"));
        assert!(!looks_like_diff("just text"));
        assert!(!looks_like_diff(" + text--- no anchor"));
    }

    #[test]
    fn patch_header_target_prefers_next_then_previous() {
        // diff --git 行优先，取 b/ 侧（next）。
        let patch = "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-a\n+b\n";
        let source = CodeViewerSource::Patch(PatchCodeViewerSource {
            scope: CodeViewerWorkspaceScope::default(),
            title: "t".into(),
            path: None,
            patch: patch.into(),
        });
        let target = get_diff_source_file_target(&source);
        assert_eq!(target.as_deref(), Some("src/main.rs"));
        // 没有 diff --git 时用 +++ 行。
        let patch2 = "--- a/foo.ts\n+++ b/foo.ts\n@@ -1 +1 @@\n";
        let source2 = CodeViewerSource::Patch(PatchCodeViewerSource {
            scope: CodeViewerWorkspaceScope::default(),
            title: "t".into(),
            path: None,
            patch: patch2.into(),
        });
        assert_eq!(
            get_diff_source_file_target(&source2).as_deref(),
            Some("foo.ts")
        );
        // /dev/null 被忽略，回退 --- 侧。
        let patch3 = "--- a/old.txt\n+++ /dev/null\n";
        let source3 = CodeViewerSource::Patch(PatchCodeViewerSource {
            scope: CodeViewerWorkspaceScope::default(),
            title: "t".into(),
            path: None,
            patch: patch3.into(),
        });
        assert_eq!(
            get_diff_source_file_target(&source3).as_deref(),
            Some("old.txt")
        );
    }

    #[test]
    fn create_file_preview_source_needs_workspace_for_relative_target() {
        let source = CodeViewerSource::Patch(PatchCodeViewerSource {
            scope: CodeViewerWorkspaceScope::default(),
            title: "t".into(),
            path: Some("src/rel.rs".into()),
            patch: String::new(),
        });
        // 相对路径 + 无工作区 → None。
        assert_eq!(create_diff_source_file_preview_source(&source, None), None);
        // 有工作区 → join。
        let file = create_diff_source_file_preview_source(&source, Some("D:/ws")).unwrap();
        assert_eq!(file.path, "D:/ws/src/rel.rs");
        assert_eq!(file.title, "rel.rs");
        assert_eq!(file.scope.workspace_path.as_deref(), Some("D:/ws"));
        // 绝对路径无工作区也 OK。
        let abs = CodeViewerSource::Patch(PatchCodeViewerSource {
            scope: CodeViewerWorkspaceScope::default(),
            title: "t".into(),
            path: Some("/abs/x.rs".into()),
            patch: String::new(),
        });
        let file = create_diff_source_file_preview_source(&abs, None).unwrap();
        assert_eq!(file.path, "/abs/x.rs");
    }

    #[test]
    fn code_preview_structured_diff_wins() {
        // raw.content 里的结构化 diff → patch（有 before/after 变化）。
        let raw = json!({"content": [{"type": "diff", "path": "src/a.rs", "oldText": "old\n", "newText": "new\n"}]});
        let call = tc(Some("Edit"), json!({}), json!({}), raw);
        let source = get_tool_call_code_preview(&call, "/ws").unwrap();
        match source {
            CodeViewerSource::Patch(p) => {
                assert_eq!(p.path.as_deref(), Some("/ws/src/a.rs"));
                // buildUnifiedDiff 的 label 用 getPathLeaf（真源 :497），
                // 头部格式是 `--- a/{label}`（真源 toolDiffPreview.ts 同款）。
                assert!(p.patch.contains("--- a/a.rs"));
            }
            other => panic!("期望 Patch，得 {other:?}"),
        }
    }

    #[test]
    fn code_preview_explicit_patch_from_output() {
        let patch = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n";
        let call = tc(Some("Bash"), json!({}), json!(patch), json!({}));
        let source = get_tool_call_code_preview(&call, "/ws").unwrap();
        assert!(matches!(source, CodeViewerSource::Patch(_)));
    }

    #[test]
    fn code_preview_edit_before_after_builds_patch() {
        let input = json!({"file_path": "src/t.ts", "old_string": "let a = 1;", "new_string": "let a = 2;"});
        let call = tc(Some("Edit"), input, json!({}), json!({}));
        let source = get_tool_call_code_preview(&call, "/ws").unwrap();
        match source {
            CodeViewerSource::Patch(p) => {
                assert_eq!(p.path.as_deref(), Some("/ws/src/t.ts"));
                assert!(p.patch.contains("-let a = 1;"));
                assert!(p.patch.contains("+let a = 2;"));
            }
            other => panic!("期望 Patch，得 {other:?}"),
        }
    }

    #[test]
    fn code_preview_read_output_is_text_with_language() {
        let call = tc(
            Some("Read"),
            json!({"file_path": "src/main.rs"}),
            json!("fn main() {}"),
            json!({}),
        );
        let source = get_tool_call_code_preview(&call, "/ws").unwrap();
        match source {
            CodeViewerSource::Text(t) => {
                assert_eq!(t.language, "rust");
                assert_eq!(t.content, "fn main() {}");
                // 真源 :509 —— title ?? kind 非空时 label 用它（不是路径叶子）。
                assert_eq!(t.title, "Read");
            }
            other => panic!("期望 Text，得 {other:?}"),
        }
        // label 为空白（title/kind 空）→ 回退路径叶子。
        let blank = CodeViewerToolCall {
            tool_name: Some("Read".into()),
            kind: Some("   ".into()),
            title: Some("  ".into()),
            input: json!({"file_path": "src/main.rs"}),
            output: json!("code"),
            raw: json!({}),
        };
        match get_tool_call_code_preview(&blank, "/ws").unwrap() {
            CodeViewerSource::Text(t) => assert_eq!(t.title, "main.rs"),
            other => panic!("期望 Text，得 {other:?}"),
        }
    }

    #[test]
    fn code_preview_path_only_is_file_source() {
        let call = tc(
            Some("Read"),
            json!({"file_path": "src/only.rs"}),
            json!({}),
            json!({}),
        );
        let source = get_tool_call_code_preview(&call, "/ws").unwrap();
        assert!(matches!(source, CodeViewerSource::File(_)));
    }

    #[test]
    fn code_preview_empty_returns_none() {
        let call = tc(Some("Bash"), json!({}), json!({}), json!({}));
        assert_eq!(get_tool_call_code_preview(&call, "/ws"), None);
    }

    #[test]
    fn content_preview_structured_diff_is_text() {
        let raw = json!({"type": "diff", "path": "a.rs", "oldText": "1", "newText": "2"});
        let call = tc(Some("Edit"), json!({}), json!({}), raw);
        let preview = get_tool_call_code_content_preview(&call, "/ws").unwrap();
        assert_eq!(preview.content, "2");
        assert_eq!(preview.language, "rust");
    }
}
