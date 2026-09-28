import type { AssetManifest } from "../types.js";

/**
 * 「扫条加载器」逐字收录（V3-3 素材库）。
 *
 * 来源：UIverse · github.com/uiverse-io/galaxy · loaders/bociKond_foolish-sloth-24.html
 * 作者：bociKond（UIverse.io 社区投稿）
 * 许可：CC BY 4.0（uiverse.io 站点条款；仓库 LICENSE 另标 MIT）——按 CC BY 要求
 *       逐件标注原作者与 UIverse.io 出处（见 source 字段与文件头的声明块）。
 * 文件为上游片段的逐字收录（原署名注释保留在 <style> 顶部）。
 */
export const UIverseBarSweepLoaderAsset: AssetManifest = {
  id: "uiverse-bar-sweep-loader",
  title: "扫条加载器",
  titleEn: "Bar Sweep Loader",
  description: "描边胶囊里色块从左扫到右的进度感加载器。",
  category: "control",
  tags: ["uiverse","加载","进度","极简","css"],
  previewHtml: "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>扫条加载器</title>\n<style>\n  * { box-sizing: border-box; margin: 0; }\n  html, body { height: 100%; }\n  body {\n    display: grid; place-items: center;\n    background: radial-gradient(120% 130% at 50% 0%, #10151f 0%, #0b0e15 60%, #080a10 100%);\n    font-family: system-ui, \"PingFang SC\", \"Microsoft YaHei\", sans-serif;\n    color: #cdd8ea; overflow: hidden;\n  }\n  .uv-stage { display: grid; place-items: center; gap: 14px; padding: 16px; }\n  .uv-credit { font-size: 11px; letter-spacing: 0.06em; color: #5b6b8c; user-select: none; }\n</style>\n</head>\n<body>\n  <div class=\"uv-stage\">\n<div class=\"loader\"></div>\r\n<style>\r\n/* From Uiverse.io by bociKond - Tags: minimalist, loading, loader, animated, color, minimal */\r\n.loader {\r\n  /* color of choise */\r\n  --clr: #05A8AA;\r\n  /* loading time of choice */\r\n  --load-time: 2s;\r\n  outline: 5px solid var(--clr);\r\n  outline-offset: 5px;\r\n  position: relative;\r\n  overflow: hidden;\r\n  border-radius: 5rem;\r\n  /* width: 10rem; */\r\n  /* height: 2rem; */\r\n  padding: 1rem 5rem;\r\n  /* use either padding or width + height*/\r\n  /* I prefer the padding one */\r\n  /* rotate: -90deg; */\r\n  /* rotate if you want/need vertical loader */\r\n}\r\n\r\n.loader::after {\r\n  content: '';\r\n  position: absolute;\r\n  top: 0;\r\n  left: 0;\r\n  width: 100%;\r\n  height: 100%;\r\n  background-color: var(--clr);\r\n  z-index: 2;\r\n  animation: loading var(--load-time) ease-in-out infinite;\r\n}\r\n\r\n@keyframes loading {\r\n  0% {\r\n    width: 0%;\r\n  }\r\n\r\n  100% {\r\n    width: 100%;\r\n  }\r\n}\r\n</style>\n    <p class=\"uv-credit\">UIverse · bociKond · CC BY 4.0</p>\n  </div>\n</body>\n</html>",
  files: [{ name: "bar-sweep-loader.html", language: "html", content: "<!--\n  扫条加载器 · bar-sweep-loader.html\n\n  来源：github.com/uiverse-io/galaxy · loaders/bociKond_foolish-sloth-24.html\n  原址：https://github.com/uiverse-io/galaxy/blob/main/loaders/bociKond_foolish-sloth-24.html\n  作者：bociKond（UIverse.io 社区）\n  版权：Copyright (c) bociKond\n  许可：CC BY 4.0（uiverse.io 站点条款；仓库 LICENSE 另标 MIT）\n\n  署名要求：CC BY 4.0 要求署名原作者与 UIverse.io。\n\n  以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n-->\n<div class=\"loader\"></div>\r\n<style>\r\n/* From Uiverse.io by bociKond - Tags: minimalist, loading, loader, animated, color, minimal */\r\n.loader {\r\n  /* color of choise */\r\n  --clr: #05A8AA;\r\n  /* loading time of choice */\r\n  --load-time: 2s;\r\n  outline: 5px solid var(--clr);\r\n  outline-offset: 5px;\r\n  position: relative;\r\n  overflow: hidden;\r\n  border-radius: 5rem;\r\n  /* width: 10rem; */\r\n  /* height: 2rem; */\r\n  padding: 1rem 5rem;\r\n  /* use either padding or width + height*/\r\n  /* I prefer the padding one */\r\n  /* rotate: -90deg; */\r\n  /* rotate if you want/need vertical loader */\r\n}\r\n\r\n.loader::after {\r\n  content: '';\r\n  position: absolute;\r\n  top: 0;\r\n  left: 0;\r\n  width: 100%;\r\n  height: 100%;\r\n  background-color: var(--clr);\r\n  z-index: 2;\r\n  animation: loading var(--load-time) ease-in-out infinite;\r\n}\r\n\r\n@keyframes loading {\r\n  0% {\r\n    width: 0%;\r\n  }\r\n\r\n  100% {\r\n    width: 100%;\r\n  }\r\n}\r\n</style>\r\n" }],
  prompt: "请把「扫条加载器」装进我的项目：一个极简加载条——胶囊形容器只有 5px 描边（outline + outline-offset，无背景），内部一条同色实心块以 2s ease-in-out 无限循环从左铺满到右再重来（width 0%→100%）；颜色与时长用 CSS 变量（--clr / --load-time）暴露，方便主题接线；提供竖排选项（rotate: -90deg）。加 role=\"progressbar\"（不定态，aria-valuetext=\"加载中\"），prefers-reduced-motion 时改为透明度脉冲。先看现有加载/进度组件，融入而不是覆盖。",
  source: { site: "UIverse", url: "https://github.com/uiverse-io/galaxy/blob/main/loaders/bociKond_foolish-sloth-24.html", license: "CC BY 4.0（作者 bociKond · UIverse.io）" },
};
