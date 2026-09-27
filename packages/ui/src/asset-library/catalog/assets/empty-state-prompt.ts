import type { AssetManifest } from "../types.js";
import { renderPromptCard } from "../prompt-card.js";

/**
 * 「空状态三变体」口令卡（自制原创）：prompt 类货——口令本身就是货。
 * 说明页用共享模板（V2-3 重做：徽标 + 标题 + 说明 + 口令全文排版页）。
 */

const PROMPT =
  "请为我的项目做一套「空状态组件」：统一的 EmptyState 组件出三种变体——无数据（插画位 + 「这里还什么都没有」+ 主行动按钮「新建」）、搜索无结果（放大镜位 + 「没有找到相关内容」+ 「清除搜索」文字按钮）、加载失败（断线位 + 「加载失败了」+ 「重试」按钮）；插画位做成插槽方便替换；垂直居中、间距宽松、文案两行以内。要求：暗色底适配、按钮接现有动作体系、插画用纯 CSS/SVG 自绘不引外部图片；先看现有的列表与错误处理约定，融入而不是覆盖。";

export const emptyStateAsset: AssetManifest = {
  id: "empty-state-prompt",
  title: "空状态三变体",
  titleEn: "Empty States Trio",
  description: "口令卡：无数据/无结果/失败三种空状态一套口令搞定。",
  descriptionEn: "A prompt card covering empty, no-result and error states.",
  category: "prompt",
  tags: ["prompt", "空状态", "组件", "口令"],
  previewHtml: renderPromptCard(
    { title: "空状态三变体", prompt: PROMPT },
    "不附带代码图纸，<b>口令本身就是货</b>。发进会话后，智能体会现做统一的 EmptyState 组件：无数据、搜索无结果、加载失败三种变体一次给全，插画位留成插槽。",
  ),
  files: [],
  prompt: PROMPT,
};
