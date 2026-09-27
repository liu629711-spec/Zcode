#!/usr/bin/env node
/**
 * react 类素材预览产物生成脚本（技术设计《ZCode-交互素材库-技术设计.md》§2）。
 *
 * 重新生成：仓库根执行 `node scripts/build-asset-previews.mjs`
 * 时机：改了任何 react 货的 demo 源码（catalog/assets/*.ts 里 files[0] 的 tsx 字符串）
 *      后跑一次，把生成的 packages/ui/src/asset-library/catalog/preview-html.ts
 *      一起提交。产物是静态文件，不挂任何构建流水线；别手改生成文件。
 *
 * 做法：
 *  1. 扫 catalog/assets/*.ts，用 esbuild 把资产模块 bundle 成 CJS 在函数沙盒里执行，
 *     取出导出的 AssetManifest（demo 源码是模板串，不做正则拆串，免转义炸弹）。
 *  2. react 货判据 = files[0].language === "tsx"。把 demo 源码写进临时 .tsx，
 *     连同 react/react-dom（node_modules 解析，React 19 无 UMD，走 bundle 路线）
 *     打成 IIFE：jsx=automatic（产物内联 jsx-runtime，自包含，无需 React 全局）。
 *  3. 套进自包含 HTML 模板（样式由 demo 自带内联 <style>），写出 preview-html.ts：
 *     export const REACT_PREVIEW_HTML: Record<string, string>（key=素材 id）。
 */

import esbuild from "esbuild";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const assetsDir = path.join(repoRoot, "packages/ui/src/asset-library/catalog/assets");
const outFile = path.join(repoRoot, "packages/ui/src/asset-library/catalog/preview-html.ts");
const reactPackageJson = path.join(repoRoot, "node_modules/react/package.json");

/** 资产 .ts → AssetManifest：esbuild bundle 成 CJS 后在函数沙盒里跑，拿导出的清单。 */
async function loadManifest(file) {
  const { outputFiles } = await esbuild.build({
    entryPoints: [file],
    bundle: true,
    platform: "node",
    format: "cjs",
    write: false,
    logLevel: "silent",
  });
  const module = { exports: {} };
  new Function("module", "exports", outputFiles[0].text)(module, module.exports);
  const manifest = Object.values(module.exports).find(
    (value) =>
      value &&
      typeof value === "object" &&
      typeof value.id === "string" &&
      Array.isArray(value.files),
  );
  if (!manifest) throw new Error(`${path.basename(file)}: 没有导出 AssetManifest`);
  return manifest;
}

/** demo tsx + react/react-dom → 自包含 IIFE JS（自动 jsx-runtime，React 全量内联）。 */
async function bundleDemo(manifest, tempDir) {
  const demo = manifest.files[0];
  const demoPath = path.join(tempDir, demo.name);
  fs.writeFileSync(demoPath, demo.content);
  const mount = [
    'import { createElement } from "react";',
    'import { createRoot } from "react-dom/client";',
    `import Demo from ${JSON.stringify(demoPath)};`,
    'createRoot(document.getElementById("root")).render(createElement(Demo));',
  ].join("\n");
  const result = await esbuild.build({
    stdin: { contents: mount, resolveDir: tempDir, sourcefile: "mount.jsx" },
    bundle: true,
    format: "iife",
    minify: true,
    target: "es2020",
    jsx: "automatic",
    nodePaths: [path.join(repoRoot, "node_modules")],
    write: false,
    logLevel: "silent",
    outdir: "virtual", // write:false 下的占位，产物走 outputFiles
  });
  return result.outputFiles[0].text;
}

/** 套自包含 HTML 模板。禁外链由 catalogCheck.validateCatalog 在测试期钉住。 */
function wrapHtml(js, manifest, reactVersion) {
  // JS 里出现 "</script" 会截断标签，转成等价的 "<\/script"
  const safeJs = js.replaceAll("</script", "<\\/script");
  // 打包标记：脚本跑完 bundle 步骤才会写入，测试拿它断言 react 确实内联进产物
  const marker = `/*__ASSET_REACT_BUNDLE__(react@${reactVersion})__*/`;
  return [
    "<!doctype html>",
    '<html lang="zh-CN">',
    "<head>",
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    `<title>${manifest.title}</title>`,
    "</head>",
    "<body>",
    '<div id="root"></div>',
    `<script>${marker}${safeJs}</script>`,
    "</body>",
    "</html>",
  ].join("\n");
}

const assetFiles = fs
  .readdirSync(assetsDir)
  .filter((name) => name.endsWith(".ts"))
  .sort();

const manifests = [];
for (const name of assetFiles) {
  manifests.push(await loadManifest(path.join(assetsDir, name)));
}
const reactManifests = manifests.filter((m) => m.files[0]?.language === "tsx");
if (reactManifests.length === 0) {
  throw new Error("没有找到 react 类货（判据 files[0].language === 'tsx'）");
}

const reactVersion = JSON.parse(fs.readFileSync(reactPackageJson, "utf8")).version;
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "asset-previews-"));
const htmlById = {};
try {
  for (const manifest of reactManifests) {
    const js = await bundleDemo(manifest, tempDir);
    htmlById[manifest.id] = wrapHtml(js, manifest, reactVersion);
    console.log(`[ok] ${manifest.id}: ${(htmlById[manifest.id].length / 1024).toFixed(0)}KB`);
  }
} finally {
  fs.rmSync(tempDir, { recursive: true, force: true });
}

const entries = Object.keys(htmlById)
  .sort()
  .map((id) => `  ${JSON.stringify(id)}: ${JSON.stringify(htmlById[id])},`)
  .join("\n");
const generated = `/**
 * 本文件由 scripts/build-asset-previews.mjs 生成（仓库根：node scripts/build-asset-previews.mjs）。
 * 改了 react 货 demo 源码（catalog/assets/*.ts）后重新跑一次并提交产物；不挂构建流水线。
 * react 货判据 = files[0].language === "tsx"；key = 素材 id。别手改本文件。
 */
export const REACT_PREVIEW_HTML: Record<string, string> = {
${entries}
};
`;
fs.writeFileSync(outFile, generated);
console.log(`[ok] wrote ${path.relative(repoRoot, outFile)} (${Object.keys(htmlById).length} react assets)`);
