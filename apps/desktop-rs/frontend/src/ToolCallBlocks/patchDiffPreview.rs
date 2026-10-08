//! 1:1 翻译 `packages/ui/src/lib/patchDiffPreview.ts`（446 行）的
//! **预览行链**（:24-446 中 EditInlineDiffContent 消费的部分）。
//!
//! **范围注明**：
//! - `countPatchFileDiffs` 已迁在 toolDiffPreview.rs（早前批次）。
//! - `getPlainTextPatchFallbackLines` 链（:347-446，含 `@pierre/diffs` 的
//!   `parsePatchFiles`/`getSingularPatch` 判定与 lockfile/gradle 降级规则）
//!   未迁——其消费者是 GitPane / previewPane / MobileRemoteShell（非
//!   ToolCallBlocks 域），随 GitPane 批次迁移。
//! - 本文件只服务 EditInlineDiffContent：`getPlainTextPatchPreviewLines`
//!   把 patch 转成轻量文本预览行。

/// `MAX_PLAIN_TEXT_FALLBACK_RENDER_LINES`（真源 :7）。
const MAX_PLAIN_TEXT_FALLBACK_RENDER_LINES: usize = 800;

/// 截断 marker（真源 :10-11）——内部 token，展示文案在组件层走 i18n。
const FALLBACK_TRUNCATED_MARKER_PREFIX: &str = "\\ __ZCODE_DIFF_TRUNCATED__:";

/// `buildTruncatedMarkerLine`（真源 :24-26）。
fn build_truncated_marker_line(omitted_line_count: usize) -> String {
    format!("{FALLBACK_TRUNCATED_MARKER_PREFIX}{omitted_line_count}")
}

/// `parseTruncatedMarkerOmittedLineCount`（真源 :150-162）。
pub fn parse_truncated_marker_omitted_line_count(line: &str) -> Option<usize> {
    let count_text = line.strip_prefix(FALLBACK_TRUNCATED_MARKER_PREFIX)?;
    // 正则约束 `(\d+)$`：必须全为数字且至少一位。
    if count_text.is_empty() || !count_text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let omitted_line_count: usize = count_text.parse().ok()?;
    if omitted_line_count == 0 {
        return None;
    }
    Some(omitted_line_count)
}

/// `getPatchPreviewLineContent`（真源 :164-170）：剥掉 `+`/`-`/空格 marker。
pub fn get_patch_preview_line_content(line: &str) -> String {
    if line.starts_with('+') || line.starts_with('-') || line.starts_with(' ') {
        return line[1..].to_string();
    }
    line.to_string()
}

/// `isPatchHunkHeaderLine`（真源 :172-174）。
fn is_patch_hunk_header_line(line: &str) -> bool {
    line == "@@" || line.starts_with("@@ ")
}

/// `limitPlainTextPreviewLines`（真源 :197-211）：超 800 行时头尾各半 +
/// 中间截断 marker。
fn limit_plain_text_preview_lines(lines: &[String]) -> Vec<String> {
    if lines.len() <= MAX_PLAIN_TEXT_FALLBACK_RENDER_LINES {
        return lines.to_vec();
    }

    let keep_count = MAX_PLAIN_TEXT_FALLBACK_RENDER_LINES - 1;
    let head_count = keep_count.div_ceil(2);
    let tail_count = keep_count - head_count;
    let omitted_count = lines.len() - keep_count;

    let mut out: Vec<String> = lines[..head_count].to_vec();
    out.push(build_truncated_marker_line(omitted_count));
    out.extend_from_slice(&lines[lines.len() - tail_count..]);
    out
}

/// `collectPatchMetadataPreviewLines`（真源 :231-244）：丢弃 diff --git /
/// index / --- / +++ 协议头，保留 rename/mode 等元信息行。
fn collect_patch_metadata_preview_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| {
            if line.trim().is_empty() {
                return false;
            }
            !line.starts_with("diff --git ")
                && !line.starts_with("index ")
                && !line.starts_with("--- ")
                && !line.starts_with("+++ ")
        })
        .cloned()
        .collect()
}

/// `collectPlainTextPreviewLines`（真源 :246-276）：进入 hunk 后原样保留
/// （只跳 hunk 头本身，不按前缀过滤——防误删正文里的 `++ `/`-- ` 行）。
fn collect_plain_text_preview_lines(lines: &[String]) -> Vec<String> {
    let mut preview_lines: Vec<String> = Vec::new();
    let mut in_hunk = false;

    for line in lines {
        if !in_hunk {
            if is_patch_hunk_header_line(line) {
                in_hunk = true;
            }
            continue;
        }
        if is_patch_hunk_header_line(line) {
            continue;
        }
        preview_lines.push(line.clone());
    }

    if in_hunk || !preview_lines.is_empty() {
        return preview_lines;
    }

    // rename-only / mode-only 这类 patch 没有 @@ hunk，但仍有元信息可看。
    collect_patch_metadata_preview_lines(lines)
}

