// 素材大袋拆分（地基清理·展厅瘦身 2026-10-05）：把 UIverse 自动收录的 3,793 件
// （运行时 ~27MB：previewHtml 13.6MB + 图纸 11.9MB）从"进门整袋解析"拆成
//   - galaxy-meta.ts：只有标题/描述/标签/分类/口令等轻字段（~1.4MB），展厅进门拉它；
//   - galaxy-bodies/<源分类>.ts：previewHtml+图纸真身（每个 chunk ~2-3MB），
//     卡片滚进缓冲带/打开代码面板/递活时才按需拉。
// 派生关系：uiverse-*.ts（import-uiverse-galaxy.mjs 的产物）→ 本脚本 → galaxy-*。
// 重跑生成器后必须重跑本脚本；输出确定性（无时间戳），reverify 会做一致性对账。
// 运行：npx tsx scripts/split-asset-galaxy.mts
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(scriptDir, "..");
const generatedDir = join(
  repoRoot,
  "packages",
  "ui",
  "src",
  "asset-library",
  "catalog",
  "generated",
);

const SOURCE_CHUNKS = [
  "buttons",
  "cards",
  "checkbox",
  "forms",
  "inputs",
  "loaders",
  "notifications",
  "patterns",
  "radio-button",
  "toggle-switch",
  "tooltips",
];

const modules = await Promise.all(
  SOURCE_CHUNKS.map(async (chunk) => {
    const mod = await import(pathToFileURL(join(generatedDir, `uiverse-${chunk}.js`)).href);
    const list = mod[`UIVERSE_AUTO_${chunk.toUpperCase().replace(/-/g, "_")}`];
    if (!Array.isArray(list) || list.length === 0) {
      throw new Error(`源分类 ${chunk} 缺数组 UIVERSE_AUTO_${chunk.toUpperCase().replace(/-/g, "_")}`);
    }
    return { chunk, list };
  }),
);

const seenIds = new Set();
const metaEntries = [];
let bodyCount = 0;
for (const { chunk, list } of modules) {
  const bodies = {};
  for (const manifest of list) {
    if (seenIds.has(manifest.id)) {
      throw new Error(`id 重复：${manifest.id}`);
    }
    seenIds.add(manifest.id);
    bodies[manifest.id] = { previewHtml: manifest.previewHtml, files: manifest.files };
    bodyCount += 1;
    metaEntries.push({
      ...manifest,
      previewHtml: "",
      files: [],
      filesCount: manifest.files.length,
      bodyFrom: `galaxy:${chunk}`,
    });
  }
  const bodyFile = join(generatedDir, "galaxy-bodies", `${chunk}.ts`);
  await mkdir(dirname(bodyFile), { recursive: true });
  // 巨型字面量直接标类型会撞 TS2590（union type too complex），统一走
  // JSON.parse 发射：转义安全、TS 零推断负担、运行时解析也更快。
  const bodyPayload = JSON.stringify(JSON.stringify(bodies));
  await writeFile(
    bodyFile,
    `// 派生产物（scripts/split-asset-galaxy.mts）：uiverse-${chunk}.ts 的正文真身，勿手改。\n` +
      "// 由展厅按需拉取（assetBodies.ts 的 bodyFrom=\"galaxy:" + chunk + "\" 走线）；\n" +
      "// 重跑 import-uiverse-galaxy.mjs 后必须重跑拆分脚本保持一致。\n" +
      'import type { AssetFile } from "../../types.js";\n\n' +
      "export interface GalaxyBody {\n  previewHtml: string;\n  files: AssetFile[];\n}\n\n" +
      "export const GALAXY_BODIES: Record<string, GalaxyBody> = JSON.parse(\n  " +
      bodyPayload +
      ",\n) as Record<string, GalaxyBody>;\n",
    "utf8",
  );
  console.log(`galaxy-bodies/${chunk}.ts：${Object.keys(bodies).length} 件`);
}

const metaFile = join(generatedDir, "galaxy-meta.ts");
const metaPayload = JSON.stringify(JSON.stringify(metaEntries));
await writeFile(
  metaFile,
  "// 派生产物（scripts/split-asset-galaxy.mts）：UIverse 大袋的轻字段清单，勿手改。\n" +
    "// 展厅进门只拉这份；previewHtml/files 真身在 galaxy-bodies/<源分类>.ts，\n" +
    "// 按 bodyFrom 经 assetBodies.ts 按需解析。重跑生成器后必须重跑拆分脚本。\n" +
    'import type { AssetManifest } from "../types.js";\n\n' +
    "export const GALAXY_META: AssetManifest[] = JSON.parse(\n  " +
    metaPayload +
    ",\n) as AssetManifest[];\n",
  "utf8",
);
console.log(`galaxy-meta.ts：${metaEntries.length} 件（正文 chunk 共 ${bodyCount} 件）`);
if (metaEntries.length !== bodyCount) {
  throw new Error("meta 与正文件数不一致");
}
