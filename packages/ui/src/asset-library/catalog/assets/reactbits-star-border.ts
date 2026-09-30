import type { AssetManifest } from "../types.js";

/**
 * 星轨流边按钮（V5 扩批，灵感来自 React Bits 的 Star Border）：
 * conic 渐变光带沿边框匀速环绕，适合主 CTA。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>星轨流边按钮</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 22px; }
  .halo {
    position: relative; padding: 2px; border-radius: 999px; overflow: hidden;
    /* conic 渐变里只留两段亮弧，旋转即"星轨绕行" */
    background: conic-gradient(
      from var(--angle, 0deg),
      transparent 0deg, rgba(165, 180, 252, 0.9) 24deg,
      transparent 60deg, transparent 180deg,
      rgba(244, 114, 182, 0.7) 210deg, transparent 250deg, transparent 360deg
    );
    animation: orbit 3.6s linear infinite;
  }
  @property --angle { syntax: "<angle>"; initial-value: 0deg; inherits: false; }
  @keyframes orbit { to { --angle: 360deg; } }
  .halo button {
    border: 0; border-radius: 999px; cursor: pointer;
    padding: 13px 34px; font: inherit; font-size: 15px; font-weight: 700; color: #f4f7ff;
    background: linear-gradient(180deg, #4f46e5, #7c3aed);
  }
  .halo button:hover { filter: brightness(1.12); }
  .halo button:focus-visible { outline: 2px solid #a5b4fc; outline-offset: 3px; }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  @media (prefers-reduced-motion: reduce) { .halo { animation: none; } }
</style>
</head>
<body>
  <div class="stage">
    <div class="halo">
      <button type="button">立即开始</button>
    </div>
    <p class="hint">两道亮弧沿按钮边缘匀速环绕</p>
  </div>
</body>
</html>`;

export const starBorderAsset: AssetManifest = {
  id: "reactbits-star-border",
  title: "星轨流边按钮",
  titleEn: "Star Border",
  description: "conic 渐变亮弧沿按钮边框匀速环绕一圈，主 CTA 的常亮聚光灯。",
  descriptionEn: "Conic light arcs orbiting a primary CTA's border.",
  category: "control",
  tags: ["react-bits", "控件", "CTA", "流光", "边框"],
  previewHtml: HTML,
  files: [{ name: "reactbits-star-border.html", language: "html", content: HTML }],
  prompt:
    "请把「星轨流边按钮」装进我的项目：按钮外套一层 2px 的光环容器（conic-gradient 只留两段亮弧、其余透明），用 @property --angle 注册角度变量 + keyframes 旋转让亮弧匀速绕行（约 3.6s/圈）；内层按钮圆角略小形成 1~2px 光边；focus-visible 有独立描边；尊重 prefers-reduced-motion（光带静止在顶部）。先看现有主按钮体系，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/animations/star-border",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
