import type { AssetManifest } from "../types.js";

/**
 * 「毛玻璃登录页」口令卡（自制原创）：prompt 类货——没有代码图纸，口令本身就是货。
 * preview 是一张简洁说明页。
 */

const PREVIEW_HTML = `<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>毛玻璃登录页 · 口令卡</title>
<style>
  body {
    min-height: 100vh; margin: 0; display: grid; place-items: center;
    background: linear-gradient(150deg, #0e1626 0%, #17233c 55%, #0b1220 100%);
    font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif; color: #dbe4f0;
  }
  .note {
    width: min(420px, 88vw); padding: 28px 30px; border-radius: 20px;
    background: rgb(255 255 255 / 0.06); border: 1px solid rgb(255 255 255 / 0.14);
    backdrop-filter: blur(14px);
  }
  h1 { margin: 0 0 6px; font-size: 20px; }
  .kind { font-size: 12px; color: #7dd3fc; letter-spacing: 0.12em; }
  p { margin: 14px 0 0; font-size: 14px; line-height: 1.8; color: #aebad0; }
  b { color: #e6edf5; }
</style>
</head>
<body>
  <div class="note">
    <div class="kind">口令卡 · PROMPT</div>
    <h1>毛玻璃登录页</h1>
    <p>这是一件<b>口令货</b>：不附带代码图纸，<b>口令本身就是货</b>。</p>
    <p>复制或发进会话后，智能体会按口令现做一张 backdrop-filter 毛玻璃登录卡片：深色渐变底、半透明高光描边、聚焦过渡与响应式都已在口令里约定。</p>
    <p>适合放在<b>新会话</b>里直接点单，也可改几个词换成你自己的口味。</p>
  </div>
</body>
</html>`;

export const frostedGlassLoginAsset: AssetManifest = {
  id: "frosted-glass-login-prompt",
  title: "毛玻璃登录页",
  description: "口令卡：一句成套口令，让智能体现做一张毛玻璃质感登录页。",
  category: "prompt",
  tags: ["prompt", "登录页", "毛玻璃", "口令"],
  previewHtml: PREVIEW_HTML,
  files: [],
  prompt:
    "请为我的项目做一个「毛玻璃登录页」：整页深色渐变背景（#0e1626 → #17233c），中央一张 backdrop-filter: blur 的半透明登录卡片（背景 rgb(255 255 255 / 6%)、1px 白色高光描边、圆角 20px、柔和投影），内含标题、邮箱与密码输入框、一个渐变主登录按钮和「忘记密码」链接；输入框聚焦时有柔和的蓝色描边过渡，卡片入场带一次轻微上浮淡入。要求：单文件组件、移动端卡片宽度自适应、动效克制并尊重 prefers-reduced-motion；先看现有的路由、表单与样式方案再融入，不要覆盖全局样式。",
};
