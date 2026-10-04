import assert from "node:assert/strict";
import test from "node:test";
import { ASSET_CATALOG } from "../src/asset-library/catalog/index.js";
import {
  ASSET_SANDBOX,
  assertSandboxSafe,
  validateCatalog,
} from "../src/asset-library/catalog/catalogCheck.js";
import {
  OPEN_SOURCE_PREVIEWS,
  PREVIEW_RUNTIME_CHUNKS,
  REACT_PREVIEW_HTML,
} from "../src/asset-library/catalog/preview-html.js";
import {
  assembleOpenSourcePreview,
  orderPreviewChunks,
} from "../src/asset-library/catalog/previewAssemble.js";

// ============================================================
// react 货预编译产物的断言（技术设计 §6：build-asset-previews 产物断言）
// ============================================================
// 钉的是：每件 react 货都有产物、产物自包含（无外链/无 localStorage）、
// react 确实内联进了产物。运行：
// npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/assetPreview.test.ts
// 产物缺失/过期时：仓库根 node scripts/build-asset-previews.mjs 重新生成。
//
// 两条产线（V3-3 起）：
//  - 自制货：demo 即图纸，react 内联进每份产物 → REACT_PREVIEW_HTML。
//  - 收录货（Beautiful UI/RareUI）：上游 React+Tailwind 源码，运行时拆成共享块
//    （PREVIEW_RUNTIME_CHUNKS）+ 每件自己的 CSS/组件体（OPEN_SOURCE_PREVIEWS），
//    由 previewAssemble 运行时拼装。判据是卡片带 preview 字段。

/** 自制 react 货判据：files[0] 是 tsx 且没有 preview 配置。 */
const selfMadeReactAssets = ASSET_CATALOG.filter(
  (asset) => asset.files[0]?.language === "tsx" && !asset.preview,
);
/**
 * V3-3 收录货分两类（与 scripts/build-asset-previews.mjs 同源）：
 *  - React+Tailwind 货（Beautiful UI/RareUI）：卡片带 preview 配置，产物是共享运行时分块。
 *  - 纯 HTML/CSS 片段（UIverse）：previewHtml 直接写在卡里，没有构建期产物。
 */
const reactOpenSourceAssets = ASSET_CATALOG.filter((asset) => asset.preview);
const THREE_SITES = /^(Beautiful UI|RareUI|UIverse)$/;
const threeSiteAssets = ASSET_CATALOG.filter(
  (asset) => asset.source && THREE_SITES.test(asset.source.site),
);

test("react 货都有非空预编译产物，且产物表不收录目录外 id", () => {
  assert.ok(selfMadeReactAssets.length >= 2, "至少要有 2 件自制 react 种子货");
  for (const asset of selfMadeReactAssets) {
    const html = REACT_PREVIEW_HTML[asset.id];
    assert.ok(
      typeof html === "string" && html.trim() !== "",
      `${asset.id} 缺预编译产物（跑 node scripts/build-asset-previews.mjs）`,
    );
  }
  for (const id of Object.keys(REACT_PREVIEW_HTML)) {
    assert.ok(
      ASSET_CATALOG.some((asset) => asset.id === id),
      `产物表收录了目录外的 id：${id}`,
    );
  }
});

