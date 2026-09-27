import type { AssetManifest } from "./types.js";

/**
 * 口令卡说明页模板（V2-3 重做，技术设计 §10）：prompt 类货的 preview 从
 * "空白渐变"换成有内容的排版页——左上角徽标 + 素材名 + 两三行说明 +
 * 口令全文（等宽引用块）+ 底部一行使用提示。纯 HTML 静态页，暗色系。
 * 6 件口令卡共用这一份模板，素材文件里只填 title/intro/prompt。
 */

function escapeHtml(text: string): string {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

export function renderPromptCard(asset: Pick<AssetManifest, "title" | "prompt">, intro: string): string {
  const title = escapeHtml(asset.title);
  return `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title} · 口令卡</title>
<style>
  * { box-sizing: border-box; }
  body {
    min-height: 100vh; margin: 0; padding: 34px 22px 26px;
    display: flex; flex-direction: column; align-items: center;
    background: linear-gradient(150deg, #0d1526 0%, #152040 55%, #090d1c 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #dbe4f0;
  }
  .card {
    width: min(640px, 100%); display: flex; flex-direction: column;
    padding: 26px 28px; border-radius: 20px;
    background: rgb(255 255 255 / 0.06); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(14px); box-shadow: 0 18px 44px rgb(0 0 0 / 0.35);
  }
  .badge {
    align-self: flex-start; display: inline-flex; align-items: center; gap: 7px;
    padding: 4px 12px; border-radius: 999px; font-size: 12px; letter-spacing: 0.14em;
    color: #7dd3fc; background: rgb(125 211 252 / 0.1); border: 1px solid rgb(125 211 252 / 0.35);
  }
  .badge::before { content: ""; width: 6px; height: 6px; border-radius: 50%; background: #7dd3fc; }
  h1 { margin: 16px 0 0; font-size: 24px; letter-spacing: 0.02em; color: #f0f5fc; }
  .intro { margin: 10px 0 0; font-size: 14px; line-height: 1.9; color: #aebad0; }
  .intro b { color: #e6edf5; }
  .cmd-label {
    margin-top: 20px; font-size: 12px; letter-spacing: 0.12em; color: #7c8db0;
  }
  blockquote {
    margin: 8px 0 0; padding: 14px 16px; border-radius: 12px;
    border-left: 3px solid #38bdf8; background: rgb(8 12 22 / 0.55);
    font: 13px/1.9 ui-monospace, "Cascadia Code", Consolas, monospace;
    color: #c8d6ea; white-space: pre-wrap; word-break: break-word;
    max-height: 200px; overflow: auto;
  }
  .foot { margin-top: 18px; padding-top: 14px; border-top: 1px dashed rgb(255 255 255 / 0.14); font-size: 12.5px; color: #7c8db0; }
  .foot b { color: #aebad0; }
</style>
</head>
<body>
  <main class="card">
    <span class="badge">口令卡 · PROMPT</span>
    <h1>${title}</h1>
    <p class="intro">${escapeHtml(intro)}</p>
    <div class="cmd-label">口令全文</div>
    <blockquote>${escapeHtml(asset.prompt)}</blockquote>
    <p class="foot"><b>复制或发到会话后，智能体会按口令现做。</b>适合放在新会话里直接点单，改几个词就是你的口味。</p>
  </main>
</body>
</html>`;
}
