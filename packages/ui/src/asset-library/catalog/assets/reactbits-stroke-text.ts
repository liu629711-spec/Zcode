import type { AssetManifest } from "../types.js";

/**
 * 描边渐显文字（V5 扩批，灵感来自 React Bits 的 Stroke Text）：
 * 空心描边字随滚动进度被"实色填充"擦亮。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>描边渐显文字</title>
<style>
  body {
    min-height: 100vh; margin: 0;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .spacer { height: 46vh; display: grid; place-items: center; }
  .spacer p { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; }
  h1 {
    margin: 0 auto; max-width: 780px; padding: 0 24px;
    font-size: clamp(34px, 8vw, 72px); font-weight: 900; line-height: 1.25; text-align: center;
  }
  h1 .line {
    /* 描边底字 + 实色副本擦亮：副本宽度由滚动进度控制 */
    position: relative; color: transparent;
    -webkit-text-stroke: 1px #4a5878;
  }
  h1 .line::before {
    content: attr(data-text);
    position: absolute; inset: 0; color: #e6edf5;
    -webkit-text-stroke: 0;
    width: var(--fill, 0%); overflow: hidden; white-space: nowrap;
  }
</style>
</head>
<body>
  <div class="spacer"><p>向下滚动，描边被逐步擦亮 ↓</p></div>
  <h1>
    <span class="line" data-text="想法落地成界面">想法落地成界面</span><br>
    <span class="line" data-text="只隔一行口令">只隔一行口令</span>
  </h1>
  <div class="spacer"></div>
  <script>
    var lines = document.querySelectorAll(".line");
    function onScroll() {
      var viewportMid = window.innerHeight / 2;
      lines.forEach(function (line) {
        var box = line.getBoundingClientRect();
        // 元素中心越过视口中线的进度 → 填充宽度
        var progress = 1 - Math.min(1, Math.max(0, (box.top + box.height / 2) / viewportMid));
        line.style.setProperty("--fill", (progress * 100).toFixed(1) + "%");
      });
    }
    onScroll();
    document.addEventListener("scroll", onScroll, { passive: true });
  </script>
</body>
</html>`;

export const strokeTextAsset: AssetManifest = {
  id: "reactbits-stroke-text",
  title: "描边渐显文字",
  titleEn: "Stroke Text",
  description: "空心描边大标题随滚动进度被实色逐列擦亮，滚动叙事感。",
  descriptionEn: "Outlined headlines fill with color as scroll progress advances.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "描边", "滚动", "填充"],
  previewHtml: HTML,
  files: [{ name: "reactbits-stroke-text.html", language: "html", content: HTML }],
  prompt:
    "请把「描边渐显文字」装进我的项目：大标题做成描边空心字（-webkit-text-stroke），同一文本用 ::before content:attr(data-text) 叠一份实色副本，副本容器宽度由滚动进度驱动（元素中心相对视口中线的距离映射 0~100%，overflow hidden 裁切实现「擦亮」）；监听 scroll 用 passive + rAF 节流。先看现有滚动叙事区块，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/stroke-text",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
