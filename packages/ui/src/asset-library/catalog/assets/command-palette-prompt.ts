import type { AssetManifest } from "../types.js";
import { renderPromptCard } from "../prompt-card.js";

/**
 * 「命令面板」口令卡（自制原创）：prompt 类货——口令本身就是货。
 * 说明页用共享模板（V2-3 重做：徽标 + 标题 + 说明 + 口令全文排版页）。
 */

const PROMPT =
  "请为我的项目做一个「命令面板」：Ctrl/Cmd+K 呼出、Esc 关闭的居中模态——顶部搜索框自动聚焦、实时过滤；下方命令列表分「最近使用 / 快捷操作 / 页面导航」三组，每项带图标位 + 名称 + 键位提示；↑↓ 移动高亮、Enter 执行、鼠标悬停同步高亮；无结果时显示空态引导；点击遮罩关闭。要求：焦点管理完整（打开聚焦搜索框、关闭后焦点还给触发者）、列表用 roving tabindex 或 aria-activedescendant、动效仅 120ms 淡入缩放并尊重 prefers-reduced-motion；命令动作先接现有路由跳转，融入而不是覆盖。";

export const commandPaletteAsset: AssetManifest = {
  id: "command-palette-prompt",
  title: "命令面板",
  titleEn: "Command Palette",
  description: "口令卡：Ctrl+K 呼出的命令面板，键盘可达全套。",
  descriptionEn: "A prompt card for a keyboard-first Ctrl+K palette.",
  category: "prompt",
  tags: ["prompt", "命令面板", "快捷键", "口令"],
  previewHtml: renderPromptCard(
    { title: "命令面板", prompt: PROMPT },
    "不附带代码图纸，<b>口令本身就是货</b>。发进会话后，智能体会现做 Ctrl/Cmd+K 呼出的命令面板：分组过滤、全键盘操作、焦点圈与空态都已写进口令。",
  ),
  files: [],
  prompt: PROMPT,
};
