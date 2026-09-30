import type { AssetManifest } from "../types.js";

/**
 * 涌动扭曲文字（V5 扩批，灵感来自 React Bits 的 Warp Text）：
 * 逐字按正弦相位做上下位移，波浪涌过整行。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>涌动扭曲文字</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 20px; }
  h1 { margin: 0; font-size: clamp(28px, 6vw, 52px); font-weight: 900; letter-spacing: 0.06em; }
  h1 .ch { display: inline-block; color: #e6edf5; will-change: transform; }
  h1 .ch:nth-child(3n) { color: #a5b4fc; }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  @media (prefers-reduced-motion: reduce) { h1 .ch { transform: none !important; } }
</style>
</head>
<body>
  <div class="stage">
    <h1 id="warp">波浪涌过这一行字</h1>
    <p class="hint">正弦相位逐字位移，波峰自左向右涌动</p>
  </div>
  <script>
    var el = document.getElementById("warp");
    var text = el.textContent;
    el.textContent = "";
    var glyphs = Array.from(text).map(function (ch) {
      var span = document.createElement("span");
      span.className = "ch";
      span.textContent = ch;
      el.appendChild(span);
      return span;
    });

    var frame = 0;
    function tick() {
      frame += 1;
      glyphs.forEach(function (span, index) {
        var phase = frame * 0.045 - index * 0.45; // 相位差制造"涌动"
        span.style.transform = "translateY(" + (Math.sin(phase) * 10).toFixed(2) + "px)";
      });
      requestAnimationFrame(tick);
    }
    tick();
  </script>
</body>
</html>`;

export const warpTextAsset: AssetManifest = {
  id: "reactbits-warp-text",
  title: "涌动扭曲文字",
  titleEn: "Warp Text",
  description: "逐字按正弦相位上下位移，波峰自左向右涌过整行标题。",
  descriptionEn: "Per-glyph sine offsets send a wave across the line.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "波浪", "正弦", "循环"],
  previewHtml: HTML,
  files: [{ name: "reactbits-warp-text.html", language: "html", content: HTML }],
  prompt:
    "请把「涌动扭曲文字」装进我的项目：标题按字符拆成 inline-block span，requestAnimationFrame 里给每个字按 (帧率×速率 − 序号×相位差) 的正弦值做 translateY（振幅约 10px），相邻字相位差约 0.45rad，波峰自左向右涌动；tabular 数字宽度防抖；尊重 prefers-reduced-motion（静止）。先看现有标题场景，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/warp-text",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
