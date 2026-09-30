import type { AssetManifest } from "../types.js";

/**
 * 机械翻牌字（V5 扩批，灵感来自 React Bits 的 SplitFlap Text）：
 * 机场/车站翻牌板——字符在滚动的字符环里停到目标位。
 * 效果自实现（未复制/移植原组件代码，许可见 source）。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>机械翻牌字</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: radial-gradient(120% 130% at 50% 0%, #101526 0%, #0a0e1a 60%, #070a12 100%);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  }
  .stage { display: grid; place-items: center; gap: 18px; }
  .board { display: flex; gap: 8px; padding: 18px 20px; border-radius: 14px;
    background: linear-gradient(180deg, #141b2e, #0d1322); border: 1px solid #2c3750;
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.45), inset 0 1px 0 rgba(255, 255, 255, 0.06); }
  .cell {
    width: 1.35em; height: 1.7em; overflow: hidden; position: relative; border-radius: 6px;
    background: #0a0f1c; border: 1px solid #1f2940;
    font-size: clamp(20px, 4.5vw, 34px); font-weight: 700; color: #ffd66e;
    text-align: center; line-height: 1.7em;
  }
  .cell .strip { position: absolute; left: 0; right: 0; top: 0; will-change: transform; }
  .cell .strip div { height: 1.7em; }
  .cell::after { /* 中缝阴影，制造翻牌板质感 */
    content: ""; position: absolute; left: 0; right: 0; top: 50%; height: 2px;
    transform: translateY(-1px); background: rgba(0, 0, 0, 0.55);
  }
  .controls { display: flex; gap: 10px; }
  button {
    border: 1px solid #2c3750; background: rgba(148, 163, 184, 0.08); color: #c7d2ea;
    font: inherit; font-size: 13px; padding: 7px 16px; border-radius: 999px; cursor: pointer;
  }
  button:hover { background: rgba(148, 163, 184, 0.16); }
  .hint { margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #5b6b8c; user-select: none; }
</style>
</head>
<body>
  <div class="stage">
    <div class="board" id="board" role="img" aria-label="DEPART 08:15"></div>
    <div class="controls">
      <button data-word="DEPART 08:15">航班</button>
      <button data-word="GATE 07 READY">登机口</button>
      <button data-word="HELLO WORLD">问好</button>
    </div>
    <p class="hint">字符在滚动环里停到目标位</p>
  </div>
  <script>
    var ALPHABET = " ABC0123456789:ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    var board = document.getElementById("board");

    function render(word) {
      board.textContent = "";
      Array.from(word).forEach(function (target, index) {
        var cell = document.createElement("div");
        cell.className = "cell";
        var strip = document.createElement("div");
        strip.className = "strip";
        // 从字符 0 滚到目标字符：速度逐格递减（前快后停的机械感）
        var steps = ALPHABET.indexOf(target) + ALPHABET.length * (index % 2);
        for (var s = 0; s <= steps; s++) {
          var glyph = document.createElement("div");
          glyph.textContent = ALPHABET[s % ALPHABET.length];
          strip.appendChild(glyph);
        }
        cell.appendChild(strip);
        board.appendChild(cell);
        strip.animate(
          [
            { transform: "translateY(0)" },
            { transform: "translateY(-" + steps * 1.7 + "em)" },
          ],
          { duration: 900 + index * 140, easing: "cubic-bezier(0.2, 0.7, 0.2, 1)", fill: "forwards" }
        );
      });
    }

    document.querySelectorAll("button").forEach(function (button) {
      button.addEventListener("click", function () { render(button.dataset.word); });
    });
    render("DEPART 08:15");
  </script>
</body>
</html>`;

export const splitFlapTextAsset: AssetManifest = {
  id: "reactbits-split-flap-text",
  title: "机械翻牌字",
  titleEn: "Split Flap Text",
  description: "机场翻牌板质感：每位字符在滚动环里逐格滚到目标字符，前快后停。",
  descriptionEn: "Airport board glyphs spinning into place with mechanical ease.",
  category: "text-animation",
  tags: ["react-bits", "文字动效", "翻牌", "机械", "数字板"],
  previewHtml: HTML,
  files: [{ name: "reactbits-split-flap-text.html", language: "html", content: HTML }],
  prompt:
    "请把「机械翻牌字」装进我的项目：一块深色翻牌板，文案按位拆进等宽单元格，每格通过一条字符序列条（字符环）从 0 滚到目标字符——用 Web Animations API 或 CSS transition 按位递增时长（约 140ms/位）做出前快后停的机械感；中缝加 2px 半透明阴影线还原翻牌板质感；支持换词重放（aria-label 同步真实文案）。先看现有数据大屏/状态条场景，融入而不是覆盖。",
  source: {
    site: "React Bits",
    url: "https://reactbits.dev/text-animations/split-flap-text",
    license: "MIT + Commons Clause v1.0（仅灵感参考，效果自实现，未复制原组件代码）",
  },
};
