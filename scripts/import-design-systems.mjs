#!/usr/bin/env node
/**
 * 一次性生成器：把 open-design 的设计风格库（143 个 DESIGN.md）
 * 转成素材库的「设计风格」货（prompt 类：说明书就是货）。
 *
 * 来源：github.com/nexu-io/open-design · design-systems/（顶层权威目录，官方插件目录是其镜像）
 * 许可：仓库 Apache-2.0，每份风格 open-design.json 标 MIT（见各卡 source 字段）。
 *
 * 用法：node scripts/import-design-systems.mjs <open-design 的 design-systems 目录> [输出目录]
 * 产物：<输出目录>/assets/design-*.ts（每风格一卡）+ 打印 index 注册片段。
 * 只在备货时跑一次，产物随代码提交（与 build-asset-previews 同模式）。
 */
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join } from "node:path";

const sourceRoot = process.argv[2];
const outDir = process.argv[3] ?? "packages/ui/src/asset-library/catalog/assets";
if (!sourceRoot || !existsSync(sourceRoot)) {
  console.error("用法：node scripts/import-design-systems.mjs <design-systems 目录> [输出目录]");
  process.exit(1);
}

const { readdirSync } = await import("node:fs");
const dirs = readdirSync(sourceRoot, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name)
  .sort();

mkdirSync(outDir, { recursive: true });

/** 从 DESIGN.md 头部引语里取一句话描述（跳过 "> Category:" 行）。 */
function descriptionFromDesignMd(designMd) {
  const lines = (designMd.match(/^> .+$/gm) ?? [])
    .map((line) => line.replace(/^> /, "").trim())
    .filter((line) => !/^Category:/i.test(line));
  return lines.slice(0, 2).join(" ");
}

/**
 * 风格元数据：优先 open-design.json（plugins/_official 旧布局），
 * 回退 manifest.json + DESIGN.md 引语（仓库顶层 design-systems/ 新布局）。
 */
function readMeta(dir, designMd) {
  const jsonPath = join(sourceRoot, dir, "open-design.json");
  if (existsSync(jsonPath)) {
    const meta = JSON.parse(readFileSync(jsonPath, "utf8"));
    return {
      title: meta.title ?? dir,
      description: meta.description ?? "",
      license: meta.license ?? "MIT",
      tags: (meta.tags ?? []).filter((tag) => tag !== "design-system"),
    };
  }
  const manifestPath = join(sourceRoot, dir, "manifest.json");
  const manifest = existsSync(manifestPath) ? JSON.parse(readFileSync(manifestPath, "utf8")) : {};
  return {
    title: manifest.name ?? dir,
    description: descriptionFromDesignMd(designMd) || manifest.description || "",
    license: "MIT",
    tags: manifest.category ? [manifest.category.toLowerCase().replace(/[^a-z0-9]+/g, "-")] : [],
  };
}

/** 风格卡的口令：让智能体照随附的 DESIGN.md 改造项目（规范全文走文件引用 chip，不内联）。 */
function buildPrompt(title) {
  return [
    `请按「${title}」这套设计风格改造我的界面。`,
    "",
    `完整设计规范在随附的 DESIGN.md 里（配色/字体/组件/布局/层次/明暗与响应式/该做与不该做），请先读它再动手，严格照规范执行；先看我项目现有结构，融入而不是覆盖，改完告诉我动了哪些文件。`,
  ].join("\n");
}

