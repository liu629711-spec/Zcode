import type { AssetManifest } from "../types.js";

/**
 * 故障风文字（自制原创）：主体文字不动，两个伪元素副本分别染青/品红，
 * clip-path 切片错位 + steps() 跳动，大部分时间静止、偶发抖动。
 * V2-3 交互：主体文字 contenteditable，input 时同步 data-text 让副本跟随。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>故障风文字</title>
<style>
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: #0a0c12;
    font-family: ui-monospace, "Cascadia Code", Consolas, "PingFang SC", monospace;
  }
  .glitch {
    position: relative; margin: 0;
    font-size: clamp(28px, 6vw, 50px); font-weight: 800; letter-spacing: 0.06em; color: #e6edf5;
  }
  .glitch::before, .glitch::after {
    content: attr(data-text); position: absolute; inset: 0; opacity: 0.85;
  }
  .glitch::before { color: #22d3ee; animation: glitch-a 3.2s infinite steps(1); }
  .glitch::after { color: #f472b6; animation: glitch-b 2.7s infinite steps(1); }
  @keyframes glitch-a {
    0%, 88%, 100% { transform: none; clip-path: inset(0 0 0 0); opacity: 0; }
    89% { transform: translate(-3px, 2px); clip-path: inset(12% 0 58% 0); opacity: 0.9; }
    93% { transform: translate(3px, -1px); clip-path: inset(62% 0 6% 0); opacity: 0.9; }
    96% { transform: translate(-2px, 1px); clip-path: inset(36% 0 38% 0); opacity: 0.9; }
  }
  @keyframes glitch-b {
    0%, 84%, 100% { transform: none; clip-path: inset(0 0 0 0); opacity: 0; }
    85% { transform: translate(3px, -2px); clip-path: inset(48% 0 22% 0); opacity: 0.9; }
    90% { transform: translate(-3px, 2px); clip-path: inset(8% 0 70% 0); opacity: 0.9; }
    94% { transform: translate(2px, -1px); clip-path: inset(70% 0 4% 0); opacity: 0.9; }
  }
  @media (prefers-reduced-motion: reduce) {
    .glitch::before, .glitch::after { animation: none; opacity: 0; }
  }
  .glitch[contenteditable] { outline: none; cursor: text; caret-color: #22d3ee; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
</style>
</head>
<body>
  <h1 class="glitch" contenteditable="true" spellcheck="false" data-text="SYSTEM ONLINE">SYSTEM ONLINE</h1>
  <p class="hint">点文字可直接编辑，故障副本实时跟随</p>
  <script>
    var g = document.querySelector(".glitch");
    g.addEventListener("input", function () {
      g.dataset.text = g.textContent; // 伪元素副本用 attr(data-text) 取字，编辑时同步
    });
    g.addEventListener("keydown", function (e) {
      if (e.key === "Enter") { e.preventDefault(); g.blur(); } // 单行：回车收起
    });
  </script>
</body>
</html>`;

export const glitchTextAsset: AssetManifest = {
  id: "glitch-text",
  title: "故障风文字",
  titleEn: "Glitch Text",
  description: "偶尔抖一下的赛博故障标题，青红副本切片错位。",
  descriptionEn: "A cyber glitch headline that jolts with cyan/magenta slices.",
  category: "text-animation",
  tags: ["css", "文字动效", "故障风", "标题"],
  previewHtml: HTML,
  files: [{ name: "glitch-text.html", language: "html", content: HTML }],
  prompt:
    "请把「故障风文字」装进我的项目：一个赛博故障（glitch）风格的标题组件——主体文字不动，::before/::after 两个伪元素副本分别染青色和品红，用 clip-path: inset 切片错位加 steps() 跳动；大部分时间静止、每隔两三秒突发抖一次，克制才有故障味；副本内容用 data-text 属性驱动；尊重 prefers-reduced-motion（降级为普通文字）。先看现有标题排版，融入而不是覆盖。",
};