test("产物自包含：无外链 script、无 localStorage，react 确实内联", () => {
  for (const [id, html] of Object.entries(REACT_PREVIEW_HTML)) {
    assert.ok(!html.includes('<script src="http'), `${id}: 产物含外链 script`);
    assert.ok(!/src\s*=\s*["']https?:/i.test(html), `${id}: 产物含外链资源`);
    assert.ok(!html.includes("localStorage"), `${id}: 产物含 localStorage（沙箱无源环境会抛）`);
    // react 打包标记由构建脚本在 bundle 成功后写入；"Minified React error" 是
    // react-dom production 里的自然字符串，两道一起钉"react 真被打进去了"
    assert.ok(html.includes("__ASSET_REACT_BUNDLE__"), `${id}: 缺 react 打包标记`);
    assert.ok(html.includes("Minified React error"), `${id}: 缺 react-dom 产物字符串`);
    assert.ok(html.length > 100_000, `${id}: 产物过小（${html.length}B），react 可能没打进去`);
  }
});

test("validateCatalog 对含 react 货的全目录仍全绿，react 货预览已下放惰性 map（瘦身拆分）", () => {
  assert.deepEqual(validateCatalog(ASSET_CATALOG), []);
  for (const asset of selfMadeReactAssets) {
    assert.equal(asset.bodyFrom, "preview-html", `${asset.id}: react 货应标 bodyFrom=preview-html`);
    assert.equal(asset.previewHtml, "", `${asset.id}: previewHtml 应为空串（真身在惰性 map）`);
    assert.ok(
      (REACT_PREVIEW_HTML[asset.id] ?? "").startsWith("<!doctype html>"),
      `${asset.id}: 惰性 map 里应有完整预览产物`,
    );
  }
});

// ============================================================
// V3-3 收录货产物（技术设计 §10 V2-4/V3-3）
// ============================================================
// 钉三件：
//  1) 每件收录货都有产物、产物与其稿件 id 一一对应（没有孤儿产物）；
//  2) react 只有一份——这是共享运行时分块能成立的前提：除 react 块外任何块/组件体
//     都不许再内联一份 React 实现（否则 framer-motion 的 hooks 会调另一个实例的
//     dispatcher，经典 Invalid hook call）；
//  3) 拼装后的 previewHtml 自包含（无外链/无 localStorage）且结构完整。

test("V3-3 收录货都有产物，产物表与目录一一对应", () => {
  assert.ok(reactOpenSourceAssets.length >= 40, `收录 react 货应 ≥40 件，实际 ${reactOpenSourceAssets.length}`);
  for (const asset of reactOpenSourceAssets) {
    const entry = OPEN_SOURCE_PREVIEWS[asset.id];
    assert.ok(entry, `${asset.id} 缺构建产物（跑 node scripts/build-asset-previews.mjs）`);
    assert.ok(entry.body.trim() !== "", `${asset.id}: 组件体为空`);
    assert.ok(entry.css.trim() !== "", `${asset.id}: CSS 为空`);
    assert.ok(entry.chunks.length > 0, `${asset.id}: 没有声明任何运行时块`);
    assert.ok(entry.chunks.includes("react"), `${asset.id}: react 块必须在（组件要跑 React）`);
  }
  for (const id of Object.keys(OPEN_SOURCE_PREVIEWS)) {
    assert.ok(
      ASSET_CATALOG.some((asset) => asset.id === id),
      `产物表收录了目录外的 id：${id}`,
    );
  }
  for (const name of Object.keys(PREVIEW_RUNTIME_CHUNKS)) {
    assert.ok(
      reactOpenSourceAssets.some((asset) => OPEN_SOURCE_PREVIEWS[asset.id]!.chunks.includes(name)),
      `运行时块 ${name} 没有任何货在用（多余产物）`,
    );
  }
});

test("react 只有一份：非 react 块与组件体都不许内联 React 实现", () => {
  // 生产版 react/react-dom 里的特征串；出现在哪个产物里，就说明那里包了一份 React
  const REACT_IMPL = ["Minified React error", "react-stack-bottom-frame"];
  const hasReactImpl = (js: string) => REACT_IMPL.some((marker) => js.includes(marker));

  assert.ok(hasReactImpl(PREVIEW_RUNTIME_CHUNKS.react!), "react 块自己必须是真 React");
  for (const [name, js] of Object.entries(PREVIEW_RUNTIME_CHUNKS)) {
    if (name === "react") continue;
    assert.ok(
      !hasReactImpl(js),
      `运行时块 ${name} 里包了第二份 React：framer-motion 等库的 hooks 会调到另一个实例（Invalid hook call）`,
    );
  }
  for (const [id, entry] of Object.entries(OPEN_SOURCE_PREVIEWS)) {
    assert.ok(!hasReactImpl(entry.body), `${id}: 组件体里包了第二份 React`);
    assert.ok(
      entry.body.includes("__ZCODE_ASSET_PREVIEW__"),
      `${id}: 组件体没从共享运行时注册表取依赖（应当只带自己的代码）`,
    );
  }
  // motion 依赖 React：它必须走 shim 读注册表，而不是自己打包一份。
  // 断言用属性名而不是整串：minify 会把 globalThis.__ZCODE_ASSET_PREVIEW__ 收成局部
  // 变量（形如 `Nf=globalThis.__ZCODE_ASSET_PREVIEW__` 之后是 `Nf["react/jsx-runtime"]`），
  // 钉字面整串会假红。
  if (PREVIEW_RUNTIME_CHUNKS.motion) {
    assert.ok(
      PREVIEW_RUNTIME_CHUNKS.motion.includes("__ZCODE_ASSET_PREVIEW__"),
      "motion 块必须读共享运行时注册表",
    );
    assert.ok(
      /\["react(\/jsx-runtime|\/dom\/client)?"\]/.test(PREVIEW_RUNTIME_CHUNKS.motion),
      "motion 块必须从注册表取 React（shim），不许自己打包一份",
    );
  }
});

test("收录货拼装产物自包含且结构完整（无外链/无 localStorage）", () => {
  for (const asset of reactOpenSourceAssets) {
    const html = asset.previewHtml;
    assert.ok(html.trim() !== "", `${asset.id}: previewHtml 为空（previewAssemble 没接上）`);
    assert.ok(html.includes('id="root"'), `${asset.id}: 缺挂载点`);
    assert.ok(html.includes("<style>"), `${asset.id}: 缺内联 CSS`);
    assert.ok(!/src\s*=\s*["']https?:/i.test(html), `${asset.id}: 拼装产物含外链资源`);
    assert.ok(!html.includes("localStorage"), `${asset.id}: 拼装产物含 localStorage`);
    // 该件在产物表里声明的每个运行时块都要真出现在产物里
    for (const name of OPEN_SOURCE_PREVIEWS[asset.id]!.chunks) {
      assert.ok(
        html.includes(PREVIEW_RUNTIME_CHUNKS[name]!.slice(0, 64)),
        `${asset.id}: 产物里没拼上运行时块 ${name}`,
      );
    }
  }
});

test("V3-3 收录货台账：三站逐件 source 三字段齐全 + 图纸含原署名", () => {
  // V3-3 逐字收录的 id 带站前缀（beautifului-/rareui-/uiverse-），与 V2-4 那批
  // 效果自实现的同站货区分开：这里只台账逐字收录的新货。
  const newBatch = threeSiteAssets.filter((asset) =>
    /^(beautifului|rareui|uiverse)-/.test(asset.id),
  );
  const bySite = new Map<string, number>();
  for (const asset of newBatch) {
    assert.ok(asset.source, `${asset.id}: 缺 source（许可台账必填）`);
    for (const field of ["site", "url", "license"] as const) {
      assert.ok(asset.source[field].trim() !== "", `${asset.id}: source.${field} 为空`);
    }
    assert.match(asset.source.url, /^https:\/\//, `${asset.id}: source.url 应为 https`);
    bySite.set(asset.source.site, (bySite.get(asset.source.site) ?? 0) + 1);
    assert.ok(asset.files.length > 0, `${asset.id}: 收录货必须有图纸`);
    // 逐字收录的图纸必须保留上游署名（作者/仓库），MIT 与 CC BY 的硬要求
    const blueprint = asset.files.map((file) => file.content).join("\n");
    assert.match(blueprint, /TurboKach|Codewithswappy|Uiverse\.io/, `${asset.id}: 图纸缺上游署名`);
  }
  // 三站件数与任务书口径一致（Beautiful UI 19 / RareUI 26 / UIverse 15）
  assert.equal(bySite.get("Beautiful UI"), 19, "Beautiful UI 应收 19 件");
  assert.equal(bySite.get("RareUI"), 26, "RareUI 应收 26 件");
  assert.equal(bySite.get("UIverse"), 15, "UIverse 应收 15 件");
});

test("V3-3 许可红线：React Bits 不逐字收（MIT+Commons Clause 禁再分发组件本体）", () => {
  // 目录里的 React Bits 货必须是效果自实现（仅标注灵感来源），图纸不许是上游源码。
  const reactBits = ASSET_CATALOG.filter((asset) => asset.source?.site === "React Bits");
  assert.ok(reactBits.length >= 6, `React Bits 货应 ≥6 件，实际 ${reactBits.length}`);
  for (const asset of reactBits) {
    const blueprint = asset.files.map((file) => file.content).join("\n");
    assert.ok(!/DavidHDev|react-bits/.test(blueprint), `${asset.id}: 不许逐字收录 React Bits 源码`);
    assert.match(
      asset.source!.license,
      /Commons Clause|自实现/,
      `${asset.id}: source.license 要写明 Commons Clause 与自实现口径`,
    );
  }
});

test("assertSandboxSafe 负例照旧：混入其它 token 一律 throw", () => {
  assert.equal(ASSET_SANDBOX, "allow-scripts");
  for (const poisoned of [
    "allow-scripts allow-same-origin",
    "allow-scripts allow-popups",
    "allow-scripts allow-top-navigation",
    "allow-same-origin",
    "",
  ]) {
    assert.throws(() => assertSandboxSafe(poisoned), poisoned);
  }
});

test("拼装块序：motion 件的 react 块必须先于 motion 块注册（真机白屏回归钉）", () => {
  // motion 打包产物在模块顶层就解构 react 块注册的导出；react 不先执行它当场崩、
  // 整棵组件树挂不上（真机反馈"预览空白"真因——字母序曾把 motion 排到 react 前）。
  const motionEntries = Object.entries(OPEN_SOURCE_PREVIEWS).filter(([, entry]) =>
    entry.chunks.includes("motion"),
  );
  assert.ok(motionEntries.length > 0, "应存在使用 motion 块的收录货");
  for (const [id, entry] of motionEntries) {
    const ordered = orderPreviewChunks(entry.chunks);
    assert.equal(ordered[0], "react", `${id}: react 块必须排最前，实际 ${ordered.join("+")}`);
  }
  // 拼装产物本身同样按依赖序落 <script>
  const sampleId = motionEntries[0]![0];
  const html = assembleOpenSourcePreview(sampleId);
  const reactPos = html.indexOf(PREVIEW_RUNTIME_CHUNKS.react!.slice(0, 64));
  const motionPos = html.indexOf(PREVIEW_RUNTIME_CHUNKS.motion!.slice(0, 64));
  assert.ok(reactPos >= 0 && motionPos >= 0, "react/motion 块都应内联进拼装产物");
  assert.ok(reactPos < motionPos, `${sampleId}: 产物里 react 块须在 motion 块之前`);
});

test("React 货产物不许夹带 V3 滚动条补丁的 min-height:100% 指纹（挤顶回归钉）", () => {
  // V3 为 240px iframe 把 demo css 的 min-height:100vh 批量改成 100%，V4 固定舞台后
  // 已全部恢复 100vh；生成产物（REACT_PREVIEW_HTML）若不同步刷新就会把旧伤带回来
  // （html 无显式高度时百分比 min-height=0，居中失效→内容挤顶，真机四轮反馈之三）。
  for (const [id, html] of Object.entries(REACT_PREVIEW_HTML)) {
    assert.ok(
      !html.includes("min-height: 100%"),
      `${id}: 产物含 min-height:100% 指纹（应为 100vh）`,
    );
  }
});
