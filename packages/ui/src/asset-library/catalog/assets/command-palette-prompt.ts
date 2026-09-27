import type { AssetManifest } from "../types.js";

/**
 * 「命令面板」口令卡（自制原创）：prompt 类货——口令本身就是货。
 */

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>命令面板 · 口令卡</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: linear-gradient(150deg, #101426 0%, #1a1f3d 55%, #0c0f20 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #dbe4f0;
  }
  .note {
    width: min(420px, 88vw); padding: 28px 30px; border-radius: 20px;
    background: rgb(255 255 255 / 0.06); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(14px);
  }
  h1 { margin: 0 0 6px; font-size: 20px; }
  .kind { font-size: 12px; color: #c4b5fd; letter-spacing: 0.12em; }
  p { margin: 14px 0 0; font-size: 14px; line-height: 1.8; color: #aebad0; }
  b { color: #e6edf5; }
  kbd {
    padding: 1px 7px; border-radius: 6px; font-size: 12px;
    background: rgb(255 255 255 / 0.1); border: 1px solid rgb(255 255 255 / 0.2);
  }
</style>
</head>
<body>
  <div class="note">
    <div class="kind">口令卡 · PROMPT</div>
    <h1>命令面板</h1>
    <p>这是一件<b>口令货</b>：不附带代码图纸，<b>口令本身就是货</b>。</p>
    <p>复制或发进会话后，智能体会按口令现做 <kbd>Ctrl</kbd>+<kbd>K</kbd> 呼出的命令面板：分组过滤、全键盘操作、焦点圈与空态都已写进口令。</p>
    <p>适合放在<b>新会话</b>里直接点单，命令项可以换成你自己的动作表。</p>
  </div>
</body>
</html>`;

export const commandPaletteAsset: AssetManifest = {
  id: "command-palette-prompt",
  title: "命令面板",
  titleEn: "Command Palette",
  description: "口令卡：Ctrl+K 呼出的命令面板，键盘可达全套。",
  descriptionEn: "A prompt card for a keyboard-first Ctrl+K palette.",
  category: "prompt",
  tags: ["prompt", "命令面板", "快捷键", "口令"],
  previewHtml: PREVIEW_HTML,
  files: [],
  prompt:
    "请为我的项目做一个「命令面板」：Ctrl/Cmd+K 呼出、Esc 关闭的居中模态——顶部搜索框自动聚焦、实时过滤；下方命令列表分「最近使用 / 快捷操作 / 页面导航」三组，每项带图标位 + 名称 + 键位提示；↑↓ 移动高亮、Enter 执行、鼠标悬停同步高亮；无结果时显示空态引导；点击遮罩关闭。要求：焦点管理完整（打开聚焦搜索框、关闭后焦点还给触发者）、列表用 roving tabindex 或 aria-activedescendant、动效仅 120ms 淡入缩放并尊重 prefers-reduced-motion；命令动作先接现有路由跳转，融入而不是覆盖。",
};
