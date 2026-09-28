import type { AssetManifest } from "../types.js";

/**
 * 轻通知 toast（自制原创）：右上滑入的类型化通知卡，底部寿命进度条，
 * 约 3 秒后右滑淡出销毁；容器 aria-live 播报。
 */

const HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>轻通知 toast</title>
<style>
  * { box-sizing: border-box; margin: 0; }
  body {
    min-height: 100%; display: grid; place-items: center;
    background: radial-gradient(110% 110% at 50% 0%, #101827 0%, #0a0f18 65%, #070b12 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  }
  .fire {
    padding: 11px 26px; border: 0; border-radius: 10px; cursor: pointer;
    font-size: 14px; font-weight: 600; color: #08131f;
    background: linear-gradient(135deg, #38bdf8, #6366f1);
  }
  .fire:focus-visible { outline: 2px solid #93c5fd; outline-offset: 3px; }
  .toasts { position: fixed; top: 18px; right: 18px; display: grid; gap: 10px; width: 290px; }
  .toast {
    position: relative; overflow: hidden; padding: 12px 14px 16px; border-radius: 12px;
    background: #141b28; border: 1px solid rgb(148 163 184 / 0.18);
    box-shadow: 0 12px 28px rgb(0 0 0 / 0.45);
    animation: in 0.3s ease-out;
  }
  .toast b { display: block; font-size: 13px; color: #e7edf7; margin-bottom: 3px; }
  .toast span { font-size: 12px; color: #9fb0c9; }
  .toast .bar { position: absolute; left: 0; bottom: 0; height: 3px; width: 100%; transform-origin: left; animation: life 3.2s linear forwards; }
  .ok .bar { background: #34d399; } .info .bar { background: #38bdf8; } .warn .bar { background: #fbbf24; }
  .toast.out { animation: out 0.3s ease-in forwards; }
  @keyframes in { from { opacity: 0; transform: translateX(24px); } }
  @keyframes out { to { opacity: 0; transform: translateX(24px); } }
  @keyframes life { to { transform: scaleX(0); } }
  @media (prefers-reduced-motion: reduce) {
    .toast, .toast.out, .toast .bar { animation: none; }
    .toast.out { display: none; }
    .toast .bar { transform: scaleX(0.4); }
  }
</style>
</head>
<body>
  <button class="fire" type="button" id="fire">弹出一条通知</button>
  <div class="toasts" id="toasts" aria-live="polite"></div>
  <script>
    var box = document.getElementById("toasts");
    var kinds = [
      ["ok", "已保存", "你的更改已同步到云端"],
      ["info", "新消息", "小雨给你发了一条留言"],
      ["warn", "存储告急", "免费空间仅剩 8%"]
    ];
    var n = 0;
    document.getElementById("fire").addEventListener("click", function () {
      var k = kinds[n++ % kinds.length];
      var t = document.createElement("div");
      t.className = "toast " + k[0];
      t.innerHTML = "<b>" + k[1] + "</b><span>" + k[2] + "</span><i class='bar'></i>";
      box.prepend(t);
      setTimeout(function () {
        t.classList.add("out");
        t.addEventListener("animationend", function () { t.remove(); });
        // reduced-motion 下没有动画事件，兜底 350ms 后直接移除
        setTimeout(function () { t.remove(); }, 350);
      }, 3200);
    });
  </script>
</body>
</html>`;

export const toastNotificationAsset: AssetManifest = {
  id: "toast-notification",
  title: "轻通知 toast",
  titleEn: "Toast Notification",
  description: "右上滑入的类型化通知卡，带寿命进度条自动消失。",
  descriptionEn: "Typed toasts sliding in with a lifetime bar, auto-dismissing.",
  category: "block",
  tags: ["js", "toast", "通知", "反馈"],
  previewHtml: HTML,
  files: [{ name: "toast-notification.html", language: "html", content: HTML }],
  prompt:
    "请把「轻通知 toast」装进我的项目：一个右上角滑入的轻通知组件——通知卡带类型色条（成功/信息/警告三种）、标题加一句话正文、底部细进度条显示剩余寿命，约 3 秒后右滑淡出自动销毁；容器 aria-live=\"polite\" 播报；支持多条堆叠；尊重 prefers-reduced-motion（直接出现/消失，无滑动）。先看现有的反馈/提示体系，融入而不是覆盖。",
};
