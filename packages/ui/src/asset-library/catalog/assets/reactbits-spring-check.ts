import type { AssetManifest } from "../types.js";

/**
 * 弹簧对勾（V5 扩批，灵感来自 React Bits 的 Spring Check）：
 * 点击后圆圈回弹收拢、对勾分段画出。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>弹簧对勾</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 20px; }
  .check {
    width: 74px; height: 74px; border-radius: 50%; cursor: pointer; border: 0; padding: 0;
    background: #141b2e; border: 3px solid #2c3750; position: relative;
    transition: background 0.25s, border-color 0.25s, transform 0.35s cubic-bezier(0.3, 1.6, 0.4, 1);
  }
  .check svg { position: absolute; inset: 0; margin: auto; }
  .check path {
    fill: none; stroke: #64748b; stroke-width: 5; stroke-linecap: round; stroke-linejoin: round;
    stroke-dasharray: 48; stroke-dashoffset: 48;
  }
  .check.on { background: linear-gradient(180deg, #10b981, #059669); border-color: #34d399; transform: scale(1.06); }
  .check.on path { stroke: #fff; animation: draw 0.45s 0.1s cubic-bezier(0.4, 0, 0.3, 1) forwards; }
  .check:active { transform: scale(0.92); }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  @media (prefers-reduced-motion: reduce) { .check path { animation: none; } .check.on path { stroke-dashoffset: 0; } }
</style>
</head>
<body>
  <div class="stage">
    <button class="check" id="check" role="checkbox" aria-checked="false" aria-label="完成">
      <svg width="40" height="40" viewBox="0 0 40 40" aria-hidden="true">
        <path d="M10 21 L18 29 L31 13"></path>
      </svg>
    </button>
    <p class="hint">点一下：回弹 + 对勾分段画出</p>
  </div>
  <script>
    var button = document.getElementById("check");
    button.addEventListener("click", function () {
      var on = button.classList.toggle("on");
      button.setAttribute("aria-checked", on ? "true" : "false");
      // 未选中态把笔画复位，等待再次画出
      if (!on) {
        var path = button.querySelector("path");
        path.style.animation = "none";
        path.getBoundingClientRect(); // 强制回流，让 dashoffset 复位生效
        path.style.animation = "";
      }
    });
  </script>
</body>
</html>`;

export const springCheckAsset: AssetManifest = {
  id: "reactbits-spring-check",
  title: "弹簧对勾",
  titleEn: "Spring Check",
  description: "点击后圆圈回弹放大、对勾以描边动画分段画出，完成的仪式感。",
  descriptionEn: "A springy checkbox whose tick strokes itself in on completion.",
  category: "control",
  tags: ["react-bits", "控件", "对勾", "回弹", "完成态"],
  previewHtml: HTML,
  files: [{ name: "reactbits-spring-check.html", language: "html", content: HTML }],
  prompt:
    "请把「弹簧对勾」装进我的项目：圆形 checkbox，选中时底色渐变成绿色、整体 scale(1.06) 回弹（cubic-bezier 弹性），对勾 SVG 用 stroke-dasharray/dashoffset 48→0 的描边动画分段画出（延迟 0.1s）；按下瞬间 scale(0.92) 压手感；取消时笔画复位；role=checkbox + aria-checked 同步；尊重 prefers-reduced-motion（直接显示对勾）。先看现有勾选/完成场景，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/micro-interactions/spring-check",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
