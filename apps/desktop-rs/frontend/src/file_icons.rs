//! 文件图标名解析（1:1 翻译 \`packages/ui/src/lib/fileDisplayHelpers.ts\`）。
//!
//! 真源逻辑（resolveIconName，137-172 行）：
//! 1. 归一化路径取叶子名、转小写；
//! 2. 候选集 = { 完整名, 去最后一段扩展名 }，再逐段回退 stem（支持 vitest.config.ts 这类多段名）；
//! 3. 先查文件名别名表，命中即返回；
//! 4. 无扩展名 → document；否则查扩展名别名表，未命中则**直接用扩展名本身**当图标名
//!    （前端靠 img onError 降级到 document 图标）。
//!
//! 映射表由 scripts/gen_file_icons.mjs 从真源导出，改映射请改真源后重新生成。

const FILE_NAME_ICON_ALIASES: &[(&str, &str)] = &[
    (".editorconfig", "editorconfig"),
    (".env", "settings"),
    (".gitattributes", "git"),
    (".gitignore", "git"),
    (".npmrc", "npm"),
    (".nvmrc", "nodejs_alt"),
    (".prettierrc", "prettier"),
    (".yarnrc", "yarn"),
    ("babel.config", "babel"),
    ("bun", "lock"),
    ("bun.lock", "lock"),
    ("cargo", "rust"),
    ("cargo.lock", "lock"),
    ("dockerfile", "docker"),
    ("eslint", "eslint"),
    ("eslint.config", "eslint"),
    ("gemfile", "gemfile"),
    ("jest", "jest"),
    ("jest.config", "jest"),
    ("makefile", "makefile"),
    ("package-lock", "lock"),
    ("pnpm-lock", "lock"),
    ("readme", "readme"),
    ("tsconfig", "tsconfig"),
    ("vitest", "vitest"),
    ("vitest.config", "vitest"),
    ("yarn", "yarn"),
];

