import type { AssetManifest } from "../types.js";

/**
 * 雷达扫描（自制原创）：同心圆环 + 十字轴 + conic-gradient 旋转扇叶，
 * 两个光点按节拍闪灭模拟回波。纯 CSS，图纸单文件自包含。
 * V2-3 交互：「扫描速度」滑杆改 --spin 变量，扇叶与回波光点共用同一周期。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>雷达扫描</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 120% at 50% 0%, #0a1410 0%, #070d0b 55%, #050807 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .radar {
    position: relative; width: min(320px, 82vw); aspect-ratio: 1 / 1; border-radius: 50%;
    border: 1px solid rgb(52 211 153 / 0.4);
    background: radial-gradient(circle, rgb(52 211 153 / 0.05) 0%, transparent 72%);
    box-shadow: 0 0 40px rgb(52 211 153 / 0.12), inset 0 0 30px rgb(52 211 153 / 0.08);
    overflow: hidden;
  }
  .ring { position: absolute; inset: 0; border-radius: 50%; border: 1px solid rgb(52 211 153 / 0.18); }
  .r2 { inset: 18%; } .r3 { inset: 36%; }
  .axis-h { position: absolute; top: 0; bottom: 0; left: 50%; width: 1px; background: rgb(52 211 153 / 0.15); }
  .axis-v { position: absolute; left: 0; right: 0; top: 50%; height: 1px; background: rgb(52 211 153 / 0.15); }
  .sweep {
    position: absolute; inset: 0; border-radius: 50%;
    background: conic-gradient(from 0deg, rgb(52 211 153 / 0.45), rgb(52 211 153 / 0.08) 60deg, transparent 90deg);
    animation: sweep var(--spin, 4s) linear infinite;
  }
  .blip {
    position: absolute; width: 7px; height: 7px; border-radius: 50%; background: #4ade80;
    box-shadow: 0 0 10px rgb(74 222 128 / 0.9); animation: blip var(--spin, 4s) ease-out infinite;
  }
  .one { left: 32%; top: 38%; }
  .two { left: 64%; top: 60%; animation-delay: 2s; }
  @keyframes sweep { to { transform: rotate(360deg); } }
  @keyframes blip { 0%, 12% { opacity: 0; } 20% { opacity: 1; } 70% { opacity: 0.9; } 100% { opacity: 0; } }
  @media (prefers-reduced-motion: reduce) {
    .sweep { animation: none; }
    .blip { animation: none; opacity: 0.9; }
  }
  .panel {
    position: fixed; left: 50%; bottom: 16px; transform: translateX(-50%); z-index: 10;
    display: flex; align-items: center; gap: 10px;
    padding: 9px 14px; border-radius: 12px;
    background: rgb(8 14 12 / 0.6); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(8px); -webkit-backdrop-filter: blur(8px);
    font: 12px/1 system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #9fb8ac;
  }
  .panel label { display: flex; align-items: center; gap: 8px; user-select: none; }
  .panel input[type="range"] { width: 110px; accent-color: #4ade80; }
  .panel output { min-width: 3.2em; text-align: right; font-variant-numeric: tabular-nums; color: #e6f5ec; }
</style>
</head>
<body>
  <div class="radar" role="img" aria-label="雷达扫描装饰图">
    <div class="ring r1"></div><div class="ring r2"></div><div class="ring r3"></div>
    <div class="axis-h"></div><div class="axis-v"></div>
    <div class="sweep"></div>
    <div class="blip one"></div><div class="blip two"></div>
  </div>
  <div class="panel" role="group" aria-label="预览参数">
    <label>扫描速度 <input type="range" id="spin" min="0.3" max="3" step="0.1" value="1"></label>
    <output id="spin-out">1.0x</output>
  </div>
  <script>
    var slider = document.getElementById("spin");
    var radar = document.querySelector(".radar");
    slider.addEventListener("input", function () {
      radar.style.setProperty("--spin", (4 / Number(slider.value)) + "s");
      document.getElementById("spin-out").textContent = Number(slider.value).toFixed(1) + "x";
    });
  </script>
</body>
</html>`;

export const radarScanAsset: AssetManifest = {
  id: "radar-scan",
  title: "雷达扫描",
  titleEn: "Radar Sweep",
  description: "同心圆环加旋转扫描扇叶的雷达盘，监控风氛围件。",
  descriptionEn: "A radar dial with rings, rotating sweep and blinking blips.",
  category: "background",
  tags: ["css", "背景", "雷达", "氛围"],
  previewHtml: HTML,
  files: [{ name: "radar-scan.html", language: "html", content: HTML }],
  prompt:
    "请把「雷达扫描」装进我的项目：一块雷达仪风格的装饰面板——同心圆环加十字轴线的暗绿雷达盘，conic-gradient 扫描扇叶匀速旋转，盘面上两个光点按节拍闪灭模拟目标回波；纯 CSS 实现；尊重 prefers-reduced-motion（扫描静止、光点常亮）。适合做监控/状态页的氛围元素。先看现有页面结构，融入而不是覆盖。",
};
