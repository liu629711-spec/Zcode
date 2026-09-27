import type { AssetManifest } from "../types.js";

/**
 * 渐变流光边框卡片（react 类种子货，自制原创）：外层 1.5px 的 conic-gradient
 * 旋转流动，内层同色底盖住中间只留边框发光；@property 让角度可被动画插值。
 *
 * react 货约定：files[0] 是 tsx 图纸（完整可复制源码）；previewHtml 不在本文件写，
 * 由 catalog/index.ts 从 preview-html.ts 的 REACT_PREVIEW_HTML 接线（构建期
 * scripts/build-asset-previews.mjs 生成）。demo 源码避免反引号，嵌入模板串不用转义。
 */

const DEMO_TSX = `// @property 注册角度变量后 conic-gradient 才能被动画插值（Chromium/新版浏览器）。
const CSS = [
  "@property --gb-angle { syntax: '<angle>'; initial-value: 0deg; inherits: false; }",
  "* { box-sizing: border-box; margin: 0; }",
  "body { min-height: 100vh; display: grid; place-items: center; background: #0b101a; font-family: system-ui, 'PingFang SC', 'Microsoft YaHei', sans-serif; }",
  ".gb-card { position: relative; width: 320px; padding: 1.5px; border-radius: 18px; background: conic-gradient(from var(--gb-angle), #38bdf8, #a855f7, #f472b6, #facc15, #38bdf8); animation: gb-flow 5s linear infinite; }",
  ".gb-body { border-radius: 16.5px; background: #0d1420; padding: 26px 24px; color: #e6edf7; }",
  ".gb-badge { display: inline-block; padding: 3px 10px; border-radius: 999px; background: rgb(168 85 247 / 0.16); color: #c4b5fd; font-size: 12px; letter-spacing: 0.08em; }",
  ".gb-title { margin: 14px 0 8px; font-size: 20px; }",
  ".gb-text { font-size: 14px; line-height: 1.7; color: #9fb1cc; }",
  ".gb-cta { margin-top: 18px; padding: 9px 18px; border: 0; border-radius: 10px; background: linear-gradient(135deg, #38bdf8, #a855f7); color: #0b101a; font-size: 14px; font-weight: 600; cursor: pointer; }",
  ".gb-cta:focus-visible { outline: 2px solid #c4b5fd; outline-offset: 2px; }",
  "@keyframes gb-flow { to { --gb-angle: 360deg; } }",
  "@media (prefers-reduced-motion: reduce) { .gb-card { animation: none; } }",
].join("\\n");

export default function GlowBorderCard() {
  return (
    <div>
      <style>{CSS}</style>
      <div className="gb-card">
        <div className="gb-body">
          <span className="gb-badge">NEW</span>
          <h3 className="gb-title">渐变流光边框</h3>
          <p className="gb-text">
            conic-gradient 随角度旋转，沿圆角边框流动；暗色底上精致不喧闹，可直接替换卡片的 border。
          </p>
          <button className="gb-cta" type="button">
            查看详情
          </button>
        </div>
      </div>
    </div>
  );
}
`;

export const glowBorderCardAsset: AssetManifest = {
  id: "glow-border-card",
  title: "渐变流光边框卡片",
  description: "React 卡片区块：conic-gradient 沿圆角边框旋转流动的流光描边。",
  category: "block",
  tags: ["react", "边框", "渐变", "卡片", "暗色"],
  // 由 catalog/index.ts 从 REACT_PREVIEW_HTML["glow-border-card"] 接线（此处占位空串）
  previewHtml: "",
  files: [{ name: "GlowBorderCard.tsx", language: "tsx", content: DEMO_TSX }],
  prompt:
    "请把「渐变流光边框卡片」装进我的项目：一个 React 卡片区块，外层 1.5px 的 conic-gradient 描边随角度旋转形成流光（@property 注册角度变量做动画插值），内层实底盖住中间只留边框发光；暗色底配色，保留 focus-visible 可访问性并尊重 prefers-reduced-motion。先看现有卡片/区块组件风格，融入而不是覆盖。",
};