/// `normalizePlainTextPreviewLines`（真源 :278-289）：截断 + 去尾空行 +
/// 至少一个空串占位。
fn normalize_plain_text_preview_lines(lines: &[String]) -> Vec<String> {
    let mut normalized = limit_plain_text_preview_lines(lines);
    while normalized.last().is_some_and(|l| l.is_empty()) {
        normalized.pop();
    }
    if normalized.is_empty() {
        vec![String::new()]
    } else {
        normalized
    }
}

/// `splitPatchLines`：`patch.split(/\r?\n/)`。
fn split_patch_lines(patch: &str) -> Vec<String> {
    patch
        .split('\n')
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect()
}

/// `getPlainTextPatchPreviewLines`（真源 :339-341）。
pub fn get_plain_text_patch_preview_lines(patch: &str) -> Vec<String> {
    normalize_plain_text_preview_lines(&get_plain_text_patch_content_lines(patch))
}

/// `getPlainTextPatchContentLines`（真源 :343-345）。
pub fn get_plain_text_patch_content_lines(patch: &str) -> Vec<String> {
    collect_plain_text_preview_lines(&split_patch_lines(patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_marker_roundtrip() {
        let marker = build_truncated_marker_line(1234);
        assert_eq!(
            parse_truncated_marker_omitted_line_count(&marker),
            Some(1234)
        );
        // 非 marker / 0 / 非数字 → None。
        assert_eq!(parse_truncated_marker_omitted_line_count("+ normal"), None);
        assert_eq!(
            parse_truncated_marker_omitted_line_count("\\ __ZCODE_DIFF_TRUNCATED__:0"),
            None
        );
        assert_eq!(
            parse_truncated_marker_omitted_line_count("\\ __ZCODE_DIFF_TRUNCATED__:12x"),
            None
        );
    }

    #[test]
    fn preview_line_content_strips_marker() {
        assert_eq!(get_patch_preview_line_content("+added"), "added");
        assert_eq!(get_patch_preview_line_content("-removed"), "removed");
        assert_eq!(get_patch_preview_line_content(" context"), "context");
        assert_eq!(get_patch_preview_line_content("@@ -1 +1 @@"), "@@ -1 +1 @@");
    }

    #[test]
    fn preview_lines_keep_hunk_body_verbatim() {
        let patch = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,3 +1,3 @@\n context\n-old\n+new\n++ real plus line\n";
        let lines = get_plain_text_patch_preview_lines(patch);
        assert_eq!(
            lines,
            vec![
                " context".to_string(),
                "-old".to_string(),
                "+new".to_string(),
                "++ real plus line".to_string()
            ],
            "hunk 进入后原样保留（含 ++ 开头的真实正文行）"
        );
    }

    #[test]
    fn metadata_only_patch_falls_back_to_metadata() {
        // 没有 @@ hunk（rename-only）→ 保留元信息。
        let patch = "diff --git a/old.rs b/new.rs\nsimilarity index 100%\nrename from old.rs\nrename to new.rs\n";
        let lines = get_plain_text_patch_preview_lines(patch);
        assert!(lines.iter().any(|l| l.contains("rename from old.rs")));
        assert!(!lines.iter().any(|l| l.starts_with("diff --git")));
    }

    #[test]
    fn oversized_preview_is_truncated_with_marker() {
        let mut patch = String::from("@@ -1 +1 @@\n");
        for i in 0..1000 {
            patch.push_str(&format!("+line {i}\n"));
        }
        let lines = get_plain_text_patch_preview_lines(&patch);
        // 800 行上限里最后一行是 patch 末尾空行，被 normalize 的去尾空行步骤吃掉。
        assert_eq!(lines.len(), MAX_PLAIN_TEXT_FALLBACK_RENDER_LINES - 1);
        assert!(
            parse_truncated_marker_omitted_line_count(&lines[lines.len() / 2]).is_some()
                || lines
                    .iter()
                    .any(|l| parse_truncated_marker_omitted_line_count(l).is_some()),
            "应含截断 marker"
        );
    }

    #[test]
    fn empty_patch_yields_single_empty_line() {
        assert_eq!(get_plain_text_patch_preview_lines(""), vec![String::new()]);
    }
}
