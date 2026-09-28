#!/usr/bin/env node
/**
 * react 类素材预览产物生成脚本（技术设计《ZCode-交互素材库-技术设计.md》§2 + §10 V3-3）。
 *
 * 重新生成：仓库根执行 `node scripts/build-asset-previews.mjs`
 * 时机：改了任何 react 货的 demo 源码（catalog/assets/*.ts 里 files 的 tsx 字符串）
 *      后跑一次，把生成的 packages/ui/src/asset-library/catalog/preview-html.ts
 *      一起提交。产物是静态文件，不挂任何构建流水线；别手改生成文件。
 *
 * 两条产线：
 *  A. 自制 react 货（`files[0].language === "tsx"` 且无 `preview` 字段）——
 *     与旧版一致：demo 源码 + react/react-dom 打成 IIFE（react 内联进产物），
 *     套自包含 HTML，写进 REACT_PREVIEW_HTML。
 *  B. V3-3 逐字收录货（有 `preview` 字段，见 types.ts）——上游是 React+Tailwind
 *     （RareUI / Beautiful UI），需要两件构建期能力：
 *       1) tailwind v4 编译：用 @tailwindcss/oxide 的 Scanner 扫源码出候选类名，
 *          tailwindcss 的 compile() 把候选 + 上游设计 token（scripts/asset-preview-themes/
 *          <theme>.css，取自上游 app/globals.css）编成一份 CSS；
 *       2) 运行时分块：react / motion / three / liveline / iconoir / lucide / glimm
 *          各打成一块（`globalThis.__ZCODE_ASSET_PREVIEW__` 注册表），每件的组件体只带
 *          自己用到的块——50 件货共享一份 react，产物不至于几十 MB；拼装由
 *          catalog/previewAssemble.ts 在运行时按需完成（模块级缓存，同一件只拼一次）。
 *     图纸原文（files）原样交付；预览副本只在 `preview.patches` 声明处把上游硬编码的
 *     远程图换成内联 data URI（沙箱禁外链），构建脚本会断言每条 patch 都命中。
 */

import esbuild from "esbuild";
import fs from "node:fs";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const assetsDir = path.join(repoRoot, "packages/ui/src/asset-library/catalog/assets");
const themesDir = path.join(repoRoot, "scripts/asset-preview-themes");
const outFile = path.join(repoRoot, "packages/ui/src/asset-library/catalog/preview-html.ts");
const reactPackageJson = path.join(repoRoot, "node_modules/react/package.json");
const tailwindEntryCss = path.join(repoRoot, "node_modules/tailwindcss/index.css");

const requireFromRepo = createRequire(path.join(repoRoot, "package.json"));
const { compile } = requireFromRepo("tailwindcss");
const { Scanner } = requireFromRepo("@tailwindcss/oxide");

/** 预览运行时注册表挂在页面全局的键名（拼接产物里的分块都往这里注册）。 */
const RUNTIME_GLOBAL = "__ZCODE_ASSET_PREVIEW__";

/**
 * 运行时分块：块名 → 该块提供的裸模块 id。
 * 一件货用到哪块就带哪块。块里的模块用 `import * as ns` 整体注册，**无法 tree-shake**，
 * 所以只放"整库都要"或"多件共用、重复代价高"的库；图标库（iconoir/lucide 各上千个
 * 导出，整库 2MB+）走逐件打包靠 tree-shaking 压到 10KB 级，绝不进块。
 */
const CHUNKS = {
  react: ["react", "react-dom/client", "react/jsx-runtime"],
  motion: ["framer-motion", "motion/react"],
  three: ["three"],
  liveline: ["liveline"],
  glimm: ["glimm"],
};

/** 裸模块 id → 块名（组件源码里出现这些 import 就要求对应块）。 */
/** 块依赖序：motion 顶层解构 react 注册导出，react 必须最先注册；字母序会把 motion 排前面（真机白屏真因）。 */
const CHUNK_ORDER = Object.keys(CHUNKS);

