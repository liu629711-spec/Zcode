import type { AssetManifest } from "../types.js";
import { renderPromptCard } from "../prompt-card.js";

/**
 * 「毛玻璃登录页」口令卡（自制原创）：prompt 类货——没有代码图纸，口令本身就是货。
 * 说明页用共享模板（V2-3 重做：徽标 + 标题 + 说明 + 口令全文排版页）。
 */

const PROMPT =
  "请为我的项目做一个「毛玻璃登录页」：整页深色渐变背景（#0e1626 → #17233c），中央一张 backdrop-filter: blur 的半透明登录卡片（背景 rgb(255 255 255 / 6%)、1px 白色高光描边、圆角 20px、柔和投影），内含标题、邮箱与密码输入框、一个渐变主登录按钮和「忘记密码」链接；输入框聚焦时有柔和的蓝色描边过渡，卡片入场带一次轻微上浮淡入。要求：单文件组件、移动端卡片宽度自适应、动效克制并尊重 prefers-reduced-motion；先看现有的路由、表单与样式方案再融入，不要覆盖全局样式。";

export const frostedGlassLoginAsset: AssetManifest = {
  id: "frosted-glass-login-prompt",
  title: "毛玻璃登录页",
  description: "口令卡：一句成套口令，让智能体现做一张毛玻璃质感登录页。",
  category: "prompt",
  tags: ["prompt", "登录页", "毛玻璃", "口令"],
  previewHtml: renderPromptCard(
    { title: "毛玻璃登录页", prompt: PROMPT },
    "不附带代码图纸，<b>口令本身就是货</b>。发进会话后，智能体会现做一张 backdrop-filter 毛玻璃登录卡片：深色渐变底、半透明高光描边、聚焦过渡与响应式都已在口令里约定。",
  ),
  files: [],
  prompt: PROMPT,
};
