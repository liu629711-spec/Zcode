import type { AssetManifest } from "../types.js";

/**
 * 流光丝带背景（V2-4 首批收录，灵感来自 React Bits 的 Ribbons）：
 * canvas 多条 hue 循环的发光曲线横向流动，线宽细、辉光柔。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：「流速」滑杆（0.2x~2.5x）。
 */

const JS = `var canvas = document.getElementById("ribbons");
var ctx = canvas.getContext("2d");
var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
var w = 0, h = 0, t = 0;
var speed = 1;
function resize() { w = canvas.width = innerWidth; h = canvas.height = innerHeight; }
addEventListener("resize", resize);
resize();
function paint() {
  ctx.fillStyle = "rgb(7 9 18 / 0.5)"; // 半透明清屏留一点点拖影
  ctx.fillRect(0, 0, w, h);
  ctx.lineCap = "round";
  for (var i = 0; i < 5; i++) {
    var hue = (i * 52 + t * 24) % 360;
    ctx.strokeStyle = "hsla(" + hue + ", 85%, 65%, 0.55)";
    ctx.lineWidth = 1.6;
    ctx.shadowColor = "hsla(" + hue + ", 85%, 65%, 0.9)";
    ctx.shadowBlur = 12;
    ctx.beginPath();
    for (var x = 0; x <= w; x += 5) {
      var y = h * (0.24 + i * 0.13)
        + Math.sin(x * 0.004 + t * (0.7 + i * 0.12) + i * 1.7) * (34 + i * 8)
        + Math.sin(x * 0.009 - t * 0.5 + i) * 14;
      if (x === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.stroke();
  }
  ctx.shadowBlur = 0;
}
if (reduce) paint();
else requestAnimationFrame(function loop() { t += 0.014 * speed; paint(); requestAnimationFrame(loop); });
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
    background: rgb(7 9 18 / 0.6); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px);
    font: 12px/1 system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #9fb0c9;
  }
  .panel label { display: flex; align-items: center; gap: 8px; user-select: none; }
  .panel input[type="range"] { width: 110px; accent-color: #a5b4fc; }
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
<title>流光丝带背景</title>
<style>
  html, body { margin: 0; height: 100%; overflow: hidden; }
  body { background: #070912; }
  canvas { display: block; width: 100%; height: 100%; }
  ${PANEL_CSS}
</style>
</head>
<body>
  <canvas id="ribbons"></canvas>
  ${PANEL_HTML}
  <script>${JS}</script>
</body>
</html>`;

export const ribbonsBackgroundAsset: AssetManifest = {
  id: "ribbons-background",
  title: "流光丝带背景",
  titleEn: "Ribbons Background",
  description: "色相循环的发光曲线横向流动，赛博氛围衬底。",
  descriptionEn: "Hue-cycling glowing ribbons streaming across a dark canvas.",
  category: "background",
  tags: ["canvas", "背景", "流光", "react-bits", "暗色"],
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
  body { background: #070912; }
  canvas { display: block; width: 100%; height: 100%; }
  ${PANEL_CSS}
</style>
</head>
<body>
  <canvas id="ribbons"></canvas>
  ${PANEL_HTML}
  <script src="ribbons-background.js"></script>
</body>
</html>`,
    },
    { name: "ribbons-background.js", language: "javascript", content: JS },
  ],
  prompt:
    "请把「流光丝带背景」装进我的项目：一块全屏 canvas 氛围背景——5 条发光细曲线横向流动，每条的颜色色相随时间循环漂移（hsla + shadowBlur 辉光），正弦叠加决定起伏轨迹，半透明清屏留轻微拖影；requestAnimationFrame 驱动、resize 自适应；尊重 prefers-reduced-motion（降级为静止一帧）。内容层叠在上面保证可读。先看现有页面结构，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
