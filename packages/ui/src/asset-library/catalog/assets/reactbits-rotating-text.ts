import type { AssetManifest } from "../types.js";

/**
 * 轮换翻转词组（V5 扩批，灵感来自 React Bits 的 Rotating Text）：
 * 固定前缀 + 高亮词槽，词组按节奏翻转轮换。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>轮换翻转词组</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 20px; }
  h1 {
    margin: 0; font-size: clamp(24px, 5vw, 44px); font-weight: 800;
    letter-spacing: 0.04em; color: #e6edf5; display: flex; align-items: baseline; gap: 0.45em;
  }
  .slot {
    display: inline-grid; overflow: hidden; border-radius: 10px;
    background: linear-gradient(180deg, #6366f1, #8b5cf6);
    padding: 0.08em 0.4em; min-width: 3.2em; text-align: center;
    box-shadow: 0 8px 24px rgba(99, 102, 241, 0.35);
  }
  /* 两行词轮换：下面的词从底部升入，上面的词升出顶部 */
  .slot .reel { display: grid; grid-template-rows: 1fr 1fr; transition: transform 0.55s cubic-bezier(0.3, 0.9, 0.25, 1); }
  .slot .reel span { line-height: 1.4em; white-space: nowrap; }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  @media (prefers-reduced-motion: reduce) { .slot .reel { transition: none; } }
</style>
</head>
<body>
  <div class="stage">
    <h1>把灵感 <span class="slot" id="slot"><span class="reel" id="reel"></span></span> 变成界面</h1>
    <p class="hint">高亮词槽按节奏翻转轮换</p>
  </div>
  <script>
    var WORDS = ["写进代码", "拖进画布", "发给智能体", "讲给同事"];
    var reel = document.getElementById("reel");
    var slot = document.getElementById("slot");
    var index = 0;

    function show(next) {
      // 双缓冲：先在第二行塞新词，滚上去后再无缝复位
      reel.innerHTML = "";
      var old = document.createElement("span");
      old.textContent = WORDS[index];
      var upcoming = document.createElement("span");
      upcoming.textContent = WORDS[next];
      reel.append(old, upcoming);
      slot.style.minWidth = "0";
      slot.style.minWidth = Math.max(...WORDS.map(function (w) { return w.length; })) + "ch";
      reel.style.transition = "none";
      reel.style.transform = "translateY(0)";
      requestAnimationFrame(function () {
        reel.style.transition = "";
        reel.style.transform = "translateY(-1.4em)"; // 一个行高
      });
      index = next;
    }

    show(1);
    setInterval(function () { show((index + 1) % WORDS.length); }, 2200);
  </script>
</body>
</html>`;

export const rotatingTextAsset: AssetManifest = {
  id: "reactbits-rotating-text",
  title: "轮换翻转词组",
  titleEn: "Rotating Text",
  description: "固定前缀 + 高亮词槽，词组按节奏向上翻滚轮换，适合 hero 副标题。",
  descriptionEn: "A highlighted slot flipping through word rotations.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "轮换", "词组", "hero"],
  previewHtml: HTML,
  files: [{ name: "reactbits-rotating-text.html", language: "html", content: HTML }],
  prompt:
    "请把「轮换翻转词组」装进我的项目：hero 标题里一个高亮词槽（渐变底、圆角），词组列表按 2.2s 节奏向上翻滚轮换——双缓冲实现无缝循环（新词在下方就位，滚一个行高后复位）；词槽宽度按最长词固定避免抖动；尊重 prefers-reduced-motion（直接切换不滚动）。先看现有 hero/首页文案，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/rotating-text",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
