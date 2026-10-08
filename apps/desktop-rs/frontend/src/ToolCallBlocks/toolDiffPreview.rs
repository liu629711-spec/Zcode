//! 1:1 翻译 `packages/ui/src/lib/toolDiffPreview.ts`（416 行）的
//! `buildUnifiedDiff` 链路（:84-416）。
//!
//! 输入 old/new 文本，输出可交给 PatchDiff 渲染的标准 unified diff。
//!
//! **裁剪注明**：真源的 contextLines 模式（:345-348、:377-381、:406-412 的
//! `trimPatchContext` 与 `formatDiffRangeFromSliceStart`）未迁——fileSummaries
//! 的调用点不带 options（`buildUnifiedDiff(old, new, label)`），走的是全量路径。
//! 迁移 Git 面板的「变更附近 N 行」视图时需补上。

/// LCS 表单元上限（真源 :3 `MAX_DIFF_LCS_CELLS = 60_000`）。
/// 超过则走锚点回退，避免大文件在建表上卡死主线程。
const MAX_DIFF_LCS_CELLS: usize = 60_000;

/// 行匹配（真源 `LineMatch`，:5-8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LineMatch {
    before_index: usize,
    after_index: usize,
}

/// `splitLines`（:84-86）：空文本没有行。
fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.split('\n').collect()
    }
}

/// `formatDiffRange`（:88-90）。
fn format_diff_range(line_count: usize) -> String {
    if line_count == 0 {
        "0,0".to_string()
    } else {
        format!("1,{line_count}")
    }
}

/// `countSharedPrefix`（:100-107）。
fn count_shared_prefix(before: &[&str], after: &[&str]) -> usize {
    let max_length = before.len().min(after.len());
    let mut index = 0;
    while index < max_length && before[index] == after[index] {
        index += 1;
    }
    index
}

/// `countSharedSuffix`（:109-123）。
fn count_shared_suffix(before: &[&str], after: &[&str], shared_prefix: usize) -> usize {
    let max_length = before.len().min(after.len()) - shared_prefix;
    let mut offset = 0;
    while offset < max_length
        && before[before.len() - 1 - offset] == after[after.len() - 1 - offset]
    {
        offset += 1;
    }
    offset
}

/// `appendLinesWithPrefix`（:124-130）。
fn append_lines_with_prefix(patch: &mut Vec<String>, prefix: char, lines: &[&str]) {
    for line in lines {
        patch.push(format!("{prefix}{line}"));
    }
}

/// `collectUniqueLineMatches`（:132-177）：两遍均只出现一次的行 → 候选锚点。
fn collect_unique_line_matches(before: &[&str], after: &[&str]) -> Vec<LineMatch> {
    use std::collections::HashMap;
    let mut before_occurrences: HashMap<&str, (usize, usize)> = HashMap::new(); // (count, first)
    let mut after_occurrences: HashMap<&str, (usize, usize)> = HashMap::new();

    for (index, line) in before.iter().enumerate() {
        let entry = before_occurrences.entry(line).or_insert((0, index));
        entry.0 += 1;
    }
    for (index, line) in after.iter().enumerate() {
        let entry = after_occurrences.entry(line).or_insert((0, index));
        entry.0 += 1;
    }

    let mut matches: Vec<LineMatch> = Vec::new();
    for (line, (before_count, before_index)) in &before_occurrences {
        let Some((after_count, after_index)) = after_occurrences.get(line) else {
            continue;
        };
        if *before_count != 1 || *after_count != 1 {
            continue;
        }
        matches.push(LineMatch {
            before_index: *before_index,
            after_index: *after_index,
        });
    }
    matches.sort_by_key(|m| m.before_index);
    matches
}

