import type { AssetManifest } from "../types.js";

/**
 * 点击火花（V2-4 首批收录，灵感来自 React Bits 的 Click Spark）：
 * 点击处迸出 10 根短光线沿各自角度飞散淡出。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 * V2-3 交互：整页即演示场，点哪儿哪儿开花。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>点击火花</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100vh; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #0d1526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
    cursor: crosshair; overflow: hidden;
  }
  .hint {
    font-size: 15px; letter-spacing: 0.1em; color: #7c8db0; pointer-events: none;
    border: 1px dashed rgb(148 163 184 / 0.3); border-radius: 14px; padding: 18px 30px;
    user-select: none;
  }
  .spark {
    position: fixed; width: 14px; height: 2px; border-radius: 2px; pointer-events: none;
    background: var(--c, #7dd3fc); transform-origin: 0 50%;
    transform: rotate(var(--a));
    animation: fly 0.55s ease-out forwards;
  }
  @keyframes fly {
    to { transform: rotate(var(--a)) translateX(var(--d)) scaleX(0.25); opacity: 0; }
  }
  @media (prefers-reduced-motion: reduce) { .spark { animation: none; opacity: 0; } }
</style>
</head>
<body>
  <p class="hint">点 哪 儿 哪 儿 开 花</p>
  <script>
    var colors = ["#7dd3fc", "#e0f2fe", "#a5b4fc", "#67e8f9"];
    var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    document.addEventListener("pointerdown", function (e) {
      if (reduce) return; // 降级：不出火花
      for (var i = 0; i < 10; i++) {
        var s = document.createElement("span");
        s.className = "spark";
        s.style.left = e.clientX + "px";
        s.style.top = e.clientY + "px";
        var angle = (Math.PI * 2 * i) / 10 + Math.random() * 0.4;
        s.style.setProperty("--a", angle + "rad");
        s.style.setProperty("--d", 22 + Math.random() * 16 + "px");
        s.style.setProperty("--c", colors[i % colors.length]);
        document.body.appendChild(s);
        s.addEventListener("animationend", function () { this.remove(); });
      }
    });
  </script>
</body>
</html>`;

export const clickSparkAsset: AssetManifest = {
  id: "click-spark",
  title: "点击火花",
  titleEn: "Click Spark",
  description: "点击处迸出十根光线飞散的反馈特效，指示感强。",
  descriptionEn: "Lines burst from every click for playful feedback.",
  category: "control",
  tags: ["js", "点击反馈", "粒子", "react-bits", "微交互"],
  previewHtml: HTML,
  files: [{ name: "click-spark.html", language: "html", content: HTML }],
  prompt:
    "请把「点击火花」装进我的项目：一个点击反馈特效——指针按下处迸出约 10 根短光线，各自沿均分角度（带随机抖动）飞散 22~38px 同时缩小淡出，动画结束移除节点；光线颜色在浅青/白间轮换，pointer-events:none 不挡交互；尊重 prefers-reduced-motion（不生成火花）；只做视觉反馈不拦截业务点击。适合营销页彩蛋或空状态彩蛋，别用在密集表单。先看现有页面结构，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
