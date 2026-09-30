import type { AssetManifest } from "../types.js";

/**
 * 折纸入场文字（V5 扩批，灵感来自 React Bits 的 Fold Text）：
 * 每个词像纸片一样从折叠状态翻正展开。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>折纸入场文字</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
    perspective: 800px;
  }
  .stage { display: grid; place-items: center; gap: 18px; }
  h1 { margin: 0; font-size: clamp(26px, 5.5vw, 48px); font-weight: 800;
    letter-spacing: 0.05em; color: #e6edf5; text-align: center; line-height: 1.5; }
  h1 .word {
    display: inline-block; transform-origin: 50% 100%;
    animation: unfold 0.9s cubic-bezier(0.2, 0.8, 0.25, 1) both;
    animation-delay: calc(var(--i) * 160ms);
  }
  @keyframes unfold {
    0% { opacity: 0; transform: rotateX(-92deg) translateY(0.35em); }
    60% { opacity: 1; }
    100% { opacity: 1; transform: none; }
  }
  button {
    border: 1px solid #2c3750; background: rgba(148, 163, 184, 0.08); color: #c7d2ea;
    font: inherit; font-size: 13px; padding: 7px 16px; border-radius: 999px; cursor: pointer;
  }
  button:hover { background: rgba(148, 163, 184, 0.16); }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  @media (prefers-reduced-motion: reduce) { h1 .word { animation: none; } }
</style>
</head>
<body>
  <div class="stage">
    <h1 id="headline" contenteditable="true" spellcheck="false">像折纸一样展开的标题</h1>
    <button id="replay">重放</button>
    <p class="hint">逐词从折叠状态翻正展开，可直接编辑文案</p>
  </div>
  <script>
    var headline = document.getElementById("headline");

    function fold(text) {
      headline.textContent = "";
      var words = text.match(/\\S+/g) ?? [];
      words.forEach(function (word, index) {
        var span = document.createElement("span");
        span.className = "word";
        span.style.setProperty("--i", index);
        span.textContent = word;
        headline.appendChild(span);
        if (index < words.length - 1) headline.appendChild(document.createTextNode(" "));
      });
    }

    document.getElementById("replay").addEventListener("click", function () {
      fold(headline.textContent);
    });
    headline.addEventListener("blur", function () { fold(headline.textContent); });
    headline.addEventListener("keydown", function (event) {
      if (event.key === "Enter") { event.preventDefault(); headline.blur(); }
    });
    fold(headline.textContent);
  </script>
</body>
</html>`;

export const foldTextAsset: AssetManifest = {
  id: "reactbits-fold-text",
  title: "折纸入场文字",
  titleEn: "Fold Text",
  description: "标题逐词像纸片一样从折叠状态翻正展开，带透视与递进延迟。",
  descriptionEn: "Words unfolding like paper flaps with perspective and stagger.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "3D", "折叠", "入场"],
  previewHtml: HTML,
  files: [{ name: "reactbits-fold-text.html", language: "html", content: HTML }],
  prompt:
    "请把「折纸入场文字」装进我的项目：标题按词拆成 inline-block span，父级给 perspective:800px，每词从 rotateX(-92deg)（折叠倒伏）翻正到 0 度，transform-origin 在底边，逐词 160ms 延迟展开，cubic-bezier 带一点停顿感；文案变化/进入视口时重放；尊重 prefers-reduced-motion（直接显示）。先看现有标题排版，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/fold-text",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
