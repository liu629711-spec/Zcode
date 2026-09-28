/**
 * V3-3 收录货的预览装配器（技术设计 §10 V2-4/V3-3）。
 *
 * 背景：逐字收录的 React+Tailwind 货（RareUI / Beautiful UI）如果按老办法
 * （每件把 react 全量内联进 previewHtml）做成静态字符串，50 件货光 react 运行时
 * 就 10MB+，preview-html.ts 会涨到几十 MB。所以构建期把运行时拆成**共享块**
 * （react / motion / three / liveline / iconoir / lucide / glimm，
 * 见 PREVIEW_RUNTIME_CHUNKS），每件货只存自己的 CSS + 组件体，运行时在这里按需拼装。
 *
 * 成本模型：同一件货在一次会话里只拼一次（模块级 Map 缓存），拼接是字符串相加，
 * 毫秒级；50 件货全部展开的内存约等于各自产物之和（仍是字符串级，不是 DOM）。
 *
 * 自包含性由构建脚本与 assetPreview.test.ts 两侧钉住：无外链、无 localStorage。
 */

import { OPEN_SOURCE_PREVIEWS, PREVIEW_RUNTIME_CHUNKS } from "./preview-html.js";

const assembled = new Map<string, string>();

/**
 * 块加载顺序（依赖序，不是字母序）：motion 打包产物在模块顶层就解构 react 块
 * 注册的导出（Fragment 等），react 不先就位它当场崩掉、整棵组件树挂不上——
 * 真机反馈的"预览空白"真因即构建脚本按字母序 .sort() 把 motion 排到了 react 前。
 * 未知块名排在最后（容错将来新增的独立库）。
 */
const CHUNK_DEPENDENCY_ORDER = ["react", "motion", "three", "liveline", "glimm"];

export function orderPreviewChunks(chunks: string[]): string[] {
  return [...chunks].sort(
    (a, b) =>
      CHUNK_DEPENDENCY_ORDER.indexOf(a) - CHUNK_DEPENDENCY_ORDER.indexOf(b) ||
      a.localeCompare(b),
  );
}

/** 逃逸 `</script`：拼接产物里出现它会截断标签（老产线 wrapHtml 同款处理）。 */
function escapeScript(js: string): string {
  return js.replaceAll("</script", "<\\/script");
}

/**
 * 拼出自包含预览 HTML：运行时块（只带该件用到的）+ tailwind 产物 + 组件体。
 * 未见过的 id 返回空串——catalogCheck 会以"必填字段 previewHtml 为空"报错炸掉，
 * 不让它变成静默白屏（与老产线的缺失语义一致）。
 */
export function assembleOpenSourcePreview(id: string): string {
  const cached = assembled.get(id);
  if (cached !== undefined) return cached;
  const entry = OPEN_SOURCE_PREVIEWS[id];
  if (!entry) return "";
  const chunks = orderPreviewChunks(entry.chunks)
    .map((name) => {
      const chunk = PREVIEW_RUNTIME_CHUNKS[name];
      if (!chunk) return "";
      // 运行时块只含 react/motion/three 等第三方代码，不含 </script；仍统一逃逸
      return `<script>${escapeScript(chunk)}</script>`;
    })
    .join("\n");
  const html = [
    "<!doctype html>",
    `<html lang="zh-CN"${entry.dark ? ' class="dark"' : ""}>`,
    "<head>",
    '<meta charset="utf-8">',
    '<meta name="viewport" content="width=device-width, initial-scale=1">',
    `<title>${entry.title}</title>`,
    "<style>",
    entry.css,
    "</style>",
    '<style id="__zcode-asset-stage">',
    // 演示舞台：预览区是卡片里的缩放舞台，组件按 body 撑满居中；
    // 上游各组件自带的 body 样式已在上面的 tailwind base 里，这里只补舞台与滚动收敛。
    "html, body { height: 100%; }",
    // 舞台底色：RareUI/Beautiful UI 均为浅色主题设计（dark:false），组件大量使用
    // 浅色系配色——透明底会让深色 UI 透上来，深色组件"隐形"（真机反馈：预览空白）。
    // 非暗色 entry 一律白底装裱；将来收暗色设计货时给 entry 加 dark=true 走黑底。
    entry.dark ? "body { background: #0d1117; }" : "body { background: #ffffff; }",
    "body { margin: 0; overflow: hidden; display: flex; align-items: center; justify-content: center; }",
    entry.stage === "top" ? "body { align-items: flex-start; padding: 16px; overflow: auto; }" : "",
    "</style>",
    "</head>",
    "<body>",
    '<div id="root"></div>',
    chunks,
    `<script>${escapeScript(entry.body)}</script>`,
    "</body>",
    "</html>",
  ].join("\n");
  assembled.set(id, html);
  return html;
}
