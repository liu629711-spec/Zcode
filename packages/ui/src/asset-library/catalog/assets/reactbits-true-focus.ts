import type { AssetManifest } from "../types.js";

/**
 * 真聚焦取景框（V5 扩批，灵感来自 React Bits 的 TrueFocus）：
 * 相机对焦框在词与词之间跳动，模拟逐词对焦。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>真聚焦取景框</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 26px; }
  .scene { position: relative; padding: 10px 6px; }
  .scene p {
    margin: 0; max-width: 640px; text-align: center;
    font-size: clamp(22px, 4.5vw, 40px); font-weight: 800; letter-spacing: 0.04em;
    color: #3a4460; line-height: 1.6;
  }
  .scene p .on { color: #e6edf5; transition: color 0.35s ease; }
  .frame {
    position: absolute; width: 0; height: 0; pointer-events: none;
    border: 2px solid #a5b4fc; border-radius: 4px; opacity: 0;
    box-shadow: 0 0 18px rgba(165, 180, 252, 0.35), inset 0 0 12px rgba(165, 180, 252, 0.12);
    transition: all 0.4s cubic-bezier(0.3, 0.9, 0.3, 1);
  }
  .frame.show { opacity: 1; }
  /* 四角装饰（取景框角标） */
  .frame i { position: absolute; width: 10px; height: 10px; border: 2px solid #a5b4fc; }
  .frame i:nth-child(1) { top: -2px; left: -2px; border-right: 0; border-bottom: 0; }
  .frame i:nth-child(2) { top: -2px; right: -2px; border-left: 0; border-bottom: 0; }
  .frame i:nth-child(3) { bottom: -2px; left: -2px; border-right: 0; border-top: 0; }
  .frame i:nth-child(4) { bottom: -2px; right: -2px; border-left: 0; border-top: 0; }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
</style>
</head>
<body>
  <div class="stage">
    <div class="scene" id="scene">
      <p id="line">聚焦真正的重点</p>
      <div class="frame" id="frame" aria-hidden="true"><i></i><i></i><i></i><i></i></div>
    </div>
    <p class="hint">对焦框逐词跳动，未对焦的词暗下去</p>
  </div>
  <script>
    var scene = document.getElementById("scene");
    var line = document.getElementById("line");
    var frame = document.getElementById("frame");
    var words = Array.from(line.textContent.match(/\\S+/g));
    line.textContent = "";
    var spans = words.map(function (word, index) {
      var span = document.createElement("span");
      span.textContent = word;
      line.appendChild(span);
      if (index < words.length - 1) line.appendChild(document.createTextNode(" "));
      return span;
    });

    var current = -1;
    function focusNext() {
      if (current >= 0) spans[current].classList.remove("on");
      current = (current + 1) % spans.length;
      spans[current].classList.add("on");
      var box = spans[current].getBoundingClientRect();
      var base = scene.getBoundingClientRect();
      frame.style.width = box.width + 8 + "px";
      frame.style.height = box.height + 8 + "px";
      frame.style.left = box.left - base.left - 4 + "px";
      frame.style.top = box.top - base.top - 4 + "px";
      frame.classList.add("show");
    }

    focusNext();
    setInterval(focusNext, 1600);
  </script>
</body>
</html>`;

export const trueFocusAsset: AssetManifest = {
  id: "reactbits-true-focus",
  title: "真聚焦取景框",
  titleEn: "True Focus",
  description: "相机对焦框在词与词之间跳动，对焦词亮起，其余压暗，逐词强调。",
  descriptionEn: "A focus frame hopping word to word, lighting the focused one.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "取景框", "逐词", "强调"],
  previewHtml: HTML,
  files: [{ name: "reactbits-true-focus.html", language: "html", content: HTML }],
  prompt:
    "请把「真聚焦取景框」装进我的项目：一句话逐词包裹 span，一个带四角角标的对焦框（细描边 + 辉光）用 getBoundingClientRect 定位后在词间循环跳动（约 1.6s/词，位移带 cubic-bezier 缓动）；当前词文字亮起，其余保持压暗色；尺寸随布局变化要能重算（ResizeObserver 或重排时刷新）。先看现有 slogan/卖点强调场景，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/true-focus",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
