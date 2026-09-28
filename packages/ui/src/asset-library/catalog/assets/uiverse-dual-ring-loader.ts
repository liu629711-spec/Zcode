import type { AssetManifest } from "../types.js";

/**
 * 「双环互扣加载器」逐字收录（V3-3 素材库）。
 *
 * 来源：UIverse · github.com/uiverse-io/galaxy · loaders/AHMED-MIT_dry-wolverine-85.html
 * 作者：AHMED-MIT（UIverse.io 社区投稿）
 * 许可：CC BY 4.0（uiverse.io 站点条款；仓库 LICENSE 另标 MIT）——按 CC BY 要求
 *       逐件标注原作者与 UIverse.io 出处（见 source 字段与文件头的声明块）。
 * 文件为上游片段的逐字收录（原署名注释保留在 <style> 顶部）。
 */
export const UIverseDualRingLoaderAsset: AssetManifest = {
  id: "uiverse-dual-ring-loader",
  title: "双环互扣加载器",
  titleEn: "Dual Ring Loader",
  description: "两个缺口圆环反向旋转互扣的加载器。",
  category: "control",
  tags: ["uiverse","加载","圆环","极简","css"],
  previewHtml: "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>双环互扣加载器</title>\n<style>\n  * { box-sizing: border-box; margin: 0; }\n  html, body { height: 100%; }\n  body {\n    display: grid; place-items: center;\n    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);\n    font-family: system-ui, \"PingFang SC\", \"Microsoft YaHei\", sans-serif;\n    color: #cdd8ea; overflow: hidden;\n  }\n  .uv-stage { display: grid; place-items: center; gap: 14px; padding: 16px; }\n  .uv-credit { font-size: 11px; letter-spacing: 0.06em; color: #5b6b8c; user-select: none; }\n</style>\n</head>\n<body>\n  <div class=\"uv-stage\">\n<div class=\"spinner\">\r\n  <div></div>\r\n  <div></div>\r\n  <div></div>\r\n  <div></div>\r\n  <div></div>\r\n</div>\r\n<style>\r\n/* From Uiverse.io by AHMED-MIT - Tags: loader, accept, account */\r\n.spinner {\r\n  position: relative;\r\n  width: 33.6px;\r\n  height: 33.6px;\r\n  perspective: 67.2px;\r\n}\r\n\r\n.spinner div {\r\n  width: 100%;\r\n  height: 100%;\r\n  background: #474bff;\r\n  position: absolute;\r\n  left: 50%;\r\n  transform-origin: left;\r\n  animation: spinner-16s03x 2s infinite;\r\n}\r\n\r\n.spinner div:nth-child(1) {\r\n  animation-delay: 0.15s;\r\n}\r\n\r\n.spinner div:nth-child(2) {\r\n  animation-delay: 0.3s;\r\n}\r\n\r\n.spinner div:nth-child(3) {\r\n  animation-delay: 0.45s;\r\n}\r\n\r\n.spinner div:nth-child(4) {\r\n  animation-delay: 0.6s;\r\n}\r\n\r\n.spinner div:nth-child(5) {\r\n  animation-delay: 0.75s;\r\n}\r\n\r\n@keyframes spinner-16s03x {\r\n  0% {\r\n    transform: rotateY(0deg);\r\n  }\r\n\r\n  50%, 80% {\r\n    transform: rotateY(-180deg);\r\n  }\r\n\r\n  90%, 100% {\r\n    opacity: 0;\r\n    transform: rotateY(-180deg);\r\n  }\r\n}\r\n</style>\n    <p class=\"uv-credit\">UIverse · AHMED-MIT · CC BY 4.0</p>\n  </div>\n</body>\n</html>",
  files: [{ name: "dual-ring-loader.html", language: "html", content: "<!--\n  双环互扣加载器 · dual-ring-loader.html\n\n  来源：github.com/uiverse-io/galaxy · loaders/AHMED-MIT_dry-wolverine-85.html\n  原址：https://github.com/uiverse-io/galaxy/blob/main/loaders/AHMED-MIT_dry-wolverine-85.html\n  作者：AHMED-MIT（UIverse.io 社区）\n  版权：Copyright (c) AHMED-MIT\n  许可：CC BY 4.0（uiverse.io 站点条款；仓库 LICENSE 另标 MIT）\n\n  署名要求：CC BY 4.0 要求署名原作者与 UIverse.io。\n\n  以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n-->\n<div class=\"spinner\">\r\n  <div></div>\r\n  <div></div>\r\n  <div></div>\r\n  <div></div>\r\n  <div></div>\r\n</div>\r\n<style>\r\n/* From Uiverse.io by AHMED-MIT - Tags: loader, accept, account */\r\n.spinner {\r\n  position: relative;\r\n  width: 33.6px;\r\n  height: 33.6px;\r\n  perspective: 67.2px;\r\n}\r\n\r\n.spinner div {\r\n  width: 100%;\r\n  height: 100%;\r\n  background: #474bff;\r\n  position: absolute;\r\n  left: 50%;\r\n  transform-origin: left;\r\n  animation: spinner-16s03x 2s infinite;\r\n}\r\n\r\n.spinner div:nth-child(1) {\r\n  animation-delay: 0.15s;\r\n}\r\n\r\n.spinner div:nth-child(2) {\r\n  animation-delay: 0.3s;\r\n}\r\n\r\n.spinner div:nth-child(3) {\r\n  animation-delay: 0.45s;\r\n}\r\n\r\n.spinner div:nth-child(4) {\r\n  animation-delay: 0.6s;\r\n}\r\n\r\n.spinner div:nth-child(5) {\r\n  animation-delay: 0.75s;\r\n}\r\n\r\n@keyframes spinner-16s03x {\r\n  0% {\r\n    transform: rotateY(0deg);\r\n  }\r\n\r\n  50%, 80% {\r\n    transform: rotateY(-180deg);\r\n  }\r\n\r\n  90%, 100% {\r\n    opacity: 0;\r\n    transform: rotateY(-180deg);\r\n  }\r\n}\r\n</style>\r\n" }],
  prompt: "请把「双环互扣加载器」装进我的项目：一个加载指示器——两个只有描边（无缺口用透明弧段作视觉断点）的圆环叠放，各自以 1s 线性匀速反向旋转（一个顺时针一个逆时针），交错出互扣的动感；环径与线宽用 CSS 变量暴露，颜色取 currentColor 便于放进任何容器。加 role=\"status\" + 视觉隐藏文案，prefers-reduced-motion 时改为静态双环。先看现有加载组件，融入而不是覆盖。",
  source: { site: "UIverse", url: "https://github.com/uiverse-io/galaxy/blob/main/loaders/AHMED-MIT_dry-wolverine-85.html", license: "CC BY 4.0（作者 AHMED-MIT · UIverse.io）" },
};
