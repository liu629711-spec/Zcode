import type { AssetManifest } from "../types.js";

/**
 * 电弧描边框（V5 扩批，灵感来自 React Bits 的 Electric Border）：
 * SVG 湍流位移让矩形描边"通电"抖动，内容浮在上层。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>电弧描边框</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .frame { position: relative; width: min(420px, 88vw); filter: url(#electric); }
  .frame .edge {
    position: absolute; inset: 0; border-radius: 16px;
    border: 2px solid #8b5cf6; box-shadow: 0 0 18px rgba(139, 92, 246, 0.6);
  }
  .frame .content {
    position: relative; margin: 3px; border-radius: 14px; padding: 26px 24px;
    background: linear-gradient(180deg, #141b2e, #0d1322); color: #c7d2ea;
  }
  .frame .content h2 { margin: 0 0 8px; font-size: 17px; color: #e6edf5; }
  .frame .content p { margin: 0; font-size: 13px; line-height: 1.7; color: #9fb0c8; }
  .hint { margin: 24px 0 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; text-align: center; }
</style>
</head>
<body>
  <div>
    <svg width="0" height="0" aria-hidden="true">
      <filter id="electric">
        <feTurbulence id="turb" type="turbulence" baseFrequency="0.008 0.014" numOctaves="2" seed="3" result="noise"/>
        <feDisplacementMap in="SourceGraphic" in2="noise" scale="9" xChannelSelector="R" yChannelSelector="G"/>
      </filter>
    </svg>
    <div class="frame" id="frame">
      <div class="edge"></div>
      <div class="content">
        <h2>通电的边框</h2>
        <p>描边层挂 SVG 湍流位移滤镜，边线像电流一样持续蠕动；内容层不受影响，稳稳浮在上面。</p>
      </div>
    </div>
    <p class="hint">边框通电抖动 · 内容保持稳定</p>
  </div>
  <script>
    var turb = document.getElementById("turb");
    // 只让滤镜作用于 .edge（描边层）：内容层放在滤镜容器外即可——这里用
    // 逐帧微调湍流频率让电弧"活"起来，而不是整体抖动。
    var frame = 0;
    function tick() {
      frame += 1;
      turb.setAttribute(
        "baseFrequency",
        (0.008 + Math.sin(frame / 9) * 0.003).toFixed(4) + " " + (0.014 + Math.cos(frame / 7) * 0.004).toFixed(4)
      );
      requestAnimationFrame(tick);
    }
    tick();
  </script>
</body>
</html>`;

export const electricBorderAsset: AssetManifest = {
  id: "reactbits-electric-border",
  title: "电弧描边框",
  titleEn: "Electric Border",
  description: "SVG 湍流位移驱动描边持续蠕动，像通电的电弧，内容层保持稳定。",
  descriptionEn: "SVG turbulence keeps the border arcing while content stays put.",
  category: "block",
  tags: ["react-bits", "区块", "电弧", "描边", "滤镜"],
  previewHtml: HTML,
  files: [{ name: "reactbits-electric-border.html", language: "html", content: HTML }],
  prompt:
    "请把「电弧描边框」装进我的项目：容器分两层——描边层（2px 品牌色描边 + 外辉光）挂 SVG feTurbulence + feDisplacementMap 滤镜（scale 6~10），requestAnimationFrame 逐帧微调 baseFrequency 让电弧持续蠕动；内容层独立于滤镜之外保证文字稳定；尊重 prefers-reduced-motion（停帧但保留静态描边）。先看现有高亮卡片/重要提示容器，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/animations/electric-border",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