/** 说明页：风格卡不自带可跑预览，给一张"规范摘要页"（色板+说明）。 */
function renderPreview(title, description, designMd) {
  const accent = /#[0-9a-fA-F]{6}/.exec(designMd)?.[0] ?? "#8b8b8b";
  const swatches = [...new Set(designMd.match(/#[0-9a-fA-F]{6}/g) ?? [])].slice(0, 8);
  const sectionCount = (designMd.match(/^## /gm) ?? []).length;
  return `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><style>
*{box-sizing:border-box;margin:0}
body{min-height:100%;display:grid;place-items:center;background:#0b0f17;font-family:system-ui,'PingFang SC','Microsoft YaHei',sans-serif;color:#e8eefb;padding:24px;overflow:hidden}
.card{width:100%;max-width:520px;border:1px solid #232a38;border-radius:16px;background:linear-gradient(180deg,#121826,#0d1220);padding:20px 22px;display:flex;flex-direction:column;gap:12px}
.badge{align-self:flex-start;border-radius:999px;background:rgba(139,139,139,.16);border:1px solid ${accent}55;color:${accent};font-size:11px;letter-spacing:.08em;padding:3px 10px}
h1{font-size:20px}
p{font-size:13px;line-height:1.6;color:#9fb0c8}
.sw{display:flex;gap:8px;flex-wrap:wrap}
.sw i{width:28px;height:28px;border-radius:8px;border:1px solid rgba(255,255,255,.12);display:block}
.meta{font-size:12px;color:#6b7d97}
</style></head><body><div class="card">
<span class="badge">设计风格 · DESIGN.md</span>
<h1>${title}</h1>
<p>${description || "整套设计规范（配色/字体/组件/布局/层次/明暗/响应式/该做与不该做）。"}</p>
<div class="sw">${swatches.map((color) => `<i style="background:${color}"></i>`).join("")}</div>
<p class="meta">共 ${sectionCount} 章规范 · 口令卡：发进会话让智能体照它改造你的界面</p>
</div></body></html>`;
}

/** 序列化成 TS 字符串字面量（统一用 JSON.stringify 保证转义安全）。 */
function ts(value) {
  return JSON.stringify(value);
}

let written = 0;
const registrations = [];
const usedVarNames = new Set();
for (const dir of dirs) {
  const designPath = join(sourceRoot, dir, "DESIGN.md");
  if (!existsSync(designPath)) continue;
  const designMd = readFileSync(designPath, "utf8").trim();
  const meta = readMeta(dir, designMd);
  const id = `design-${dir}`;
  let varName = `${dir.replace(/[^a-zA-Z0-9]+(.)/g, (_, c) => c.toUpperCase()).replace(/[^a-zA-Z0-9]/g, "")}DesignAsset`;
  if (/^\d/.test(varName)) varName = `s${varName}`;
  if (usedVarNames.has(varName)) varName = `${varName}${written}`;
  usedVarNames.add(varName);
  const prompt = buildPrompt(meta.title);

  const file = `import type { AssetManifest } from "../types.js";

/**
 * 「${meta.title}」设计风格卡（V3-3 收录）。
 *
 * 来源：nexu-io/open-design · design-systems/${dir}
 * 许可：Apache-2.0（仓库）/ ${meta.license}（本风格声明）
 * 本卡为逐字收录的设计规范文本（DESIGN.md），口令让智能体照它改造用户界面。
 */
export const ${varName}: AssetManifest = {
  id: ${ts(id)},
  title: ${ts(`${meta.title} · 设计风格`)},
  description: ${ts(meta.description ? `设计风格：${meta.description}` : "整套设计规范，照它改造你的界面。")},
  category: "design-style",
  tags: ${ts(["设计风格", "design-system", ...meta.tags.slice(0, 3)])},
  previewHtml: ${ts(renderPreview(meta.title, meta.description, designMd))},
  files: [{ name: "DESIGN.md", language: "md", content: ${ts(designMd)} }],
  prompt: ${ts(prompt)},
  source: {
    site: "open-design",
    url: ${ts(`https://github.com/nexu-io/open-design/tree/main/design-systems/${dir}`)},
    license: ${ts(`Apache-2.0 / ${meta.license}`)},
  },
};
`;

  writeFileSync(join(outDir, `${id}.ts`), file, "utf8");
  written += 1;
  registrations.push({ id, varName, file: `./assets/${id}.js` });
}

console.log(`[ok] 写入 ${written} 张设计风格卡到 ${outDir}`);
if (process.env.PRINT_INDEX === "1") {
  console.log("--- index 注册片段 ---");
  for (const entry of registrations) {
    console.log(`import { ${entry.varName} } from "${entry.file}";`);
  }
  console.log("const DESIGN_ASSETS = [");
  for (const entry of registrations) console.log(`  ${entry.varName},`);
  console.log("];");
}