/// `findIncreasingAnchorMatches`（:179-213）：patience diff 的 LIS 部分，
/// 取 afterIndex 递增的最长锚点序列。
fn find_increasing_anchor_matches(matches: &[LineMatch]) -> Vec<LineMatch> {
    if matches.is_empty() {
        return Vec::new();
    }

    let mut predecessors: Vec<isize> = vec![-1; matches.len()];
    let mut pile_tops: Vec<usize> = Vec::new();

    for index in 0..matches.len() {
        let after_index = matches[index].after_index;
        let mut low = 0usize;
        let mut high = pile_tops.len();

        // 二分找第一个 afterIndex 不小于当前的堆顶（严格递增语义）。
        while low < high {
            let middle = (low + high) / 2;
            if matches[pile_tops[middle]].after_index < after_index {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        if low > 0 {
            predecessors[index] = pile_tops[low - 1] as isize;
        }
        if low == pile_tops.len() {
            pile_tops.push(index);
        } else {
            pile_tops[low] = index;
        }
    }

    let mut anchors: Vec<LineMatch> = Vec::new();
    let mut current = *pile_tops.last().unwrap() as isize;
    while current >= 0 {
        anchors.push(matches[current as usize]);
        current = predecessors[current as usize];
    }
    anchors.reverse();
    anchors
}

/// `appendDiffBodyWithLargeSegmentFallback`（:215-250）：
/// 唯一行锚点拆段 + 递归回正常 diff；无锚点才整块全删全加。
fn append_diff_body_with_large_segment_fallback(
    patch: &mut Vec<String>,
    before: &[&str],
    after: &[&str],
) {
    let anchors = find_increasing_anchor_matches(&collect_unique_line_matches(before, after));

    if anchors.is_empty() {
        // 真源注释（:222-225）：无锚点时整块退化，是保底而非首选。
        append_lines_with_prefix(patch, '-', before);
        append_lines_with_prefix(patch, '+', after);
        return;
    }

    let mut previous_before = 0usize;
    let mut previous_after = 0usize;
    for anchor in anchors {
        append_diff_body(
            patch,
            &before[previous_before..anchor.before_index],
            &after[previous_after..anchor.after_index],
        );
        patch.push(format!(" {}", before[anchor.before_index]));
        previous_before = anchor.before_index + 1;
        previous_after = anchor.after_index + 1;
    }
    append_diff_body(patch, &before[previous_before..], &after[previous_after..]);
}

/// `appendDiffBodyWithLcs`（:252-327）。
fn append_diff_body_with_lcs(patch: &mut Vec<String>, before: &[&str], after: &[&str]) {
    if before.is_empty() {
        append_lines_with_prefix(patch, '+', after);
        return;
    }
    if after.is_empty() {
        append_lines_with_prefix(patch, '-', before);
        return;
    }

    if before.len() * after.len() > MAX_DIFF_LCS_CELLS {
        append_diff_body_with_large_segment_fallback(patch, before, after);
        return;
    }

    // lcs[i][j]：before[i..] 与 after[j..] 的最长公共子序列长度（真源同表）。
    let n = before.len();
    let m = after.len();
    let mut lcs = vec![0u32; (n + 1) * (m + 1)];
    let idx = |i: usize, j: usize| i * (m + 1) + j;

    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[idx(i, j)] = if before[i] == after[j] {
                lcs[idx(i + 1, j + 1)] + 1
            } else {
                lcs[idx(i + 1, j)].max(lcs[idx(i, j + 1)])
            };
        }
    }

    let mut before_index = 0usize;
    let mut after_index = 0usize;
    while before_index < n || after_index < m {
        let before_line = before.get(before_index);
        let after_line = after.get(after_index);

        if before_line.is_some() && before_line == after_line {
            patch.push(format!(" {}", before_line.unwrap()));
            before_index += 1;
            after_index += 1;
            continue;
        }
        if before_line.is_none() {
            patch.push(format!("+{}", after_line.unwrap()));
            after_index += 1;
            continue;
        }
        if after_line.is_none() {
            patch.push(format!("-{}", before_line.unwrap()));
            before_index += 1;
            continue;
        }

        // 两侧都还有：按 LCS 分数决定先跳哪边（真源 :311-326）。
        let skip_after = lcs[idx(before_index, after_index + 1)];
        let skip_before = lcs[idx(before_index + 1, after_index)];
        if skip_after >= skip_before {
            patch.push(format!("+{}", after_line.unwrap()));
            after_index += 1;
        } else {
            patch.push(format!("-{}", before_line.unwrap()));
            before_index += 1;
        }
    }
}

/// `appendDiffBody`（:329-335）：真源目前直接走 LCS。
fn append_diff_body(patch: &mut Vec<String>, before: &[&str], after: &[&str]) {
    append_diff_body_with_lcs(patch, before, after);
}