const EXTENSION_ICON_ALIASES: &[(&str, &str)] = &[
    ("backup", "document"),
    ("bash", "console"),
    ("cjs", "javascript"),
    ("css", "css"),
    ("cts", "typescript"),
    ("doc", "word"),
    ("docx", "word"),
    ("flac", "audio"),
    ("go", "go"),
    ("html", "html"),
    ("java", "java"),
    ("jpeg", "image"),
    ("jpg", "image"),
    ("js", "javascript"),
    ("json", "json"),
    ("jsonl", "json"),
    ("jsx", "react"),
    ("m4a", "audio"),
    ("m4v", "video"),
    ("md", "markdown"),
    ("mjs", "javascript"),
    ("mov", "video"),
    ("mp3", "audio"),
    ("mp4", "video"),
    ("mts", "typescript"),
    ("ogg", "audio"),
    ("opus", "audio"),
    ("pdf", "pdf"),
    ("php", "php"),
    ("png", "image"),
    ("pptx", "powerpoint"),
    ("py", "python"),
    ("responses", "json"),
    ("rs", "rust"),
    ("sb", "storybook"),
    ("sh", "console"),
    ("snap", "snapcraft"),
    ("sql", "database"),
    ("svg", "svg"),
    ("toml", "toml"),
    ("ts", "typescript"),
    ("tsx", "react_ts"),
    ("txt", "document"),
    ("wav", "audio"),
    ("weba", "audio"),
    ("webm", "video"),
    ("xlsx", "table"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("zsh", "console"),
];

pub const DEFAULT_FILE_ICON_NAME: &str = "document";

fn lookup(table: &'static [(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table
        .binary_search_by(|(k, _)| (*k).cmp(key))
        .ok()
        .map(|idx| table[idx].1)
}

/// 解析图标名（与 resolveIconName 同语义）。
pub fn resolve_icon_name(file_path: &str) -> String {
    let normalized = file_path.replace('\\', "/");
    let leaf = match normalized.rfind('/') {
        Some(idx) => &normalized[idx + 1..],
        None => &normalized[..],
    };
    let normalized_leaf = leaf.to_lowercase();

    // 候选：完整名 → 去最后一段扩展名 → 逐段回退 stem。
    // 无扩展名时 stem 就是完整名，候选集退化为单元素——此时仍要先查文件名表
    //（Dockerfile / Makefile 正是靠这一步命中的），查不到才回落 document。
    let dot = normalized_leaf.rfind('.');
    let mut candidates: Vec<&str> = vec![normalized_leaf.as_str()];
    let mut current = match dot {
        Some(idx) => &normalized_leaf[..idx],
        None => normalized_leaf.as_str(),
    };
    if !current.is_empty() {
        candidates.push(current);
    }
    while let Some(idx) = current.rfind('.') {
        current = &current[..idx];
        if !current.is_empty() {
            candidates.push(current);
        }
    }

    for candidate in candidates {
        if let Some(icon) = lookup(FILE_NAME_ICON_ALIASES, candidate) {
            return icon.to_string();
        }
    }

    // 无扩展名 → document。
    let Some(dot) = dot else {
        return DEFAULT_FILE_ICON_NAME.to_string();
    };
    lookup(EXTENSION_ICON_ALIASES, &normalized_leaf[dot + 1..])
        .map(|s| s.to_string())
        .unwrap_or_else(|| normalized_leaf[dot + 1..].to_string())
}

/// 图标资源 URL（真源 buildMaterialFileIconSrc，fileDisplay.tsx:55-57）。
pub fn icon_src(icon_name: &str) -> String {
    format!("/material-icons/{icon_name}.svg")
}

#[cfg(test)]
mod tests {
    use super::resolve_icon_name;

    #[test]
    fn plain_extension_maps_to_alias() {
        assert_eq!(resolve_icon_name("src/main.rs"), "rust");
        assert_eq!(resolve_icon_name("a/b/c.tsx"), "react_ts");
        assert_eq!(resolve_icon_name("package.json"), "json");
    }

    #[test]
    fn whole_filename_wins_over_extension() {
        // Cargo.toml 走文件名别名，不落到 toml。
        assert_eq!(resolve_icon_name("Cargo.toml"), "rust");
        assert_eq!(resolve_icon_name("README.md"), "readme");
        assert_eq!(resolve_icon_name("Dockerfile"), "docker");
        assert_eq!(resolve_icon_name(".gitignore"), "git");
    }

    #[test]
    fn multi_dot_names_fall_back_through_stem() {
        // 逐段回退：package-lock.json → 命中 package-lock（lock）而非 json。
        assert_eq!(resolve_icon_name("package-lock.json"), "lock");
        assert_eq!(resolve_icon_name("tsconfig.base.json"), "tsconfig");
        assert_eq!(resolve_icon_name("vitest.config.ts"), "vitest");
    }

    #[test]
    fn unknown_extension_falls_back_to_extension_itself() {
        // 真源行为：未登记的扩展名直接当图标名，交给前端 img onError 降级。
        assert_eq!(resolve_icon_name("a.unknownext"), "unknownext");
    }

    #[test]
    fn extensionless_and_dotfiles_use_default() {
        assert_eq!(resolve_icon_name("LICENSE"), "document");
        assert_eq!(resolve_icon_name(".npmrc"), "npm", "点文件走文件名别名");
        // 未知点文件：stem 是 randomdotfile，两张表都没有，最终回落成"扩展名本身"
        // （= 去掉点的叶子名），交给前端 img onError 降级到 document 图标。
        assert_eq!(resolve_icon_name(".randomdotfile"), "randomdotfile");
    }

    #[test]
    fn windows_paths_are_normalized() {
        assert_eq!(resolve_icon_name("src\\lib\\index.ts"), "typescript");
        assert_eq!(resolve_icon_name("C:\\ws\\apps\\Cargo.toml"), "rust");
    }
}
