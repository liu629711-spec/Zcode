import type { AssetManifest } from "../types.js";

/**
 * 对勾复选框（V2-4 首批收录，灵感来自 UIverse 的 Animated Checkbox）：
 * 选中时 SVG 打勾路径沿 stroke 画出，带圆角方框缩放反馈，纯 CSS。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：真实 checkbox，点选打勾。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>对勾复选框</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #0d1526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #cdd8ea;
  }
  .check { display: flex; flex-direction: column; gap: 14px; }
  label { display: flex; align-items: center; gap: 12px; cursor: pointer; user-select: none; font-size: 14px; }
  input { position: absolute; opacity: 0; width: 0; height: 0; }
  .box {
    width: 26px; height: 26px; border-radius: 8px; flex: none;
    border: 2px solid #46536e; display: grid; place-items: center;
    transition: background 0.25s ease, border-color 0.25s ease, transform 0.25s cubic-bezier(0.2, 1.8, 0.4, 1);
    background: #121826;
  }
  svg { width: 15px; height: 15px; }
  svg path {
    fill: none; stroke: #062018; stroke-width: 3; stroke-linecap: round; stroke-linejoin: round;
    stroke-dasharray: 22; stroke-dashoffset: 22;
    transition: stroke-dashoffset 0.3s ease 0.05s;
  }
  input:checked + .box { background: linear-gradient(135deg, #34d399, #0ea5e9); border-color: transparent; transform: scale(1.06); }
  input:checked + .box svg path { stroke-dashoffset: 0; }
  input:focus-visible + .box { outline: 2px solid #60a5fa; outline-offset: 3px; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .box, svg path { transition: none; } }
</style>
</head>
<body>
  <div class="check">
    <label>
      <input type="checkbox" checked>
      <span class="box"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5 6.5 12 13 4.5"/></svg></span>
      同步到云端（默认开启）
    </label>
    <label>
      <input type="checkbox">
      <span class="box"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5 6.5 12 13 4.5"/></svg></span>
      完成后通知我
    </label>
    <label>
      <input type="checkbox">
      <span class="box"><svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8.5 6.5 12 13 4.5"/></svg></span>
      允许智能体自动重试
    </label>
  </div>
  <p class="hint">点选打勾，勾是"画"出来的</p>
</body>
</html>`;

export const checkmarkCheckboxAsset: AssetManifest = {
  id: "checkmark-checkbox",
  title: "对勾复选框",
  titleEn: "Checkmark Checkbox",
  description: "选中时打勾沿路径画出的复选框，overshoot 缩放反馈。",
  descriptionEn: "A checkbox whose check draws itself when ticked.",
  category: "control",
  tags: ["css", "checkbox", "表单", "uiverse", "描边动画"],
  previewHtml: HTML,
  files: [{ name: "checkmark-checkbox.html", language: "html", content: HTML }],
  prompt:
    "请把「对勾复选框」装进我的项目：一个描边动画复选框——圆角方框选中时填充渐变并做一次 1.06 倍 overshoot 缩放，白色打勾是 SVG path（stroke-dasharray/dashoffset 从 22 到 0）沿路径画出来的；未选中灰描边空框；真实 input[type=checkbox] 承载状态、focus-visible 描边；尊重 prefers-reduced-motion（无动画直接呈现）。适合设置项/待办清单。先看现有表单控件，融入而不是覆盖。",
  source: {
    site: "UIverse",
    url: "https://uiverse.io",
    license: "CC BY 4.0（按件署名，效果自实现）",
  },
};