const MODULE_TO_CHUNK = Object.fromEntries(
  Object.entries(CHUNKS).flatMap(([chunk, modules]) => modules.map((id) => [id, chunk])),
);

/**
 * 逐件打包的裸模块（**不许进块**）：图标库与工具库。它们导出多、按需取用，
 * esbuild 逐件打包时能 tree-shake（iconoir 10 个图标 9KB、lucide 20 个 11KB），
 * 塞进块要保留全量导出（2MB+），反而更贵。
 */
const PER_ASSET_MODULES = new Set([
  "iconoir-react",
  "lucide-react",
  "clsx",
  "tailwind-merge",
  // next/* 在上游是框架依赖，预览里换成 NEXT_SHIMS 的等价物（逐件内联，很小）
  "next/image",
  "next/link",
  "next/navigation",
  "next/dynamic",
]);

/** V3-3 预览专用 polyfill：上游组件引了 next/* 的东西，沙箱里给等价物。 */
const NEXT_SHIMS = {
  "next/image": `import { createElement } from "react";
export default function Image({ src, alt, width, height, fill, priority, ...rest }) {
  return createElement("img", { src: typeof src === "string" ? src : src?.src, alt: alt ?? "", width, height, ...rest });
}
`,
  "next/link": `import { createElement } from "react";
export default function Link({ href, children, prefetch, replace, scroll, shallow, passHref, legacyBehavior, ...rest }) {
  return createElement("a", { href: typeof href === "string" ? href : href?.pathname ?? "#", ...rest }, children);
}
`,
  "next/navigation": `export function useRouter() {
  return { push() {}, replace() {}, prefetch() {}, back() {}, forward() {}, refresh() {} };
}
export function usePathname() { return "/"; }
export function useSearchParams() { return new URLSearchParams(); }
`,
};

// ============================================================
// A. 旧产线：自制 react 货（demo 即图纸，react 内联）
// ============================================================

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
async function bundleSelfMadeDemo(manifest, tempDir) {
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
    define: { "process.env.NODE_ENV": '"production"' },
    nodePaths: [path.join(repoRoot, "node_modules")],
    write: false,
    logLevel: "silent",
    outdir: "virtual", // write:false 下的占位，产物走 outputFiles
  });
  return result.outputFiles[0].text;
}

// ============================================================
// B. V3-3 产线：逐字收录货（tailwind 编译 + 运行时分块）
// ============================================================

/**
 * 收集源码里的裸模块 id（import/require 字面量），用于推 required chunks。
 * 相对路径（`./x`）与本仓库别名（`@/x`）不算裸模块。
 */
