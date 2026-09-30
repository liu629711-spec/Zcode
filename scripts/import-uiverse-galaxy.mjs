#!/usr/bin/env node
/**
 * 一次性生成器：把 uiverse-io/galaxy 的社区组件（3,800+ 份自包含 HTML，MIT）
 * 批量转成素材库数据，逐字收录上游 HTML。
 *
 * 来源：github.com/uiverse-io/galaxy · MIT（署名原作者与 UIverse.io）
 * 产物：<输出目录>/uiverse-<分类>.ts（每分类一文件，uiverseRuntime.uiverseAutoAsset 包壳：
 *       图纸 = 署名头 + 上游原文；previewHtml 惰性生成，不在数据里落地，避免体积翻倍）。
 * 用法：node scripts/import-uiverse-galaxy.mjs <galaxy仓库目录> [输出目录]
 * 只在备货时跑一次，产物随代码提交（与 import-design-systems 同模式）。
 */
import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const sourceRoot = process.argv[2];
const outDir = process.argv[3] ?? "packages/ui/src/asset-library/catalog/generated";
if (!sourceRoot || !existsSync(sourceRoot)) {
  console.error("用法：node scripts/import-uiverse-galaxy.mjs <galaxy 仓库目录> [输出目录]");
  process.exit(1);
}

/** 上游目录 → 素材分类映射（zh = 展示与检索用的中文类别词）。 */
const CATEGORIES = [
  { dir: "Buttons", slug: "buttons", zh: "按钮", assetCategory: "control" },
  { dir: "Cards", slug: "cards", zh: "卡片", assetCategory: "block" },
  { dir: "Checkboxes", slug: "checkbox", zh: "复选框", assetCategory: "control" },
  { dir: "Forms", slug: "forms", zh: "表单", assetCategory: "block" },
  { dir: "Inputs", slug: "inputs", zh: "输入框", assetCategory: "control" },
  { dir: "Notifications", slug: "notifications", zh: "通知", assetCategory: "block" },
  { dir: "Patterns", slug: "patterns", zh: "纹理背景", assetCategory: "background" },
  { dir: "Radio-buttons", slug: "radio-button", zh: "单选框", assetCategory: "control" },
  { dir: "Toggle-switches", slug: "toggle-switch", zh: "开关", assetCategory: "control" },
  { dir: "Tooltips", slug: "tooltips", zh: "提示气泡", assetCategory: "control" },
  { dir: "loaders", slug: "loaders", zh: "加载动画", assetCategory: "control" },
];

/** 单件体积上限：galaxy 里个别文件塞了巨型内联图（最大 186KB），这类不是纯控件图纸。 */
const MAX_BYTES = 48 * 1024;
const KEBAB_CASE = /^[a-z0-9]+(-[a-z0-9]+)*$/;

function kebab(value) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function prettyName(slug) {
  return slug
    .split("-")
    .filter(Boolean)
    .map((word) => (/^\d/.test(word) ? word : word.charAt(0).toUpperCase() + word.slice(1)))
    .join(" ");
}

/** 上游署名注释：`From Uiverse.io by {作者}  - Tags: {逗号分隔}`（Tags 可能为空）。 */
function parseComment(html) {
  const match = /From Uiverse\.io by (.+?)\s+-\s+Tags:\s*([^\r\n]*)/.exec(html);
  if (!match) return { author: null, tags: [] };
  // 行尾可能带注释收口符（`Tags: button */`），剥掉再拆
  const tags = match[2]
    .replace(/\*\/\s*$/, "")
    .split(",")
    .map((tag) => tag.trim())
    .filter(Boolean);
  return { author: match[1].trim(), tags };
}

function buildPrompt(zh, pretty) {
  return [
    `请把这件 UIverse 社区${zh}「${pretty}」装进我的项目。`,
    "",
    "随附图纸是自包含的 HTML+CSS（可能含少量 SVG），请保留它的视觉与交互效果，改写成我项目的技术栈与样式体系；类名加前缀避免冲突，补齐 hover/focus/禁用态与 aria 语义。先看我项目现有同类组件，融入而不是覆盖，改完告诉我动了哪些文件。",
  ].join("\n");
}

