// 从真源 `packages/ui/src/i18n/locales/zh-CN.ts` 生成 Rust 文案表。
//
// 为什么不手抄：1500+ 条映射，手抄必错，且真源改文案时Rust 侧会静默过期。
// 与 `gen_file_icons.mjs` 同一思路：真源是唯一源，Rust 侧是生成物。
//
// 真源格式（zh-CN.ts）：
//   "chat.toolCall.cua.appName": "电脑控制",
//   "chat.toolCall.cua.seconds": "{duration} 秒",
//
// 插值占位符是 ICU 风格 `{name}`，本脚本原样保留 —— Rust 侧 `format()` 按名替换。
// 值里的引号/反斜杠/换行做 JSON 转义（用 JSON.stringify 而非手写）。

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const here = path.dirname(fileURLToPath(import.meta.url));
// here = <repo>/apps/desktop-rs/scripts → 退 3 层到仓库根。
const repoRoot = path.resolve(here, "../../..");
const srcPath = path.join(repoRoot, "packages/ui/src/i18n/locales/zh-CN.ts");
const src = fs.readFileSync(srcPath, "utf8");

// 只取 `"<id>": "<value>",` 这一形态。zh-CN.ts 顶层是扁平的
// 单行字面量表（无嵌套），所以正则够用；用 \r?\n 兼容 CRLF。
const entries = [];
const re = /^\s*"([^"]+)":\s*("(?:[^"\\]|\\.)*"),?\s*$/gm;
let m;
while ((m = re.exec(src)) !== null) {
  const [, id, rawValue] = m;
  const value = JSON.parse(rawValue);
  entries.push([id, value]);
}

if (entries.length < 1000) {
  throw new Error(`只解析出 ${entries.length} 条，明显少于预期，检查 zh-CN.ts 格式`);
}

// 二分查找要求有序。真源本身按键字典序（IntlProvider 生成时排序），
// 但不依赖这一点——统一排序后输出。
entries.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));

const rows = entries
  .map(([id, value]) => `    (${JSON.stringify(id)}, ${JSON.stringify(value)}),`)
  .join("\n");

