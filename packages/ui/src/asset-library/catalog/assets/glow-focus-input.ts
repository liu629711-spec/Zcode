import type { AssetManifest } from "../types.js";

/**
 * 辉光输入框（V2-4 首批收录，灵感来自 UIverse 的 Glow Input）：
 * 聚焦时底部渐变线从中间展开、外圈柔光泛起，浮动标签上移。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：真实输入框可敲字，聚焦态全套动效。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>辉光输入框</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #0d1526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .field { position: relative; width: min(320px, 86vw); }
  .field input {
    width: 100%; padding: 16px 14px 12px; border-radius: 12px;
    border: 1px solid rgb(148 163 184 / 0.25); background: #121826;
    color: #e6edf7; font-size: 15px; outline: none;
    transition: border-color 0.25s ease, box-shadow 0.25s ease;
  }
  .field label {
    position: absolute; left: 14px; top: 15px; padding: 0 4px;
    color: #7c8db0; font-size: 14px; pointer-events: none;
    transition: top 0.2s ease, font-size 0.2s ease, color 0.2s ease;
  }
  .field input:hover { border-color: rgb(148 163 184 / 0.45); }
  .field input:focus {
    border-color: rgb(56 189 248 / 0.7);
    box-shadow: 0 0 0 4px rgb(56 189 248 / 0.12), 0 0 22px rgb(56 189 248 / 0.25);
  }
  .field input:focus + label,
  .field input:not(:placeholder-shown) + label {
    top: -8px; font-size: 11.5px; color: #7dd3fc; background: #0d1322; border-radius: 4px;
  }
  .line {
    position: absolute; left: 12%; right: 12%; bottom: 0; height: 2px; border-radius: 2px;
    background: linear-gradient(90deg, #38bdf8, #a78bfa);
    transform: scaleX(0); transition: transform 0.35s cubic-bezier(0.3, 0.9, 0.3, 1);
  }
  .field input:focus ~ .line { transform: scaleX(1); }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .field *, .line { transition: none; } }
</style>
</head>
<body>
  <div class="field">
    <input type="email" id="mail" placeholder=" " autocomplete="off">
    <label for="mail">工作邮箱</label>
    <span class="line" aria-hidden="true"></span>
  </div>
  <p class="hint">点进输入框，看渐变线展开</p>
</body>
</html>`;

export const glowFocusInputAsset: AssetManifest = {
  id: "glow-focus-input",
  title: "辉光输入框",
  titleEn: "Glow Focus Input",
  description: "聚焦时渐变线展开加外圈柔光的输入框，浮动标签。",
  descriptionEn: "An input whose focus glows and expands a gradient line.",
  category: "control",
  tags: ["css", "输入框", "聚焦动效", "uiverse", "表单"],
  previewHtml: HTML,
  files: [{ name: "glow-focus-input.html", language: "html", content: HTML }],
  prompt:
    "请把「辉光输入框」装进我的项目：一个聚焦动效输入框——默认灰描边暗底，聚焦时外圈泛柔光（box-shadow 双层）且底部一条青→紫渐变细线从中间向两侧展开（scaleX）；浮动标签默认在框内，输入或聚焦时缩到框上沿变成小字（:placeholder-shown 技法，placeholder 留空格）；label 用 for 关联输入框保证可访问性；尊重 prefers-reduced-motion（无过渡）。先看现有表单体系，作为输入变体融入，不要覆盖全局 input 样式。",
  source: {
    site: "UIverse",
    url: "https://uiverse.io",
    license: "CC BY 4.0（按件署名，效果自实现）",
  },
};
