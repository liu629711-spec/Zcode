import type { AssetManifest } from "../types.js";

/**
 * 分段选择器（自制原创）：真实 radio 承载状态，滑块按 :checked 序号 translateX
 * 平滑换位，纯 CSS 无 JS。图纸分 html+css 两件可拿走。
 */

const CSS = `* { box-sizing: border-box; margin: 0; }
body {
  min-height: 100%; display: grid; place-items: center;
  background: radial-gradient(110% 110% at 50% 0%, #101827 0%, #0a0f18 65%, #070b12 100%);
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
}
.seg {
  position: relative; display: inline-flex; padding: 4px; border-radius: 12px;
  background: #141b28; box-shadow: inset 0 1px 5px rgb(0 0 0 / 0.5);
}
.seg input { position: absolute; opacity: 0; pointer-events: none; }
.seg label {
  position: relative; z-index: 1; padding: 8px 24px; border-radius: 9px; cursor: pointer;
  color: #8ea0bd; font-size: 14px; user-select: none; transition: color 0.25s ease;
}
.seg-ind {
  position: absolute; top: 4px; left: 4px;
  width: calc((100% - 8px) / 3); height: calc(100% - 8px); border-radius: 9px;
  background: linear-gradient(135deg, #38bdf8, #818cf8);
  box-shadow: 0 2px 8px rgb(56 189 248 / 0.4);
  transition: transform 0.3s cubic-bezier(0.3, 1.4, 0.4, 1);
}
.seg input:nth-of-type(1):checked ~ .seg-ind { transform: translateX(0); }
.seg input:nth-of-type(2):checked ~ .seg-ind { transform: translateX(100%); }
.seg input:nth-of-type(3):checked ~ .seg-ind { transform: translateX(200%); }
.seg input:nth-of-type(1):checked ~ label:nth-of-type(1),
.seg input:nth-of-type(2):checked ~ label:nth-of-type(2),
.seg input:nth-of-type(3):checked ~ label:nth-of-type(3) { color: #0b1120; font-weight: 600; }
.seg input:focus-visible ~ .seg-ind { outline: 2px solid #60a5fa; outline-offset: 2px; }
@media (prefers-reduced-motion: reduce) { .seg-ind, .seg label { transition: none; } }`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>分段选择器</title>
<style>${CSS}</style>
</head>
<body>
  <div class="seg" role="radiogroup" aria-label="时间范围">
    <input type="radio" name="seg" id="seg-d" checked>
    <label for="seg-d">日</label>
    <input type="radio" name="seg" id="seg-w">
    <label for="seg-w">周</label>
    <input type="radio" name="seg" id="seg-m">
    <label for="seg-m">月</label>
    <span class="seg-ind" aria-hidden="true"></span>
  </div>
</body>
</html>`;

export const segmentedControlAsset: AssetManifest = {
  id: "segmented-control",
  title: "分段选择器",
  titleEn: "Segmented Control",
  description: "纯 CSS 分段单选，渐变滑块平滑换位。",
  descriptionEn: "A pure-CSS radio segmented control with a sliding pill.",
  category: "control",
  tags: ["css", "选择器", "分段", "表单"],
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
<link rel="stylesheet" href="segmented-control.css">
</head>
<body>
  <div class="seg" role="radiogroup" aria-label="时间范围">
    <input type="radio" name="seg" id="seg-d" checked>
    <label for="seg-d">日</label>
    <input type="radio" name="seg" id="seg-w">
    <label for="seg-w">周</label>
    <input type="radio" name="seg" id="seg-m">
    <label for="seg-m">月</label>
    <span class="seg-ind" aria-hidden="true"></span>
  </div>
</body>
</html>`,
    },
    { name: "segmented-control.css", language: "css", content: CSS },
  ],
  prompt:
    "请把「分段选择器」装进我的项目：一组纯 CSS 的分段单选（segmented control）——真实 radio input 承载状态，选中的渐变小胶囊滑块用 translateX 平滑滑到对应段，label 文字随选中变色；支持键盘方向键切换与 focus-visible 描边；尊重 prefers-reduced-motion（滑块瞬移）。先看现有的筛选/切换控件风格，融入而不是覆盖。",
};
