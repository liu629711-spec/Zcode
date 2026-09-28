import type { AssetManifest } from "../types.js";

/**
 * 打字机文字（自制原创）：JS 逐字打字 + 光标闪烁，CJK 长度可靠（不走 CSS ch 步进）。
 * 图纸单文件自包含，内容与 preview 同一份。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>打字机文字</title>
<style>
  body {
    min-height: 100%; display: grid; place-items: center;
    background: linear-gradient(160deg, #0b1020 0%, #101a30 55%, #0a0f1c 100%);
    font-family: ui-monospace, "Cascadia Code", Consolas, "PingFang SC", monospace;
  }
  .stage { padding: 32px; }
  .type {
    font-size: clamp(20px, 4.5vw, 32px); font-weight: 600; color: #e6edf5;
    letter-spacing: 0.04em; white-space: nowrap;
  }
  .type::after {
    content: ""; display: inline-block; width: 2px; height: 1.15em; margin-left: 3px;
    background: #7dd3fc; vertical-align: text-bottom;
    animation: caret 0.9s step-end infinite;
  }
  @keyframes caret { 50% { opacity: 0; } }
  @media (prefers-reduced-motion: reduce) {
    .type::after { animation: none; }
  }
  .type[contenteditable] { outline: none; cursor: text; caret-color: #7dd3fc; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
</style>
</head>
<body>
  <p class="stage"><span class="type" id="type" contenteditable="true" spellcheck="false" aria-label="你好，这里是交互素材库。"></span></p>
  <p class="hint">点文字可直接编辑，失焦后按新文案重新打字</p>
  <script>
    const el = document.getElementById("type");
    let gen = 0; // 代次令牌：重播时让旧打字链自废
    function type(text) {
      const my = ++gen;
      el.setAttribute("aria-label", text);
      if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
        el.textContent = text; // 降级：直接呈现全文，只留静态光标
        return;
      }
      let i = 0;
      (function tick() {
        if (my !== gen) return;
        el.textContent = text.slice(0, ++i);
        if (i < text.length) setTimeout(tick, 140);
      })();
    }
    type("你好，这里是交互素材库。");
    el.addEventListener("blur", () => type(el.textContent.replace(/\\n/g, "")));
    el.addEventListener("keydown", (e) => {
      if (e.key === "Enter") { e.preventDefault(); el.blur(); } // 单行：回车收起
    });
  </script>
</body>
</html>`;

export const typewriterAsset: AssetManifest = {
  id: "typewriter-text",
  title: "打字机文字",
  description: "逐字打出的标题行，带闪烁光标；中文逐字步进不打架。",
  category: "text-animation",
  tags: ["文字动效", "js", "标题", "暗色"],
  previewHtml: HTML,
  files: [{ name: "typewriter.html", language: "html", content: HTML }],
  prompt:
    "请把「打字机文字」装进我的项目：一个逐字打出的标题组件，文本从空到全文逐字出现，末尾带闪烁竖线光标，打完后光标保留；中文按字符步进（不要用 CSS ch 宽度做步进，中文宽度不可靠）；速度约 120~160ms/字，尊重 prefers-reduced-motion（降级为直接显示全文）。先看现有标题排版风格，融入而不是覆盖。",
};