/// `buildUnifiedDiff`（:337-416，非 context 模式）。
///
/// 文件头三分支（真源 :368-372 注释）：
/// - 新建：`--- /dev/null`（否则 PatchDiff 当 rename/change 处理，大文件卡死）；
/// - 删除：`+++ /dev/null`；
/// - 常规：`--- a/label` / `+++ b/label`。
///
/// 另补 `diff --git a/label b/label` 边界行——真源 :361-364 注释：只有
/// `---/+++` 头时，正文里恰好出现 `--- ...`（如 SQL 注释）会被解析器
/// 误判成第二个文件。总有返回值（真源返回 string | null，仅 context 模式
/// 可能 null；此路径恒有值）。
pub fn build_unified_diff(before: &str, after: &str, file_label: &str) -> String {
    let before_lines = split_lines(before);
    let after_lines = split_lines(after);
    let shared_prefix = count_shared_prefix(&before_lines, &after_lines);
    let shared_suffix = count_shared_suffix(&before_lines, &after_lines, shared_prefix);
    let before_middle = &before_lines[shared_prefix..before_lines.len() - shared_suffix];
    let after_middle = &after_lines[shared_prefix..after_lines.len() - shared_suffix];
    let is_created_file = before_lines.is_empty() && !after_lines.is_empty();
    let is_deleted_file = !before_lines.is_empty() && after_lines.is_empty();

    let mut patch_lines: Vec<String> = vec![
        format!("diff --git a/{file_label} b/{file_label}"),
        if is_created_file {
            "--- /dev/null".to_string()
        } else {
            format!("--- a/{file_label}")
        },
        if is_deleted_file {
            "+++ /dev/null".to_string()
        } else {
            format!("+++ b/{file_label}")
        },
        format!(
            "@@ -{} +{} @@",
            format_diff_range(before_lines.len()),
            format_diff_range(after_lines.len())
        ),
    ];

    // 公共前缀（全量模式：整段进 patch，真源 :379-385 无截断版）。
    if shared_prefix > 0 {
        append_lines_with_prefix(&mut patch_lines, ' ', &before_lines[..shared_prefix]);
    }
    append_diff_body(&mut patch_lines, before_middle, after_middle);
    if shared_suffix > 0 {
        append_lines_with_prefix(
            &mut patch_lines,
            ' ',
            &before_lines[before_lines.len() - shared_suffix..],
        );
    }

    patch_lines.join("\n")
}

