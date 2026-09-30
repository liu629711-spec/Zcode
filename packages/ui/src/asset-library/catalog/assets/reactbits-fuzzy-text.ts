import type { AssetManifest } from "../types.js";

/**
 * 毛糊抖动文字（V5 扩批，灵感来自 React Bits 的 Fuzzy Text）：
 * 悬停时文字边缘炸开毛刺抖动，SVG 湍流滤镜实现。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>毛糊抖动文字</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 18px; }
  h1 {
    margin: 0; font-size: clamp(30px, 7vw, 60px); font-weight: 900; letter-spacing: 0.08em;
    color: #a5b4fc; cursor: default; user-select: none;
    filter: url(#fuzzy);
  }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
</style>
</head>
<body>
  <div class="stage">
    <svg width="0" height="0" aria-hidden="true">
      <filter id="fuzzy">
        <feTurbulence id="turb" type="fractalNoise" baseFrequency="0.02 0.09" numOctaves="2" result="noise" seed="7"/>
        <feDisplacementMap in="SourceGraphic" in2="noise" scale="0" xChannelSelector="R" yChannelSelector="G"/>
      </filter>
    </svg>
    <h1 id="fuzzyText">毛糊抖动</h1>
    <p class="hint">移入文字：边缘炸开毛刺；移出：恢复锐利</p>
  </div>
  <script>
    var turb = document.getElementById("turb");
    var displacement = document.querySelector("feDisplacementMap");
    var text = document.getElementById("fuzzyText");
    var chaos = 0; // 当前毛刺强度
    var target = 0;
    var frame = 0;

    function tick() {
      frame += 1;
      chaos += (target - chaos) * 0.12; // 平滑逼近目标强度
      displacement.setAttribute("scale", chaos.toFixed(2));
      // 抖动时湍流频率随帧漂移，毛刺才会"活"
      turb.setAttribute(
        "baseFrequency",
        (0.02 + Math.sin(frame / 6) * 0.012).toFixed(4) + " " + (0.09 + Math.cos(frame / 5) * 0.03).toFixed(4)
      );
      requestAnimationFrame(tick);
    }
    tick();

    text.addEventListener("mouseenter", function () { target = 16; });
    text.addEventListener("mouseleave", function () { target = 0; });
  </script>
</body>
</html>`;

export const fuzzyTextAsset: AssetManifest = {
  id: "reactbits-fuzzy-text",
  title: "毛糊抖动文字",
  titleEn: "Fuzzy Text",
  description: "悬停时文字边缘炸开毛刺抖动，SVG 湍流位移滤镜驱动，移出恢复锐利。",
  descriptionEn: "SVG turbulence frays the glyphs on hover, snapping back on leave.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "滤镜", "抖动", "悬停"],
  previewHtml: HTML,
  files: [{ name: "reactbits-fuzzy-text.html", language: "html", content: HTML }],
  prompt:
    "请把「毛糊抖动文字」装进我的项目：文字挂 SVG feTurbulence + feDisplacementMap 滤镜，悬停时把 displacement scale 平滑拉到 12~18、baseFrequency 随帧微漂移让毛刺活动，移出时平滑归零恢复锐利；用 requestAnimationFrame 做强度插值，避免突变；尊重 prefers-reduced-motion（不抖）。先看现有标题/LOGO 场景，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/fuzzy-text",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
