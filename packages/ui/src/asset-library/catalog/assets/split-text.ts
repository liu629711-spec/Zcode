import type { AssetManifest } from "../types.js";

/**
 * 分行入场文字（V2-4 首批收录，灵感来自 React Bits 的 Split Text）：
 * 文字拆成单字 span，逐字带序号延迟上浮旋转进场。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：主体文字 contenteditable，失焦后按新文案重新入场。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>分行入场文字</title>
<style>
  body {
    min-height: 100%; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .split {
    margin: 0; font-size: clamp(28px, 6vw, 50px); font-weight: 800;
    letter-spacing: 0.04em; color: #e6edf5; text-align: center; line-height: 1.3;
  }
  .split .ch {
    display: inline-block;
    animation: rise 0.7s cubic-bezier(0.2, 0.9, 0.3, 1.1) both;
    animation-delay: calc(var(--i) * 45ms);
  }
  @keyframes rise { from { opacity: 0; transform: translateY(0.7em) rotate(6deg); } to { opacity: 1; transform: none; } }
  .split[contenteditable] { outline: none; cursor: text; caret-color: #a5b4fc; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .split .ch { animation: none; } }
</style>
</head>
<body>
  <h1 class="split" id="split" contenteditable="true" spellcheck="false">逐字升起排列</h1>
  <p class="hint">点文字可直接编辑，失焦后重新入场</p>
  <script>
    var el = document.getElementById("split");
    function split(text) {
      el.textContent = "";
      var i = 0;
      Array.from(text).forEach(function (ch) {
        var s = document.createElement("span");
        s.className = "ch";
        s.style.setProperty("--i", String(i++));
        s.textContent = ch === " " ? "\\u00a0" : ch;
        el.appendChild(s);
      });
    }
    split(el.textContent);
    el.addEventListener("blur", function () { split(el.textContent.replace(/\\n/g, "")); });
    el.addEventListener("keydown", function (e) {
      if (e.key === "Enter") { e.preventDefault(); el.blur(); } // 单行：回车收起
    });
  </script>
</body>
</html>`;

export const splitTextAsset: AssetManifest = {
  id: "split-text",
  title: "分行入场文字",
  titleEn: "Split Text Reveal",
  description: "标题拆成单字逐个升起进场，序号延迟做出波浪节奏。",
  descriptionEn: "A headline split into glyphs rising in with staggered rhythm.",
  category: "text-animation",
  tags: ["css", "文字动效", "标题", "react-bits", "暗色"],
  previewHtml: HTML,
  files: [{ name: "split-text.html", language: "html", content: HTML }],
  prompt:
    "请把「分行入场文字」装进我的项目：一个标题入场组件——把文案按字符拆成 inline-block 的 span，每个字带序号递增的 animation-delay（约 45ms/字），以轻微上浮加 6 度旋转的轨迹进场，cubic-bezier 带一点回弹；中文按码位拆分、空格转不间断空格；文案变化后要能重放动画（React 里可用 key 重挂载或重置动画）；尊重 prefers-reduced-motion（直接完整显示）。先看现有标题排版，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
