import type { AssetManifest } from "../types.js";

/**
 * 解码文字（自制原创）：字符先在随机字符池翻滚，再从左到右逐位落定；
 * JS 逐帧驱动，中文按码位处理，落定后与目标文本逐字相等。
 * V2-3 交互：主体文字 contenteditable，失焦后按新文案重新解码。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>解码文字</title>
<style>
  body {
    min-height: 100%; display: grid; place-items: center;
    background: linear-gradient(165deg, #0a1220 0%, #0b1930 55%, #081020 100%);
    font-family: ui-monospace, "Cascadia Code", Consolas, "PingFang SC", monospace;
  }
  .scramble {
    font-size: clamp(20px, 4.5vw, 34px); font-weight: 600; color: #93e6c8;
    letter-spacing: 0.05em; white-space: pre-wrap; text-align: center;
  }
  .scramble[contenteditable] { outline: none; cursor: text; caret-color: #93e6c8; min-width: 3em; }
  .hint {
    position: fixed; left: 50%; bottom: 18px; transform: translateX(-50%);
    margin: 0; font-size: 12px; letter-spacing: 0.08em; color: #4c6a5e; user-select: none;
  }
</style>
</head>
<body>
  <p class="scramble" id="scramble" contenteditable="true" spellcheck="false" aria-label="正在接入交互素材库"></p>
  <p class="hint">点文字可直接编辑，失焦后按新文案重新解码</p>
  <script>
    var pool = "ABCDEFGHJKMNPQRSTUVWXYZ0123456789<>/#%&@=+*";
    var el = document.getElementById("scramble");
    var reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    var gen = 0; // 代次令牌：重播时让旧动画自废，避免两套 tick 互相覆盖
    function decode(target) {
      var my = ++gen;
      el.setAttribute("aria-label", target);
      if (reduce || target === "") { el.textContent = target; return; } // 降级：直接呈现全文
      var frame = 0, settle = 3; // 每位字符翻滚 3 帧后落定
      (function tick() {
        if (my !== gen) return;
        var out = "";
        for (var i = 0; i < target.length; i++) {
          out += frame > (i + 1) * settle
            ? target.charAt(i)
            : pool.charAt(Math.floor(Math.random() * pool.length));
        }
        el.textContent = out;
        frame += 1;
        if (frame <= target.length * settle + settle) setTimeout(tick, 42);
      })();
    }
    decode("正在接入交互素材库");
    el.addEventListener("blur", function () { decode(el.textContent.trim()); });
    el.addEventListener("keydown", function (e) {
      if (e.key === "Enter") { e.preventDefault(); el.blur(); } // 单行：回车收起
    });
  </script>
</body>
</html>`;

export const textScrambleAsset: AssetManifest = {
  id: "text-scramble",
  title: "解码文字",
  titleEn: "Text Scramble",
  description: "字符从乱码翻滚到逐位落定的解码动效，中文友好。",
  descriptionEn: "Characters roll through glyphs then settle left to right.",
  category: "text-animation",
  tags: ["js", "文字动效", "解码", "加载"],
  previewHtml: HTML,
  files: [{ name: "text-scramble.html", language: "html", content: HTML }],
  prompt:
    "请把「解码文字」装进我的项目：一个字符解码（scramble）动效组件——文本出现时每位字符先在随机字符池里翻滚，再从左到右逐位落定为真实字符；用 setTimeout 逐帧驱动而不是 CSS，中文按码位处理；落定后必须与目标文本逐字相等，真实文本提前放进 aria-label 保证无障碍；尊重 prefers-reduced-motion（降级为直接显示全文）。适合标题或数据加载完成后的揭示动画。先看现有文案风格，融入而不是覆盖。",
};