function collectExternalModules(sources) {
  const found = new Set();
  for (const source of sources) {
    for (const match of source.matchAll(/(?:from|require\()\s*["']([^"']+)["']/g)) {
      const id = match[1];
      if (id.startsWith(".") || id.startsWith("@/")) continue;
      found.add(id);
    }
  }
  return [...found].sort();
}

/** 注册表读取式 shim：把块里的真模块按真导出名逐个再导出（tree-shaking 交给 esbuild）。 */
function runtimeShim(moduleId, exportNames) {
  const names = exportNames.filter(
    (name) => name !== "default" && name !== "__esModule" && /^[A-Za-z_$][A-Za-z0-9_$]*$/.test(name),
  );
  const lines = [`const M = globalThis.${RUNTIME_GLOBAL}[${JSON.stringify(moduleId)}];`];
  for (const name of names) lines.push(`export const ${name} = M.${name};`);
  lines.push("export default M;");
  return `${lines.join("\n")}\n`;
}

/**
 * 真模块的导出名（构建期在 Node 里探测一次）。
 * 先 require（CJS 包快），失败再动态 import（ESM-only 的包如 glimm 走这条）。
 * 用模块命名空间本身包一层，返回的键与注册表里的命名空间一致。
 */
async function exportNamesOf(moduleId) {
  try {
    const mod = requireFromRepo(moduleId);
    if (mod && (typeof mod === "object" || typeof mod === "function")) return Object.keys(mod);
  } catch {
    // 落到动态 import
  }
  try {
    const ns = await import(moduleId);
    return Object.keys(ns);
  } catch {
    // 探测不到就退化：shim 只剩 default，源码头 import 具名会在 esbuild 阶段当场报错，不静默
    return [];
  }
}

/**
 * react 的三个模块在非 react 块里的替身：读注册表（react 块总会先执行）。
 * 不这样做的后果：每个块各自内联一份 react，framer-motion 的 hooks 会调到
 * 另一份 React 实例的 dispatcher（经典 Invalid hook call）。
 */
const REACT_SHIM_SOURCES = {
  react: `const R = (globalThis.${RUNTIME_GLOBAL} = globalThis.${RUNTIME_GLOBAL} || {});
const M = R["react"];
export default M;
export const { Children, Component, Fragment, Profiler, PureComponent, StrictMode, Suspense, cloneElement, createContext, createElement, createRef, forwardRef, isValidElement, lazy, memo, startTransition, use, useActionState, useCallback, useContext, useDebugValue, useDeferredValue, useEffect, useId, useImperativeHandle, useInsertionEffect, useLayoutEffect, useMemo, useOptimistic, useReducer, useRef, useState, useSyncExternalStore, useTransition, version } = M;
`,
  "react-dom/client": `const R = (globalThis.${RUNTIME_GLOBAL} = globalThis.${RUNTIME_GLOBAL} || {});
const M = R["react-dom/client"];
export default M;
export const { createRoot, hydrateRoot } = M;
`,
  "react/jsx-runtime": `const R = (globalThis.${RUNTIME_GLOBAL} = globalThis.${RUNTIME_GLOBAL} || {});
const M = R["react/jsx-runtime"];
export default M;
export const { Fragment, jsx, jsxs } = M;
`,
};

/**
 * 打一个运行时块：把块内各模块的命名空间注册进注册表。
 * react 块用真 react/react-dom；其余块的 react 三个入口换成读注册表的替身
 * （见 REACT_SHIM_SOURCES：react 块在拼装时永远排最前，替身读得到）。
 */
async function buildRuntimeChunk(chunkName, moduleIds, tempDir) {
  const shimPaths = {};
  for (const [id, source] of Object.entries(REACT_SHIM_SOURCES)) {
    const file = path.join(tempDir, `chunk-shim-react-${id.replace(/[^A-Za-z0-9]/g, "_")}.js`);
    fs.writeFileSync(file, source);
    shimPaths[id] = file;
  }
  const plugin = {
    name: `runtime-chunk-${chunkName}`,
    setup(build) {
      build.onResolve({ filter: /^react$|^react-dom\/client$|^react\/jsx-runtime$/ }, (args) =>
        chunkName === "react" ? null : { path: shimPaths[args.path] },
      );
    },
  };
  const lines = [`const R = (globalThis.${RUNTIME_GLOBAL} = globalThis.${RUNTIME_GLOBAL} || {});`];
  for (const id of moduleIds) {
    lines.push(`import * as ns_${id.replace(/[^A-Za-z0-9]/g, "_")} from ${JSON.stringify(id)};`);
  }
  for (const id of moduleIds) {
    lines.push(`R[${JSON.stringify(id)}] = ns_${id.replace(/[^A-Za-z0-9]/g, "_")};`);
  }
  const result = await esbuild.build({
    stdin: { contents: lines.join(String.fromCharCode(10)), resolveDir: tempDir, sourcefile: `chunk-${chunkName}.tsx`, loader: "tsx" },
    bundle: true,
    format: "iife",
    minify: true,
    target: "es2020",
    jsx: "automatic",
    define: { "process.env.NODE_ENV": '"production"' },
    nodePaths: [path.join(repoRoot, "node_modules")],
    plugins: [plugin],
    write: false,
    logLevel: "silent",
    outdir: "virtual",
  });
  const js = result.outputFiles[0].text;
  if (chunkName !== "react" && js.includes("Minified React error")) {
    throw new Error(`运行时块 ${chunkName} 里混进了 react-dom 副本（react 必须只有一份）`);
  }
  return js;
}

/** `cn()` 助手源码：上游组件普遍 `import { cn } from "@/lib/utils"`，按上游实现等价重写。 */
const CN_SHIM = `import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
export function cn(...inputs) {
  return twMerge(clsx(inputs));
}
export default cn;
`;

/** 写入一件货的预览源码树：图纸逐字（entry 打离线化 patch），返回入口文件名。 */
function writePreviewSources(manifest, tempDir) {
  let entryName = null;
  for (const [index, file] of manifest.files.entries()) {
    let content = file.content;
    if (index === 0) {
      for (const patch of manifest.preview.patches ?? []) {
        if (!content.includes(patch.from)) {
          throw new Error(`${manifest.id}: 预览 patch 未命中（${patch.note}）：${patch.from.slice(0, 60)}…`);
        }
        content = content.replaceAll(patch.from, patch.to);
      }
      entryName = file.name;
    }
    fs.writeFileSync(path.join(tempDir, path.basename(file.name)), content);
  }
  fs.writeFileSync(path.join(tempDir, "lib-utils.ts"), CN_SHIM);
  fs.writeFileSync(path.join(tempDir, "__demo.tsx"), manifest.preview.demo);
  return entryName;
}

/** 组装 esbuild 别名表：裸模块 → 运行时 shim，@/lib/utils → cn，@/components/** → 同目录同名文件。 */
async function previewAliases(tempDir) {
  const alias = {};
  for (const [chunkName, moduleIds] of Object.entries(CHUNKS)) {
    for (const id of moduleIds) {
      const shimPath = path.join(tempDir, `shim-${id.replace(/[^A-Za-z0-9]/g, "_")}.js`);
      fs.writeFileSync(shimPath, runtimeShim(id, await exportNamesOf(id)));
      alias[id] = shimPath;
    }
  }
  for (const [id, source] of Object.entries(NEXT_SHIMS)) {
    const shimPath = path.join(tempDir, `shim-${id.replace(/[^A-Za-z0-9]/g, "_")}.js`);
    fs.writeFileSync(shimPath, source);
    alias[id] = shimPath;
  }
  return alias;
}

/** tailwind 编译：扫 tempDir 里的源码出候选类名，套上游 token 编成 CSS。 */
async function compileTailwindCss(tempDir, themeName) {
  const themeFile = path.join(themesDir, `${themeName}.css`);
  const themeCss = fs.readFileSync(themeFile, "utf8");
  const entry = `@import "tailwindcss";\n${themeCss}`;
  const scanner = new Scanner({
    sources: [{ base: fs.realpathSync(tempDir), pattern: "**/*.{tsx,ts,js,html}", negated: false }],
  });
  const candidates = [...new Set(scanner.scan())];
  const compiler = await compile(entry, {
    base: tempDir,
    loadStylesheet: (id, base) => {
      const resolved = id === "tailwindcss" ? tailwindEntryCss : path.resolve(base === "<input>" ? tempDir : base, id);
      return { path: resolved, base: path.dirname(resolved), content: fs.readFileSync(resolved, "utf8") };
    },
  });
  return { css: compiler.build(candidates), candidates: candidates.length };
}

/** 一件收录货 → { chunks, css, body }（body 已把 react/motion 等换成注册表读取）。 */
async function buildOpenSourcePreview(manifest, runtimeTmp) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), `asset-${manifest.id}-`));
  try {
    const entryName = writePreviewSources(manifest, tempDir);
    const entryBase = path.basename(entryName, path.extname(entryName));
    const alias = await previewAliases(tempDir);
    alias["@/lib/utils"] = path.join(tempDir, "lib-utils.ts");
    // 图纸之间的相对依赖（如 atoms/Shimmer）在 tempDir 里是平铺的，按 basename 兜底解析
    const flatComponents = {
      name: "flat-components",
      setup(build) {
        build.onResolve({ filter: /^@\/components\// }, (args) => {
          const base = path.basename(args.path);
          const candidate = path.join(tempDir, `${base}.tsx`);
          if (fs.existsSync(candidate)) return { path: candidate };
          return { errors: [{ text: `${manifest.id}: 预览依赖未随图纸提供：${args.path}` }] };
        });
      },
    };
    const result = await esbuild.build({
      stdin: {
        contents: [
          'import { createRoot } from "react-dom/client";',
          'import Demo from "./__demo";',
          'createRoot(document.getElementById("root")).render(<Demo />);',
        ].join("\n"),
        resolveDir: tempDir,
        sourcefile: "mount.tsx",
        loader: "tsx",
      },
      bundle: true,
      format: "iife",
      minify: true,
      target: "es2020",
      jsx: "automatic",
      define: { "process.env.NODE_ENV": '"production"' },
      nodePaths: [path.join(repoRoot, "node_modules")],
      plugins: [flatComponents],
      alias,
      write: false,
      logLevel: "silent",
      outdir: "virtual",
    });
    const body = result.outputFiles[0].text;
    if (/\bDynamic require of /.test(body)) {
      throw new Error(`${manifest.id}: 产物里出现 esbuild 的 require 兜底桩，说明有裸模块没被 shim 住`);
    }
    // 源码头推 required chunks（比扫产物里被 minify 过的注册表读法可靠）
    const usedModules = collectExternalModules([
      ...manifest.files.map((file) => file.content),
      manifest.preview.demo,
    ]);
    const chunks = [
      ...new Set(
        usedModules
          .filter((id) => !PER_ASSET_MODULES.has(id))
          .map((id) => {
            const chunk = MODULE_TO_CHUNK[id];
            if (!chunk) {
              throw new Error(
                `${manifest.id}: 源码 import 了未登记的裸模块 "${id}"（要么在 CHUNKS 开块，要么进 PER_ASSET_MODULES 逐件打包）`,
              );
            }
            return chunk;
          }),
      ),
    ].sort((a, b) => CHUNK_ORDER.indexOf(a) - CHUNK_ORDER.indexOf(b));
    const { css } = await compileTailwindCss(tempDir, manifest.preview.theme);
    if (!css.includes("@layer")) throw new Error(`${manifest.id}: tailwind 产物异常`);
    console.log(
      `[ok] ${manifest.id}: css ${(css.length / 1024).toFixed(0)}KB · body ${(body.length / 1024).toFixed(0)}KB · chunks ${chunks.join("+")}`,
    );
    return { entry: entryBase, chunks, css, body };
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

// ============================================================
// 产物装配
// ============================================================

/** 旧产线模板：套自包含 HTML（样式由 demo 自带内联 <style> 注入）。 */
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

const selfMadeReact = manifests.filter(
  (m) => m.files[0]?.language === "tsx" && !m.preview,
);
const openSourceAssets = manifests.filter((m) => m.preview);
if (selfMadeReact.length === 0) {
  throw new Error("没有找到自制 react 类货（判据 files[0].language === 'tsx' 且无 preview 字段）");
}

const reactVersion = JSON.parse(fs.readFileSync(reactPackageJson, "utf8")).version;
const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "asset-previews-"));

