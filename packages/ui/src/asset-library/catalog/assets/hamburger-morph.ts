import type { AssetManifest } from "../types.js";

/**
 * 汉堡变叉按钮（自制原创）：三条横线在选中时平滑变形为 X，纯 CSS，
 * 真实 checkbox 承载状态。图纸分 html+css 两件可拿走。
 */

const CSS = `* { box-sizing: border-box; margin: 0; }
body {
  min-height: 100vh; display: grid; place-items: center;
  background: radial-gradient(110% 110% at 50% 0%, #101827 0%, #0a0f18 65%, #070b12 100%);
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
}
.burger {
  position: relative; display: block; width: 54px; height: 54px; cursor: pointer;
  border-radius: 14px; background: #141b28;
  box-shadow: inset 0 1px 5px rgb(0 0 0 / 0.5), 0 4px 12px rgb(0 0 0 / 0.35);
}
.burger input { position: absolute; inset: 0; margin: 0; opacity: 0; cursor: pointer; }
.burger input:focus-visible { outline: 2px solid #60a5fa; outline-offset: 2px; border-radius: 14px; }
.bar {
  position: absolute; left: 15px; right: 15px; height: 2px; border-radius: 2px;
  background: #bcd0ee;
  transition: transform 0.3s ease, top 0.3s ease, opacity 0.25s ease;
}
.b1 { top: 18px; } .b2 { top: 26px; } .b3 { top: 34px; }
.burger input:checked ~ .b1 { top: 26px; transform: rotate(45deg); }
.burger input:checked ~ .b2 { opacity: 0; }
.burger input:checked ~ .b3 { top: 26px; transform: rotate(-45deg); }
@media (prefers-reduced-motion: reduce) { .bar { transition: none; } }`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>汉堡变叉按钮</title>
<style>${CSS}</style>
</head>
<body>
  <label class="burger">
    <input type="checkbox" aria-label="打开菜单">
    <span class="bar b1"></span><span class="bar b2"></span><span class="bar b3"></span>
  </label>
</body>
</html>`;

export const hamburgerMorphAsset: AssetManifest = {
  id: "hamburger-morph",
  title: "汉堡变叉按钮",
  titleEn: "Hamburger Toggle",
  description: "三条横线平滑变 X 的菜单开合钮，纯 CSS。",
  descriptionEn: "A pure-CSS hamburger that morphs into a cross when open.",
  category: "control",
  tags: ["css", "图标按钮", "菜单", "导航"],
  previewHtml: PREVIEW_HTML,
  files: [
    {
      name: "index.html",
      language: "html",
      content: `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="stylesheet" href="hamburger-morph.css">
</head>
<body>
  <label class="burger">
    <input type="checkbox" aria-label="打开菜单">
    <span class="bar b1"></span><span class="bar b2"></span><span class="bar b3"></span>
  </label>
</body>
</html>`,
    },
    { name: "hamburger-morph.css", language: "css", content: CSS },
  ],
  prompt:
    "请把「汉堡变叉按钮」装进我的项目：一个纯 CSS 的菜单开合按钮——三条横线在选中时平滑变形为 X（上下两条旋转、中间淡出），真实 checkbox 承载状态，focus-visible 有描边；作为移动端导航/抽屉的开关使用；尊重 prefers-reduced-motion（变形瞬切）。先看现有的导航结构把开合状态接上，融入而不是覆盖。",
};
