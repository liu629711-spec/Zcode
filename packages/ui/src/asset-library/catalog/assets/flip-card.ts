import type { AssetManifest } from "../types.js";

/**
 * 翻转卡片（V2-4 首批收录，灵感来自 RareUI 的 Flip Card）：
 * 点击沿 Y 轴 3D 翻面，正反两态内容分明，键盘可操作。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：点击/Enter/Space 翻面。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>翻转卡片</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #0d1526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .flip {
    width: min(300px, 84vw); height: 190px; perspective: 900px; cursor: pointer;
    border: 0; padding: 0; background: transparent; text-align: inherit; font: inherit;
  }
  .flip:focus-visible { outline: 2px solid #60a5fa; outline-offset: 4px; border-radius: 20px; }
  .inner {
    position: relative; width: 100%; height: 100%;
    transform-style: preserve-3d; transition: transform 0.65s cubic-bezier(0.3, 0.9, 0.3, 1);
  }
  .flip[aria-pressed="true"] .inner { transform: rotateY(180deg); }
  .face {
    position: absolute; inset: 0; border-radius: 20px; padding: 24px;
    backface-visibility: hidden; -webkit-backface-visibility: hidden;
    display: flex; flex-direction: column; justify-content: center; gap: 8px;
  }
  .front {
    background: linear-gradient(135deg, #16203a, #101828);
    border: 1px solid rgb(125 211 252 / 0.25); color: #e6edf7;
  }
  .front::after {
    content: "点击翻面 →"; position: absolute; right: 18px; bottom: 14px;
    font-size: 11px; letter-spacing: 0.12em; color: #5f7396;
  }
  .front .glyph { font-size: 40px; }
  .back {
    transform: rotateY(180deg);
    background: linear-gradient(135deg, #0ea5e9, #6366f1); color: #06121f;
  }
  .back b { font-size: 16px; }
  .back p { font-size: 12.5px; line-height: 1.8; opacity: 0.85; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .inner { transition: none; } }
</style>
</head>
<body>
  <button class="flip" type="button" id="flip" aria-pressed="false" aria-label="素材卡：点击查看背面说明">
    <span class="inner" aria-hidden="true">
      <span class="face front">
        <span class="glyph">🂠</span>
        <b style="font-size: 17px;">正面 · 素材名</b>
        <span style="font-size: 12.5px; color: #9fb0c9;">一点就翻过去</span>
      </span>
      <span class="face back">
        <b>背面 · 详细说明</b>
        <p>3D 翻转用 perspective + rotateY + backface-visibility 实现，正反两面内容可以完全不同，适合成语卡、词条卡、抽奖卡。</p>
      </span>
    </span>
  </button>
  <p class="hint">点击或用 Enter / Space 翻面</p>
  <script>
    var flip = document.getElementById("flip");
    flip.addEventListener("click", function () {
      var pressed = flip.getAttribute("aria-pressed") === "true";
      flip.setAttribute("aria-pressed", String(!pressed));
    });
  </script>
</body>
</html>`;

export const flipCardAsset: AssetManifest = {
  id: "flip-card",
  title: "翻转卡片",
  titleEn: "Flip Card",
  description: "点击沿 Y 轴 3D 翻面的双面卡，键盘可翻。",
  descriptionEn: "A two-sided card flipping on Y with a soft ease.",
  category: "block",
  tags: ["css", "卡片", "3D翻转", "rareui", "shadcn风"],
  previewHtml: HTML,
  files: [{ name: "flip-card.html", language: "html", content: HTML }],
  prompt:
    "请把「翻转卡片」装进我的项目：一个双面翻转卡——外层 perspective 900px，内层 preserve-3d + rotateY(180deg) 翻转（0.65s 柔和贝塞尔），正反两面 backface-visibility: hidden 各自承载内容（正面渐变描边暗卡、背面亮色实底）；整卡是 button，aria-pressed 记录翻面态，Enter/Space 可翻；尊重 prefers-reduced-motion（瞬切）。适合词条卡/成就卡。先看现有卡片体系，融入而不是覆盖。",
  source: {
    site: "RareUI",
    url: "https://rareui.com",
    license: "MIT",
  },
};
