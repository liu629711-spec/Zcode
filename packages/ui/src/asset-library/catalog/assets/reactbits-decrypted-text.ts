import type { AssetManifest } from "../types.js";

/**
 * 解密浮现文字（V5 扩批，灵感来自 React Bits 的 Decrypted Text）：
 * 文案先以随机字符乱码滚动，再从左到右逐位"解密"定格。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>解密浮现文字</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .stage { display: grid; place-items: center; gap: 18px; }
  h1 {
    margin: 0; font-size: clamp(24px, 5vw, 44px); font-weight: 800;
    letter-spacing: 0.06em; color: #e6edf5; min-height: 1.4em; text-align: center;
  }
  h1 .resolved { color: #a5b4fc; }
  h1 .scrambling { color: #64748b; opacity: 0.85; }
  button {
    border: 1px solid #2c3750; background: rgba(148, 163, 184, 0.08); color: #c7d2ea;
    font: inherit; font-size: 13px; padding: 7px 16px; border-radius: 999px; cursor: pointer;
  }
  button:hover { background: rgba(148, 163, 184, 0.16); }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
  @media (prefers-reduced-motion: reduce) { h1 { animation: none; } }
</style>
</head>
<body>
  <div class="stage">
    <h1 id="cipher">系统已就绪，欢迎回来</h1>
    <button id="replay">重新解密</button>
    <p class="hint">机密感的乱码滚动 → 逐位定格</p>
  </div>
  <script>
    var GLYPHS = "!<>-_\\\\/[]{}—=+*^?#________";
    var el = document.getElementById("cipher");
    var timer = null;

    function decode(text, speed) {
      clearInterval(timer);
      var frame = 0;
      var resolved = 0; // 已定格字符数
      var queue = Array.from(text).map(function (ch) { return { ch: ch }; });
      timer = setInterval(function () {
        frame += 1;
        // 每帧按节奏多解锁一位，未解锁位继续乱码
        resolved = Math.min(queue.length, Math.floor(frame / 2));
        el.textContent = "";
        queue.forEach(function (item, index) {
          var span = document.createElement("span");
          if (index < resolved) {
            span.className = "resolved";
            span.textContent = item.ch;
          } else {
            span.className = "scrambling";
            span.textContent = GLYPHS[Math.floor(Math.random() * GLYPHS.length)];
          }
          el.appendChild(span);
        });
        if (resolved === queue.length) clearInterval(timer);
      }, speed);
    }

    function replay() { decode(el.textContent, 45); }
    document.getElementById("replay").addEventListener("click", replay);
    decode("系统已就绪，欢迎回来", 45);
  </script>
</body>
</html>`;

export const decryptedTextAsset: AssetManifest = {
  id: "reactbits-decrypted-text",
  title: "解密浮现文字",
  titleEn: "Decrypted Text",
  description: "文案先以随机字符乱码滚动，再从左到右逐位解密定格，自带机密感。",
  descriptionEn: "Scrambled glyphs resolve left-to-right into the real message.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "乱码", "解密", "暗色"],
  previewHtml: HTML,
  files: [{ name: "reactbits-decrypted-text.html", language: "html", content: HTML }],
  prompt:
    "请把「解密浮现文字」装进我的项目：一段文字入场时先用随机符号（如 !<>-_[]*—）逐帧乱码滚动，然后从左到右每隔几帧多定格一位真实字符，直到全部解密完成；支持触发重放（进入视口/挂载时自动播一次）；中文按码位处理、乱码字符宽度对齐避免抖动；尊重 prefers-reduced-motion（直接显示最终文案）。先看现有标题/欢迎语场景，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/decrypted-text",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