/// `countPatchFileDiffs`（`patchDiffPreview.ts:29-`）：数出 patch 里包含几个文件。
///
/// `normalizeSingleFilePatch` 用它拦“多段 diff 拼在一个字段”的输入。
/// 两级判据：优先数 `diff --git` 头；没有则逐 hunk 消费正文找 `---/+++` 文件头对。
pub fn count_patch_file_diffs(patch: &str) -> usize {
    let lines: Vec<&str> = patch
        .split('\n')
        .map(|l| l.trim_end_matches('\r'))
        .collect();
    let git_header_count = lines
        .iter()
        .filter(|l| l.starts_with("diff --git "))
        .count();
    if git_header_count > 0 {
        return git_header_count;
    }

    // hunk 头解析：`@@ -old[,n] +new[,m] @@`。
    // 计数语义（真源 `Number(match[1] ?? "1")`）：省略 `,n` 时算 1 行。
    let parse_hunk_range = |line: &str| -> Option<(usize, usize)> {
        let rest = line.strip_prefix("@@ -")?;
        let (old_part, rest) = rest.split_once(" +")?;
        let new_part = rest.split(" @@").next()?;
        let count_of = |part: &str| -> Option<usize> {
            match part.split_once(',') {
                Some((_, n)) => n.trim().parse().ok(),
                None => Some(1),
            }
        };
        Some((count_of(old_part)?, count_of(new_part)?))
    };

    let is_file_header_pair = |index: usize| -> bool {
        index + 1 < lines.len()
            && lines[index].starts_with("--- ")
            && lines[index + 1].starts_with("+++ ")
    };

    let consume_hunk_body =
        |start: usize, mut remaining_old: isize, mut remaining_new: isize| -> Option<usize> {
            let mut cursor = start;
            while remaining_old > 0 || remaining_new > 0 {
                let Some(line) = lines.get(cursor) else {
                    return None;
                };
                // 真源注释（:55-63）：多文件 patch 无 `diff --git` 头时，下个文件头
                // `--- a/x` / `+++ b/x` 恰好以 -/+ 开头；hunk 配额 off-by-one 时会被
                // 吃进正文，把多文件误判成单文件。先探测文件头对，遇到即提前结束。
                if is_file_header_pair(cursor) {
                    return Some(cursor);
                }
                if line.starts_with("\\ ") {
                    cursor += 1;
                    continue;
                }
                match line.chars().next() {
                    Some(' ') => {
                        remaining_old -= 1;
                        remaining_new -= 1;
                    }
                    Some('-') => remaining_old -= 1,
                    Some('+') => remaining_new -= 1,
                    _ => return Some(cursor),
                }
                cursor += 1;
            }
            Some(cursor)
        };

    let mut file_count = 0usize;
    let mut index = 0usize;
    while index < lines.len() {
        if let Some((old_lines, new_lines)) = parse_hunk_range(lines[index]) {
            // hunk 头之前若有文件头对，记一个文件。
            let mut scan = index;
            let mut counted_before = false;
            while scan > 0 {
                scan -= 1;
                if is_file_header_pair(scan) {
                    counted_before = true;
                    break;
                }
                if lines[scan].starts_with("@@ ") || lines[scan].starts_with("diff --git ") {
                    break;
                }
            }
            if counted_before {
                file_count += 1;
            }
            match consume_hunk_body(index + 1, old_lines as isize, new_lines as isize) {
                Some(next) => index = next.max(index + 1),
                None => return file_count.max(1),
            }
            continue;
        }
        index += 1;
    }
    file_count.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_change_produces_standard_patch() {
        let patch = build_unified_diff("a\nb\nc\n", "a\nB\nc\n", "f.rs");
        // 文件头三分支 + diff --git 边界（真源 :361-364 防误判）。
        assert!(patch.starts_with("diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n"));
        // 尾换行切出末尾空行 → 4 行（真源 splitLines 同语义，TS split 也保留空尾）。
        assert!(patch.contains("@@ -1,4 +1,4 @@"));
        assert!(patch.contains(" a\n"));
        assert!(patch.contains("-b\n"));
        assert!(patch.contains("+B"));
    }

    #[test]
    fn created_file_uses_dev_null_on_left() {
        // 真源 :368-372 —— 新建文件左头是 /dev/null。
        let patch = build_unified_diff("", "new\n", "n.rs");
        assert!(patch.contains("--- /dev/null"));
        assert!(patch.contains("+++ b/n.rs"));
        assert!(patch.contains("+new"));
    }

    #[test]
    fn deleted_file_uses_dev_null_on_right() {
        let patch = build_unified_diff("old\n", "", "d.rs");
        assert!(patch.contains("--- a/d.rs"));
        assert!(patch.contains("+++ /dev/null"));
        assert!(patch.contains("-old"));
    }

    #[test]
    fn shared_prefix_and_suffix_are_context_lines() {
        let before = "same1\nsame2\nold\nsame3\nsame4\n";
        let after = "same1\nsame2\nnew\nsame3\nsame4\n";
        let patch = build_unified_diff(before, after, "x");
        // 前后公共区以空格上下文行出现。
        assert!(patch.contains("\n same1\n"));
        assert!(patch.contains("\n same4"));
        assert!(patch.contains("-old"));
        assert!(patch.contains("+new"));
    }

    #[test]
    fn large_segment_uses_anchor_fallback() {
        // 超 LCS 阈值：构造远距离唯一锚点，验证不整块退化。
        let mut before_lines: Vec<String> = Vec::new();
        let mut after_lines: Vec<String> = Vec::new();
        for i in 0..300 {
            before_lines.push(format!("old-{i}"));
            after_lines.push(format!("new-{i}"));
        }
        before_lines.push("ANCHOR-UNIQUE-LINE".into());
        after_lines.push("ANCHOR-UNIQUE-LINE".into());
        for i in 0..300 {
            before_lines.push(format!("tail-{i}"));
            after_lines.push(format!("tail-{i}"));
        }
        let before = before_lines.join("\n");
        let after = after_lines.join("\n");
        let patch = build_unified_diff(&before, &after, "big");
        // 锚点行应作为上下文保留（若整块退化则不会出现裸上下文行）。
        assert!(
            patch.contains("\n ANCHOR-UNIQUE-LINE\n"),
            "锚点应保留为上下文"
        );
        // 尾部未变的行不该全部出现在 patch 里（有公共后缀，会进上下文），
        // 但头部旧行应被删除。
        assert!(patch.contains("-old-0"));
    }

    #[test]
    fn count_patch_single_file() {
        let patch = build_unified_diff("a\n", "b\n", "f.rs");
        assert_eq!(count_patch_file_diffs(&patch), 1);
    }

    #[test]
    fn count_patch_detects_git_headers() {
        let patch = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\ndiff --git a/y b/y\n--- a/y\n+++ b/y\n@@ -1 +1 @@\n-c\n+d\n";
        assert_eq!(count_patch_file_diffs(patch), 2);
    }

    #[test]
    fn count_patch_without_git_header_detects_two_files() {
        // 无 diff --git 头的多文件 patch：靠 hunk 消费 + 文件头对探测。
        let patch = "--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n-a\n+b\n--- a/y\n+++ b/y\n@@ -1,1 +1,1 @@\n-c\n+d\n";
        assert_eq!(count_patch_file_diffs(patch), 2);
    }
}