// —— 旧产线 ——
const htmlById = {};
for (const manifest of selfMadeReact) {
  const js = await bundleSelfMadeDemo(manifest, tempDir);
  htmlById[manifest.id] = wrapHtml(js, manifest, reactVersion);
  console.log(`[ok] ${manifest.id}: ${(htmlById[manifest.id].length / 1024).toFixed(0)}KB（自制 react 货）`);
}

// —— V3-3 产线：运行时分块只打用到的那些 ——
const usedChunks = new Set();
for (const manifest of openSourceAssets) {
  for (const id of collectExternalModules([
    ...manifest.files.map((file) => file.content),
    manifest.preview.demo,
  ])) {
    if (PER_ASSET_MODULES.has(id)) continue;
    const chunk = MODULE_TO_CHUNK[id];
    if (!chunk) {
      throw new Error(
        `${manifest.id}: 源码 import 了未登记的裸模块 "${id}"（要么在 CHUNKS 开块，要么进 PER_ASSET_MODULES 逐件打包）`,
      );
    }
    usedChunks.add(chunk);
  }
}
const runtimeChunks = {};
for (const chunkName of [...usedChunks].sort()) {
  runtimeChunks[chunkName] = await buildRuntimeChunk(chunkName, CHUNKS[chunkName], tempDir);
  console.log(`[ok] runtime chunk ${chunkName}: ${(runtimeChunks[chunkName].length / 1024).toFixed(0)}KB`);
}

