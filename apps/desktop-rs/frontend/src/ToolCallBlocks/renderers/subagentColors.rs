//! 1:1 翻译 `packages/ui/src/lib/subagentColors.ts`（46 行）。
//!
//! 子代理配色：8 色词表 + 名字哈希取色 + 文本/底色类名表。
//! Agent 卡用颜色区分不同子代理类型（同名恒定色，视觉上可追踪同一代理）。

/// `SUBAGENT_COLORS`（真源 :3-12）——**顺序参与哈希取模，不可重排**。
pub const SUBAGENT_COLORS: [&str; 8] = [
    "yellow", "red", "orange", "green", "cyan", "blue", "purple", "pink",
];

/// `SUBAGENT_TEXT_COLOR_CLASS`（真源 :25-34）。
pub fn subagent_text_color_class(color: &str) -> &'static str {
    match color {
        "blue" => "text-sky-700 dark:text-sky-300",
        "cyan" => "text-cyan-700 dark:text-cyan-300",
        "green" => "text-emerald-700 dark:text-emerald-300",
        "orange" => "text-orange-700 dark:text-orange-300",
        "pink" => "text-pink-700 dark:text-pink-300",
        "purple" => "text-violet-700 dark:text-violet-300",
        "red" => "text-rose-700 dark:text-rose-300",
        "yellow" => "text-amber-700 dark:text-amber-300",
        _ => "",
    }
}

/// `isSubagentColor`（真源 :36-38）。
pub fn is_subagent_color(value: &str) -> bool {
    SUBAGENT_COLORS.contains(&value)
}

/// `resolveSubagentColorFromName`（真源 :40-46）：
/// `hash = (hash * 31 + charCode) >>> 0`（无符号 32 位回绕），取模定色。
pub fn resolve_subagent_color_from_name(name: &str) -> &'static str {
    let mut hash: u32 = 0;
    for ch in name.chars() {
        // TS 的 charCodeAt 是 UTF-16 code unit；对 BMP 外字符 Rust chars() 与
        // JS 的逐 code unit 不同——子代理名基本是 ASCII/BMP 标识符，按 chars 处理。
        for unit in ch.encode_utf16(&mut [0; 2]).iter().take_while(|u| **u != 0) {
            hash = hash.wrapping_mul(31).wrapping_add(*unit as u32);
        }
    }
    SUBAGENT_COLORS[(hash as usize) % SUBAGENT_COLORS.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_order_is_stable() {
        // 顺序参与哈希取模——重排会改变所有代理的颜色。
        assert_eq!(SUBAGENT_COLORS[0], "yellow");
        assert_eq!(SUBAGENT_COLORS[7], "pink");
    }

    #[test]
    fn name_hash_is_deterministic() {
        // 同名恒同色（哈希确定性）。
        let a = resolve_subagent_color_from_name("general-purpose");
        let b = resolve_subagent_color_from_name("general-purpose");
        assert_eq!(a, b);
        assert!(is_subagent_color(a));
        // 与 TS 算法对照：手算 "a" → hash = 97 → 97 % 8 = 1 → "red"。
        assert_eq!(resolve_subagent_color_from_name("a"), "red");
        // 空名 → hash 0 → "yellow"。
        assert_eq!(resolve_subagent_color_from_name(""), "yellow");
    }

    #[test]
    fn text_classes_cover_all_colors() {
        for color in SUBAGENT_COLORS {
            assert!(
                !subagent_text_color_class(color).is_empty(),
                "{color} 应有类名"
            );
        }
        assert_eq!(subagent_text_color_class("bogus"), "");
    }
}
