import type { AssetManifest } from "../types.js";

/**
 * 昼夜开关（V2-4 首批收录，灵感来自 UIverse 的 Day/Night Toggle）：
 * 太阳/月亮一枚旋钮切换，轨道天空渐变随昼夜变化，纯 CSS。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：真实 checkbox，点按切换昼夜。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>昼夜开关</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100%; display: grid; place-items: center;
    background: #0a0d15;
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .sky { position: relative; display: inline-block; }
  .sky input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .track {
    display: block; width: 108px; height: 56px; border-radius: 999px;
    background: linear-gradient(180deg, #7dd3fc, #bae6fd);
    box-shadow: inset 0 2px 8px rgb(0 0 0 / 0.18);
    transition: background 0.5s ease;
    overflow: hidden; position: relative;
  }
  .star {
    position: absolute; width: 3px; height: 3px; border-radius: 50%; background: #fff;
    opacity: 0; transition: opacity 0.5s ease;
  }
  .s1 { left: 20px; top: 14px; } .s2 { left: 34px; top: 32px; } .s3 { left: 52px; top: 12px; } .s4 { left: 66px; top: 28px; }
  .knob {
    position: absolute; top: 5px; left: 5px; width: 46px; height: 46px; border-radius: 50%;
    background: radial-gradient(circle at 34% 30%, #fff7c2, #fbbf24 68%);
    box-shadow: 0 3px 9px rgb(0 0 0 / 0.3);
    transition: transform 0.5s cubic-bezier(0.3, 1.4, 0.4, 1), background 0.5s ease;
  }
  .knob::after { /* 月坑 */
    content: ""; position: absolute; right: 9px; top: 10px; width: 9px; height: 9px; border-radius: 50%;
    background: rgb(0 0 0 / 0.14); opacity: 0; transition: opacity 0.4s ease;
    box-shadow: -7px 12px 0 -2.5px rgb(0 0 0 / 0.14);
  }
  .sky input:checked + .track { background: linear-gradient(180deg, #1e2a52, #3b4a7d); }
  .sky input:checked + .track .star { opacity: 0.9; }
  .sky input:checked + .track .knob { transform: translateX(52px); background: radial-gradient(circle at 34% 30%, #f1f5f9, #cbd5e1 70%); }
  .sky input:checked + .track .knob::after { opacity: 1; }
  .sky input:focus-visible + .track { outline: 2px solid #60a5fa; outline-offset: 3px; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    font-size: 12px; letter-spacing: 0.08em; color: #4c5872; user-select: none;
  }
  @media (prefers-reduced-motion: reduce) { .track, .knob { transition: none; } }
</style>
</head>
<body>
  <label class="sky">
    <input type="checkbox" aria-label="切换昼夜模式">
    <span class="track" aria-hidden="true">
      <span class="star s1"></span><span class="star s2"></span><span class="star s3"></span><span class="star s4"></span>
      <span class="knob"></span>
    </span>
  </label>
  <p class="hint">点按切换：太阳落、月亮升</p>
</body>
</html>`;

export const dayNightToggleAsset: AssetManifest = {
  id: "day-night-toggle",
  title: "昼夜开关",
  titleEn: "Day-Night Toggle",
  description: "太阳翻成月亮、轨道变夜空的开关，纯 CSS 有戏。",
  descriptionEn: "A sun-to-moon switch with a sky that turns to night.",
  category: "control",
  tags: ["css", "开关", "昼夜", "uiverse", "主题切换"],
  previewHtml: HTML,
  files: [{ name: "day-night-toggle.html", language: "html", content: HTML }],
  prompt:
    "请把「昼夜开关」装进我的项目：一个纯 CSS 的昼夜主题开关——胶囊轨道白天是浅蓝天空渐变，拨过去变成深蓝夜空并浮现四颗小星；旋钮白天是带高光的太阳（暖黄径向渐变），切换后平移并变成带月坑的灰月亮；真实 checkbox 承载状态，focus-visible 有描边，切换缓动 0.5s 带 overshoot；尊重 prefers-reduced-motion（瞬切）。可作为明暗主题的入口控件，接现有主题切换状态。先看现有表单控件风格，融入而不是覆盖。",
  source: {
    site: "UIverse",
    url: "https://uiverse.io",
    license: "CC BY 4.0（按件署名，效果自实现）",
  },
};