const openSourcePreviews = {};
for (const manifest of openSourceAssets) {
  const built = await buildOpenSourcePreview(manifest, tempDir);
  openSourcePreviews[manifest.id] = {
    title: manifest.title,
    theme: manifest.preview.theme,
    dark: manifest.preview.dark ?? false,
    stage: manifest.preview.stage ?? "center",
    chunks: built.chunks,
    css: built.css,
    body: built.body,
  };
}
fs.rmSync(tempDir, { recursive: true, force: true });

const chunkEntries = Object.keys(runtimeChunks)
  .sort()
  .map((name) => `  ${JSON.stringify(name)}: ${JSON.stringify(runtimeChunks[name])},`)
  .join("\n");
const openSourceEntries = Object.keys(openSourcePreviews)
  .sort()
  .map((id) => {
    const entry = openSourcePreviews[id];
    return `  ${JSON.stringify(id)}: {\n    title: ${JSON.stringify(entry.title)},\n    dark: ${entry.dark},\n    stage: ${JSON.stringify(entry.stage)},\n    chunks: ${JSON.stringify(entry.chunks)},\n    css: ${JSON.stringify(entry.css)},\n    body: ${JSON.stringify(entry.body)},\n  },`;
  })
  .join("\n");
const selfMadeEntries = Object.keys(htmlById)
  .sort()
  .map((id) => `  ${JSON.stringify(id)}: ${JSON.stringify(htmlById[id])},`)
  .join("\n");

