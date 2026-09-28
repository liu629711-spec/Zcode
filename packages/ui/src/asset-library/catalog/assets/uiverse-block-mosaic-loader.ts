import type { AssetManifest } from "../types.js";

/**
 * 「方块呼吸加载器」逐字收录（V3-3 素材库）。
 *
 * 来源：UIverse · github.com/uiverse-io/galaxy · loaders/A-nshuman_fluffy-fox-90.html
 * 作者：A-nshuman（UIverse.io 社区投稿）
 * 许可：CC BY 4.0（uiverse.io 站点条款；仓库 LICENSE 另标 MIT）——按 CC BY 要求
 *       逐件标注原作者与 UIverse.io 出处（见 source 字段与文件头的声明块）。
 * 文件为上游片段的逐字收录（原署名注释保留在 <style> 顶部）。
 */
export const UIverseBlockMosaicLoaderAsset: AssetManifest = {
  id: "uiverse-block-mosaic-loader",
  title: "方块呼吸加载器",
  titleEn: "Block Mosaic Loader",
  description: "四个方块轮流伸缩的网格加载器。",
  category: "control",
  tags: ["uiverse","加载","网格","呼吸","css"],
  previewHtml: "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>方块呼吸加载器</title>\n<style>\n  * { box-sizing: border-box; margin: 0; }\n  html, body { height: 100%; }\n  body {\n    display: grid; place-items: center;\n    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);\n    font-family: system-ui, \"PingFang SC\", \"Microsoft YaHei\", sans-serif;\n    color: #cdd8ea; overflow: hidden;\n  }\n  .uv-stage { display: grid; place-items: center; gap: 14px; padding: 16px; }\n  .uv-credit { font-size: 11px; letter-spacing: 0.06em; color: #5b6b8c; user-select: none; }\n</style>\n</head>\n<body>\n  <div class=\"uv-stage\">\n<div class=\"blocks\">\r\n  <div class=\"block\"></div>\r\n  <div class=\"block\"></div>\r\n  <div class=\"block\"></div>\r\n  <div class=\"block\"></div>\r\n</div>\r\n\r\n<style>\r\n/* From Uiverse.io by A-nshuman  - Tags: animation, loading, loader, creative, loading animation */\r\n.blocks {\r\n  border: 2px solid #2b83e2;\r\n  max-width: 158px;\r\n  padding: 4px;\r\n  border-radius: 8px;\r\n  gap: 4px;\r\n  display: flex;\r\n  flex-wrap: wrap;\r\n}\r\n.blocks .block {\r\n  display: flex;\r\n  flex: 1;\r\n  border-radius: 4px;\r\n  background: #2b83e2;\r\n  width: 75px;\r\n  height: 75px;\r\n  animation: blockLoading 1s infinite;\r\n}\r\n.blocks .block:nth-child(1) {\r\n  animation-delay: 0ms;\r\n}\r\n.blocks .block:nth-child(2) {\r\n  animation-delay: 200ms;\r\n}\r\n.blocks .block:nth-child(3) {\r\n  animation-delay: 400ms;\r\n}\r\n.blocks .block:nth-child(4) {\r\n  animation-delay: 600ms;\r\n}\r\n@keyframes blockLoading {\r\n  0%,\r\n  100% {\r\n    flex: 1;\r\n  }\r\n  50% {\r\n    flex: 4;\r\n  }\r\n}\r\n\r\n</style>\n    <p class=\"uv-credit\">UIverse · A-nshuman · CC BY 4.0</p>\n  </div>\n</body>\n</html>",
  files: [{ name: "block-mosaic-loader.html", language: "html", content: "<!--\n  方块呼吸加载器 · block-mosaic-loader.html\n\n  来源：github.com/uiverse-io/galaxy · loaders/A-nshuman_fluffy-fox-90.html\n  原址：https://github.com/uiverse-io/galaxy/blob/main/loaders/A-nshuman_fluffy-fox-90.html\n  作者：A-nshuman（UIverse.io 社区）\n  版权：Copyright (c) A-nshuman\n  许可：CC BY 4.0（uiverse.io 站点条款；仓库 LICENSE 另标 MIT）\n\n  署名要求：CC BY 4.0 要求署名原作者与 UIverse.io。\n\n  以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n-->\n<div class=\"blocks\">\r\n  <div class=\"block\"></div>\r\n  <div class=\"block\"></div>\r\n  <div class=\"block\"></div>\r\n  <div class=\"block\"></div>\r\n</div>\r\n\r\n<style>\r\n/* From Uiverse.io by A-nshuman  - Tags: animation, loading, loader, creative, loading animation */\r\n.blocks {\r\n  border: 2px solid #2b83e2;\r\n  max-width: 158px;\r\n  padding: 4px;\r\n  border-radius: 8px;\r\n  gap: 4px;\r\n  display: flex;\r\n  flex-wrap: wrap;\r\n}\r\n.blocks .block {\r\n  display: flex;\r\n  flex: 1;\r\n  border-radius: 4px;\r\n  background: #2b83e2;\r\n  width: 75px;\r\n  height: 75px;\r\n  animation: blockLoading 1s infinite;\r\n}\r\n.blocks .block:nth-child(1) {\r\n  animation-delay: 0ms;\r\n}\r\n.blocks .block:nth-child(2) {\r\n  animation-delay: 200ms;\r\n}\r\n.blocks .block:nth-child(3) {\r\n  animation-delay: 400ms;\r\n}\r\n.blocks .block:nth-child(4) {\r\n  animation-delay: 600ms;\r\n}\r\n@keyframes blockLoading {\r\n  0%,\r\n  100% {\r\n    flex: 1;\r\n  }\r\n  50% {\r\n    flex: 4;\r\n  }\r\n}\r\n\r\n</style>\r\n    " }],
  prompt: "请把「方块呼吸加载器」装进我的项目：一个加载指示器——蓝色细描边圆角方框内是 2×2 四个同色方块，四个方块各错开 200ms 依次在 1s 周期里横向/纵向伸张（flex 1→4→1），整体像呼吸一样此起彼伏；外框尺寸 158px 上限、内边距与间隙 4px。容器加 role=\"status\" + 视觉隐藏的「加载中」文案，尊重 prefers-reduced-motion（改为静态或轻微透明度呼吸）。先看现有加载组件，融入而不是覆盖。",
  source: { site: "UIverse", url: "https://github.com/uiverse-io/galaxy/blob/main/loaders/A-nshuman_fluffy-fox-90.html", license: "CC BY 4.0（作者 A-nshuman · UIverse.io）" },
};
