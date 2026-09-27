import type { AssetManifest } from "../types.js";

/**
 * 真丝绸缎背景（V2-4 首批收录，灵感来自 React Bits 的 Silk）：
 * canvas 多条半透明波绸以 lighter 叠加 + blur 滤镜，缓慢起伏如绸面反光。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：「流速」滑杆（0.2x~2.5x）。
 */

const JS = `var canvas = document.getElementById("silk");
var ctx = canvas.getContext("2d");
var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
var w = 0, h = 0, t = 0;
var speed = 1;
// 每条绸 = 两条正弦叠加的波面，颜色/相位/基线错开，lighter 叠出光泽
var ribbons = [
  { base: 0.30, amp1: 46, f1: 0.006, p1: 0.35, amp2: 22, f2: 0.013, p2: 0.9, color: "rgba(124, 58, 237, 0.20)" },
  { base: 0.42, amp1: 54, f1: 0.005, p1: 0.28, amp2: 26, f2: 0.011, p2: 1.1, color: "rgba(219, 39, 119, 0.16)" },
  { base: 0.54, amp1: 40, f1: 0.007, p1: 0.42, amp2: 18, f2: 0.014, p2: 0.8, color: "rgba(14, 165, 233, 0.18)" },
  { base: 0.66, amp1: 58, f1: 0.004, p1: 0.22, amp2: 24, f2: 0.010, p2: 1.2, color: "rgba(20, 184, 166, 0.14)" },
  { base: 0.78, amp1: 44, f1: 0.006, p1: 0.38, amp2: 20, f2: 0.012, p2: 0.95, color: "rgba(139, 92, 246, 0.17)" }
];
function resize() { w = canvas.width = innerWidth; h = canvas.height = innerHeight; }
addEventListener("resize", resize);
resize();
function paint() {
  ctx.fillStyle = "#07050f";
  ctx.fillRect(0, 0, w, h);
  ctx.filter = "blur(26px)";
  ctx.globalCompositeOperation = "lighter";
  for (var i = 0; i < ribbons.length; i++) {
    var r = ribbons[i];
    ctx.beginPath();
    ctx.moveTo(0, h);
    for (var x = 0; x <= w; x += 6) {
      var y = h * r.base
        + Math.sin(x * r.f1 + t * r.p1) * r.amp1
        + Math.sin(x * r.f2 - t * r.p2) * r.amp2;
      ctx.lineTo(x, y);
    }
    ctx.lineTo(w, h);
    ctx.closePath();
    ctx.fillStyle = r.color;
    ctx.fill();
  }
  ctx.filter = "none";
  ctx.globalCompositeOperation = "source-over";
}
if (reduce) paint();
else requestAnimationFrame(function loop() { t += 0.012 * speed; paint(); requestAnimationFrame(loop); });
var spd = document.getElementById("flow");
spd.addEventListener("input", function () {
  speed = Number(spd.value);
  document.getElementById("flow-out").textContent = speed.toFixed(1) + "x";
});`;

const PANEL_CSS = `
  .panel {
    position: fixed; left: 14px; bottom: 14px; z-index: 10;
    display: flex; align-items: center; gap: 10px;
    padding: 9px 14px; border-radius: 12px;
    background: rgb(7 5 15 / 0.55); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px);
    font: 12px/1 system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #9fb0c9;
  }
  .panel label { display: flex; align-items: center; gap: 8px; user-select: none; }
  .panel input[type="range"] { width: 110px; accent-color: #c4b5fd; }
  .panel output { min-width: 3.2em; text-align: right; font-variant-numeric: tabular-nums; color: #e6edf7; }
`;

const PANEL_HTML = `
  <div class="panel" role="group" aria-label="预览参数">
    <label>流速 <input type="range" id="flow" min="0.2" max="2.5" step="0.1" value="1"></label>
    <output id="flow-out">1.0x</output>
  </div>
`;

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>真丝绸缎背景</title>
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  canvas { display: block; width: 100%; height: 100%; }
  ${PANEL_CSS}
</style>
</head>
<body>
  <canvas id="silk"></canvas>
  ${PANEL_HTML}
  <script>${JS}</script>
</body>
</html>`;

export const silkBackgroundAsset: AssetManifest = {
  id: "silk-background",
  title: "真丝绸缎背景",
  titleEn: "Silk Background",
  description: "多条彩绸轻叠流动的暗色氛围底，像绸面反光。",
  descriptionEn: "Translucent silk ribbons shimmering over a dark canvas.",
  category: "background",
  tags: ["canvas", "背景", "氛围", "react-bits", "暗色"],
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
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  canvas { display: block; width: 100%; height: 100%; }
  ${PANEL_CSS}
</style>
</head>
<body>
  <canvas id="silk"></canvas>
  ${PANEL_HTML}
  <script src="silk-background.js"></script>
</body>
</html>`,
    },
    { name: "silk-background.js", language: "javascript", content: JS },
  ],
  prompt:
    "请把「真丝绸缎背景」装进我的项目：一块全屏 canvas 氛围背景——深紫黑底上 4~6 条半透明彩色波绸（每条是两条正弦叠加的波面，紫/品红/蓝/青绿错开基线与相位），用 globalCompositeOperation=lighter 叠加加 blur 滤镜做出绸面反光质感，缓慢起伏流动；requestAnimationFrame 驱动、resize 自适应；尊重 prefers-reduced-motion（降级为静止一帧）。内容层叠在上面保证可读。先看现有页面结构，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