const generated = `/**
 * 本文件由 scripts/build-asset-previews.mjs 生成（仓库根：node scripts/build-asset-previews.mjs）。
 * 改了 react 货 demo 源码（catalog/assets/*.ts）或收录货的预览配置后重新跑一次并提交产物；
 * 不挂构建流水线。别手改本文件。
 *
 * 三张表：
 *  - REACT_PREVIEW_HTML：自制 react 货（demo 即图纸，react 内联进每份产物）。
 *  - PREVIEW_RUNTIME_CHUNKS：V3-3 收录货共享的运行时块（react/motion/three/…），
 *    每件只带自己用到的块，50 件货共用一份 react，产物不必逐件内联。
 *  - OPEN_SOURCE_PREVIEWS：收录货的预览体（tailwind 编好的 CSS + 组件 JS），
 *    由 catalog/previewAssemble.ts 按需拼成自包含 HTML（模块级缓存）。
 *
 * 产物断言在 packages/ui/test/assetPreview.test.ts：每件货都要有产物、
 * 自包含（无外链/无 localStorage）、react 确实在内。
 */
export const REACT_PREVIEW_HTML: Record<string, string> = {
${selfMadeEntries}
};

export const PREVIEW_RUNTIME_CHUNKS: Record<string, string> = {
${chunkEntries}
};

export interface OpenSourcePreviewRecord {
  title: string;
  dark: boolean;
  stage: "center" | "top";
  chunks: string[];
  css: string;
  body: string;
}

export const OPEN_SOURCE_PREVIEWS: Record<string, OpenSourcePreviewRecord> = {
${openSourceEntries}
};
`;
fs.writeFileSync(outFile, generated);
console.log(
  `[ok] wrote ${path.relative(repoRoot, outFile)}：自制 ${Object.keys(htmlById).length} 件 · 收录 ${Object.keys(openSourcePreviews).length} 件 · 运行时块 ${Object.keys(runtimeChunks).length} 个（${(generated.length / 1024 / 1024).toFixed(2)}MB）`,
);