const out = String.raw`//! 中文文案表（1:1 翻译 \`packages/ui/src/i18n/locales/zh-CN.ts\` 的 \`chat.*\` 段）。
//!
//! **本文件是生成物**，由 \`scripts/gen_i18n.mjs\` 从真源导出，改文案请改真源后重新生成。
//!
//! - 值保留 ICU 插值占位符（如 \`{duration} 秒\`），Rust 侧用 \`format()\` 按名替换；
//! - 共 ${entries.length} 条；查表走二分（表已按字典序排列）。
//!
//! 未收录的 id 返回 \`id\` 本身 —— 与真源 \`formatMessage\` 找不到 key 时的行为一致，
//! 不会因为漏一条就崩，但界面会显示原始 id（便于发现漏迁）。

const MESSAGES: &[(&str, &str)] = &[
${rows}
];

/// 查文案（未收录时回退成 id 本身）。
pub fn text(id: &str) -> String {
    lookup(id).unwrap_or(id).to_string()
}

/// 查文案并做 \`{name}\` 插值。
///
/// 占位符按名替换：值为 \`None\` 或不命中时保留原 \`{name}\`，
/// 这样肉眼能看出缺参数（与真源 react-intl 的行为不完全一致，
/// 真源会抛错；这里选择宽松，避免单条缺参炸掉整张卡）。
pub fn format(id: &str, values: &[(String, String)]) -> String {
    let mut out = text(id);
    for (key, value) in values {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

fn lookup(key: &str) -> Option<&'static str> {
    MESSAGES
        .binary_search_by(|(k, _)| (*k).cmp(key))
        .ok()
        .map(|idx| MESSAGES[idx].1)
}

#[cfg(test)]
mod tests {
    use super::{format, lookup, text};

    #[test]
    fn table_is_sorted_for_binary_search() {
        // 二分查找的前提：表必须按 key 有序。生成脚本已排序，这里防回归。
        super::MESSAGES
            .windows(2)
            .for_each(|w| assert!(w[0].0 < w[1].0, "未按字典序: {} >= {}", w[0].0, w[1].0));
    }

    #[test]
    fn keys_are_unique() {
        let mut keys: Vec<&str> = super::MESSAGES.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), before, "存在重复 key");
    }

    #[test]
    fn known_cua_messages_match_source() {
        // 逐条对齐 packages/ui/src/i18n/locales/zh-CN.ts。
        assert_eq!(text("chat.toolCall.cua.appName"), "电脑控制");
        assert_eq!(text("chat.toolCall.cua.leftClick"), "单击");
        assert_eq!(text("chat.toolCall.cua.rightClick"), "右键单击");
        assert_eq!(text("chat.toolCall.cua.type"), "输入文本");
        assert_eq!(text("chat.toolCall.cua.elementTarget"), "元素 #{index}");
        assert_eq!(text("chat.toolCall.cua.holdKey"), "长按按键");
        assert_eq!(text("chat.toolCall.cua.pressKeyAction"), "按下");
        assert_eq!(text("chat.toolCall.cua.listWindowsCount"), "{count} 个窗口");
        assert_eq!(text("chat.toolCall.cua.seconds"), "{duration} 秒");
        assert_eq!(text("chat.toolCall.cua.default"), "使用 Computer Use");
    }

    #[test]
    fn known_detail_messages_match_source() {
        assert_eq!(text("chat.toolCall.cua.details.elementTarget"), "界面元素 #{index}");
        assert_eq!(text("chat.toolCall.cua.details.coordinateTarget"), "坐标 {x}, {y}");
        assert_eq!(text("chat.toolCall.cua.details.completed"), "操作已完成");
        assert_eq!(text("chat.toolCall.cua.details.failed"), "操作失败");
        assert_eq!(text("chat.toolCall.cua.details.elementStale"), "元素已失效");
        assert_eq!(text("chat.toolCall.cua.details.accessReady"), "Computer Use 已就绪");
    }

    #[test]
    fn format_interpolates_by_name() {
        assert_eq!(
            format("chat.toolCall.cua.elementTarget", &[("index".into(), "57".into())]),
            "元素 #57"
        );
        assert_eq!(
            format(
                "chat.toolCall.cua.details.coordinateTarget",
                &[("x".into(), "10".into()), ("y".into(), "20".into())]
            ),
            "坐标 10, 20"
        );
        assert_eq!(
            format("chat.toolCall.cua.listWindowsCount", &[("count".into(), "3".into())]),
            "3 个窗口"
        );
    }

    #[test]
    fn format_keeps_placeholder_when_param_missing() {
        // 缺参数时保留占位符，肉眼可辨（不抛错）。
        assert_eq!(format("chat.toolCall.cua.seconds", &[]), "{duration} 秒");
    }

    #[test]
    fn unknown_id_falls_back_to_itself() {
        assert_eq!(text("chat.toolCall.not.migrated"), "chat.toolCall.not.migrated");
        assert_eq!(lookup("chat.toolCall.not.migrated"), None);
    }

    #[test]
    fn values_with_quotes_and_braces_survive_roundtrip() {
        // 真源里有带引号与花括号的值（如 JSON 片段示例），转义必须正确。
        for (id, value) in super::MESSAGES {
            assert_eq!(
                text(id),
                *value,
                "id={id} 查表结果与生成值不一致（生成脚本转义有误）"
            );
        }
    }
}
`;

fs.mkdirSync(path.join(here, "../frontend/src/ToolCallBlocks"), { recursive: true });
fs.writeFileSync(
  path.join(here, "../frontend/src/ToolCallBlocks/i18n.rs"),
  out,
);
console.log(`i18n messages: ${entries.length}`);