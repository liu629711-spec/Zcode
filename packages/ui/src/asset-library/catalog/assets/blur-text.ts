import type { AssetManifest } from "../types.js";

/**
 * 模糊渐显文字（V2-4 首批收录，灵感来自 React Bits 的 Blur Text）：
 * 字符从 blur(12px)+透明+下沉 渐清晰落位，序号延迟接力。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：主体文字 contenteditable，失焦后按新文案重新渐显。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>模糊渐显文字</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 10%, #131024 0%, #0b0a18 60%, #070610 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .blur-in {
    margin: 0; font-size: clamp(26px, 5.5vw, 46px); font-weight: 700;
    letter-spacing: 0.03em; color: #f1eaff; text-align: center; line-height: 1.4;
  }
  .blur-in .ch {
    display: inline-block;
    animation: unblur 0.9s ease both;
    animation-delay: calc(var(--i) * 70ms);
  }
  @keyframes unblur {
    from { opacity: 0; filter: blur(12px); transform: translateY(0.35em); }
    to { opacity: 1; filter: blur(0); transform: none; }
  }
  .blur-in[contenteditable] { outline: none; cursor: text; caret-color: #c4b5fd; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #4c4a6b; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .blur-in .ch { animation: none; } }
</style>
</head>
<body>
  <h1 class="blur-in" id="text" contenteditable="true" spellcheck="false">从模糊里清晰起来</h1>
  <p class="hint">点文字可直接编辑，失焦后重新渐显</p>
  <script>
    var el = document.getElementById("text");
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
      if (e.key === "Enter") { e.preventDefault(); el.blur(); }
    });
  </script>
</body>
</html>`;

export const blurTextAsset: AssetManifest = {
  id: "blur-text",
  title: "模糊渐显文字",
  titleEn: "Blur Text Reveal",
  description: "字符从高斯模糊与透明中渐清晰落位，柔而不闪。",
  descriptionEn: "Glyphs sharpen out of a blur, soft and unhurried.",
  category: "text-animation",
  tags: ["css", "文字动效", "模糊", "react-bits", "暗色"],
  previewHtml: HTML,
  files: [{ name: "blur-text.html", language: "html", content: HTML }],
  prompt:
    "请把「模糊渐显文字」装进我的项目：一个标题/口号入场组件——文案按字符拆成 inline-block span，每字从 opacity 0 + filter: blur(12px) + 下沉 0.35em 过渡到完全清晰，动画约 0.9s、每字延迟约 70ms 接力；只动 opacity/transform/filter，性能友好；文案更新后可重放；尊重 prefers-reduced-motion（直接完整显示）。先看现有标题排版，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
