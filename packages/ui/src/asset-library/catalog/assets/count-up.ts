import type { AssetManifest } from "../types.js";

/**
 * 数字攀升（V2-4 首批收录，灵感来自 React Bits 的 Count Up）：
 * rAF + easeOutCubic 把数值从当前值滚到目标值，千分位格式化。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：目标值滑杆（0~99999），拖动即从当前值滚向新目标。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>数字攀升</title>
<style>
  body {
    min-height: 100%; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #0d1526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: flex; flex-direction: column; align-items: center; gap: 26px; }
  .num {
    margin: 0; font-size: clamp(40px, 8vw, 72px); font-weight: 800;
    font-variant-numeric: tabular-nums; color: #7dd3fc;
    text-shadow: 0 0 26px rgb(125 211 252 / 0.35);
  }
  .panel {
    display: flex; align-items: center; gap: 10px;
    padding: 9px 14px; border-radius: 12px;
    background: rgb(255 255 255 / 0.06); border: 1px solid rgb(255 255 255 / 0.14);
    font: 12px/1 system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #9fb0c9;
  }
  .panel label { display: flex; align-items: center; gap: 8px; user-select: none; }
  .panel input[type="range"] { width: 150px; accent-color: #7dd3fc; }
  .panel output { min-width: 4.5em; text-align: right; font-variant-numeric: tabular-nums; color: #e6edf7; }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
</style>
</head>
<body>
  <div class="stage">
    <p class="num" id="num" role="img" aria-label="当前数值 0">0</p>
    <div class="panel" role="group" aria-label="预览参数">
      <label>目标值 <input type="range" id="goal" min="0" max="99999" step="500" value="20260"></label>
      <output id="goal-out">20,260</output>
    </div>
    <p class="hint">拖动滑杆改目标，数字从当前值滚上去</p>
  </div>
  <script>
    var num = document.getElementById("num");
    var goal = document.getElementById("goal");
    var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    var gen = 0; // 代次令牌：拖动中途换目标，旧动画自废
    function fmt(n) { return n.toLocaleString("zh-CN"); }
    function countTo(target) {
      var my = ++gen;
      var from = Number(num.dataset.v || "0");
      var dur = 1400;
      if (reduce) { num.dataset.v = String(target); num.textContent = fmt(target); return; }
      var t0 = null;
      requestAnimationFrame(function step(ts) {
        if (my !== gen) return;
        if (t0 === null) t0 = ts;
        var k = Math.min(1, (ts - t0) / dur);
        var eased = 1 - Math.pow(1 - k, 3); // easeOutCubic
        var v = Math.round(from + (target - from) * eased);
        num.dataset.v = String(v);
        num.textContent = fmt(v);
        num.setAttribute("aria-label", "当前数值 " + fmt(v));
        if (k < 1) requestAnimationFrame(step);
      });
    }
    countTo(20260);
    goal.addEventListener("input", function () {
      countTo(Number(goal.value));
      document.getElementById("goal-out").textContent = fmt(Number(goal.value));
    });
  </script>
</body>
</html>`;

export const countUpAsset: AssetManifest = {
  id: "count-up",
  title: "数字攀升",
  titleEn: "Count Up",
  description: "数值从当前值 easeOut 滚向目标，千分位格式化不跳字。",
  descriptionEn: "A value easing up to its target with thousands separators.",
  category: "text-animation",
  tags: ["js", "数字动效", "计数器", "react-bits", "仪表盘"],
  previewHtml: HTML,
  files: [{ name: "count-up.html", language: "html", content: HTML }],
  prompt:
    "请把「数字攀升」装进我的项目：一个计数动画组件——数值变化时用 requestAnimationFrame 从当前值滚向目标值，easeOutCubic 缓动约 1.4s，等宽数字（tabular-nums）避免跳动，千分位按 locale 格式化；目标值中途变化要能从当前进度续滚（不是从头再来）；值语义给 aria-label 同步；尊重 prefers-reduced-motion（直接显示终值）。适合 KPI 数字、统计页。先看现有的数据展示风格，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