const ts = (value) => JSON.stringify(value);

mkdirSync(outDir, { recursive: true });

let writtenTotal = 0;
let skippedOversize = 0;
const usedIds = new Set();
const summary = [];

for (const category of CATEGORIES) {
  const dirPath = join(sourceRoot, category.dir);
  if (!existsSync(dirPath)) {
    console.error(`[warn] 缺少分类目录：${dirPath}`);
    continue;
  }
  const entries = [];
  for (const name of readdirSync(dirPath).sort()) {
    if (!name.endsWith(".html")) continue;
    const filePath = join(dirPath, name);
    if (statSync(filePath).size > MAX_BYTES) {
      skippedOversize += 1;
      continue;
    }
    const html = readFileSync(filePath, "utf8");
    const { author: commentAuthor, tags } = parseComment(html);
    const stem = name.slice(0, -".html".length);
    const underscore = stem.indexOf("_");
    const fileAuthor = underscore > 0 ? stem.slice(0, underscore) : stem;
    const nameSlug = underscore > 0 ? stem.slice(underscore + 1) : stem;
    const author = commentAuthor ?? fileAuthor;

    let id = kebab(`uiverse-${category.slug}-${fileAuthor}-${nameSlug || "element"}`);
    if (!KEBAB_CASE.test(id)) id = kebab(id.replace(/[^a-z0-9-]/g, ""));
    while (usedIds.has(id)) id = `${id}-${entries.length + 1}`;
    usedIds.add(id);

    const pretty = prettyName(nameSlug || "element");
    const tagList = [...new Set([category.zh, "uiverse", ...tags.slice(0, 3)])];
    entries.push({
      id,
      title: `${category.zh} · ${pretty}`,
      titleEn: pretty,
      description: `UIverse 社区组件 · 作者 ${author}。${
        tags.length ? `标签：${tags.slice(0, 4).join(" / ")}。` : "纯 HTML+CSS 实现。"
      }预览即真实效果，图纸可整段复制。`,
      category: category.assetCategory,
      tags: tagList,
      prompt: buildPrompt(category.zh, pretty),
      author,
      origin: `${category.dir}/${name}`,
      html,
    });
  }

  const varName = `UIVERSE_AUTO_${category.slug.toUpperCase().replace(/-/g, "_")}`;
  const file = `/**
 * UIverse（uiverse-io/galaxy）自动收录 · ${category.dir} · ${entries.length} 件。
 * 由 scripts/import-uiverse-galaxy.mjs 生成，勿手改；口径见同目录 uiverseRuntime.ts。
 * 许可：MIT（仓库 LICENSE）· 署名原作者与 UIverse.io。
 */
import type { AssetManifest } from "../types.js";
import { uiverseAutoAsset } from "./uiverseRuntime.js";

export const ${varName}: AssetManifest[] = [
${entries
  .map(
    (entry) => `  uiverseAutoAsset({
    id: ${ts(entry.id)},
    title: ${ts(entry.title)},
    titleEn: ${ts(entry.titleEn)},
    description: ${ts(entry.description)},
    category: ${ts(entry.category)},
    tags: ${ts(entry.tags)},
    prompt: ${ts(entry.prompt)},
    author: ${ts(entry.author)},
    origin: ${ts(entry.origin)},
    html: ${ts(entry.html)},
  }),`,
  )
  .join("\n")}
];
`;

  writeFileSync(join(outDir, `uiverse-${category.slug}.ts`), file, "utf8");
  writtenTotal += entries.length;
  summary.push(`${category.dir}: ${entries.length}`);
}

console.log(`[ok] 写入 ${writtenTotal} 件 UIverse 组件到 ${outDir}`);
console.log(`     ${summary.join(" · ")}`);
if (skippedOversize > 0) console.log(`[skip] ${skippedOversize} 件超过 ${MAX_BYTES / 1024}KB 未收录`);
