// PromptMentionNode.ts 承载 Lexical 节点行为 + 全部分类图标 DOM 数据，
// 新增 plugin 图标后超出 max-lines(400) 门禁。图标数据/DOM 构建是纯展示常量，
// 与节点行为解耦到本文件；PromptMentionNode 继续 re-export 保持既有导入面兼容。
//
// PromptMentionNode 是 Lexical 自定义 DOM 节点，不能直接渲染 lucide-react 组件。
// 这里保留 lucide 的 IconNode 数据形状，再用原生 SVG DOM 生成图标。
const SVG_NAMESPACE = "http://www.w3.org/2000/svg";

export type MentionLucideIconNode = ReadonlyArray<
  readonly ["path" | "circle" | "rect", Readonly<Record<string, string>>]
>;
export const SKILL_MENTION_ICON_NODE = [
  [
    "path",
    {
      d: "m21.64 3.64-1.28-1.28a1.21 1.21 0 0 0-1.72 0L2.36 18.64a1.21 1.21 0 0 0 0 1.72l1.28 1.28a1.2 1.2 0 0 0 1.72 0L21.64 5.36a1.2 1.2 0 0 0 0-1.72",
    },
  ],
  ["path", { d: "m14 7 3 3" }],
  ["path", { d: "M5 6v4" }],
  ["path", { d: "M19 14v4" }],
  ["path", { d: "M10 2v2" }],
  ["path", { d: "M7 8H3" }],
  ["path", { d: "M21 16h-4" }],
  ["path", { d: "M11 3H9" }],
] as const satisfies MentionLucideIconNode;
export const SUBAGENT_MENTION_ICON_NODE = [
  ["path", { d: "M12 8V4H8" }],
  ["rect", { width: "16", height: "12", x: "4", y: "8", rx: "2" }],
  ["path", { d: "M2 14h2" }],
  ["path", { d: "M20 14h2" }],
  ["path", { d: "M15 13v2" }],
  ["path", { d: "M9 13v2" }],
] as const satisfies MentionLucideIconNode;
export const WHITEBOARD_MENTION_ICON_NODE = [
  [
    "path",
    {
      d: "M12 22a8 8 0 0 1-8-8 8.5 8.5 0 0 1 8.5-8.5H16a4 4 0 0 1 0 8h-1.5a2 2 0 0 0 0 4H16a6 6 0 0 1-4 4.5Z",
    },
  ],
  ["path", { d: "M7.5 14h.01" }],
  ["path", { d: "M9.5 10h.01" }],
  ["path", { d: "M14.5 9h.01" }],
  ["path", { d: "M17 12h.01" }],
] as const satisfies MentionLucideIconNode;
export const GOAL_COMMAND_MENTION_ICON_NODE = [
  ["path", { d: "M12 13V2l8 4-8 4" }],
  ["path", { d: "M20.561 10.222a9 9 0 1 1-12.55-5.29" }],
  ["path", { d: "M8.002 9.997a5 5 0 1 0 8.9 2.02" }],
] as const satisfies MentionLucideIconNode;
export const WORKFLOW_COMMAND_MENTION_ICON_NODE = [
  ["rect", { width: "8", height: "8", x: "3", y: "3", rx: "2" }],
  ["path", { d: "M7 11v4a2 2 0 0 0 2 2h4" }],
  ["rect", { width: "8", height: "8", x: "13", y: "13", rx: "2" }],
] as const satisfies MentionLucideIconNode;
export const COMPACT_COMMAND_MENTION_ICON_NODE = [
  ["path", { d: "M15 12h-5" }],
  ["path", { d: "M15 8h-5" }],
  ["path", { d: "M19 17V5a2 2 0 0 0-2-2H4" }],
  [
    "path",
    {
      d: "M8 21h12a2 2 0 0 0 2-2v-1a1 1 0 0 0-1-1H11a1 1 0 0 0-1 1v1a2 2 0 1 1-4 0V5a2 2 0 1 0-4 0v2a1 1 0 0 0 1 1h3",
    },
  ],
] as const satisfies MentionLucideIconNode;
export const COMMAND_MENTION_ICON_NODE = [
  [
    "path",
    {
      d: "M21 5v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2Z",
    },
  ],
  ["path", { d: "m9 15 6-6" }],
] as const satisfies MentionLucideIconNode;
export const SESSION_MENTION_ICON_NODE = [
  [
    "path",
    {
      d: "M14 9a2 2 0 0 1-2 2H6l-4 4V5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2z",
    },
  ],
  ["path", { d: "M18 9h2a2 2 0 0 1 2 2v10l-4-4h-6a2 2 0 0 1-2-2v-1" }],
] as const satisfies MentionLucideIconNode;
export const PLUGIN_MENTION_ICON_NODE = [
  ["path", { d: "M17 19a1 1 0 0 1-1-1v-2a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2a1 1 0 0 1-1 1z" }],
  ["path", { d: "M17 21v-2" }],
  ["path", { d: "M19 14V6.5a1 1 0 0 0-7 0v11a1 1 0 0 1-7 0V10" }],
  ["path", { d: "M21 21v-2" }],
  ["path", { d: "M3 5V3" }],
  ["path", { d: "M4 10a2 2 0 0 1-2-2V6a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2a2 2 0 0 1-2 2z" }],
  ["path", { d: "M7 5V3" }],
] as const satisfies MentionLucideIconNode;

/** 素材引用（可复用组件货）：lucide「component」四菱形。 */
export const COMPONENT_MENTION_ICON_NODE = [
  ["path", { d: "M5.5 8.5 9 12l-3.5 3.5L2 12l3.5-3.5Z" }],
  ["path", { d: "m12 2 3.5 3.5L12 9 8.5 5.5 12 2Z" }],
  ["path", { d: "M18.5 8.5 22 12l-3.5 3.5L15 12l3.5-3.5Z" }],
  ["path", { d: "m12 15 3.5 3.5L12 22l-3.5-3.5 3.5-3.5Z" }],
] as const satisfies MentionLucideIconNode;

/** 设计风格引用：lucide「palette」调色板。 */
export const PALETTE_MENTION_ICON_NODE = [
  [
    "path",
    {
      d: "M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z",
    },
  ],
  ["circle", { cx: "13.5", cy: "6.5", r: ".5", fill: "currentColor" }],
  ["circle", { cx: "17.5", cy: "10.5", r: ".5", fill: "currentColor" }],
  ["circle", { cx: "8.5", cy: "7.5", r: ".5", fill: "currentColor" }],
  ["circle", { cx: "6.5", cy: "12.5", r: ".5", fill: "currentColor" }],
] as const satisfies MentionLucideIconNode;

export function createMentionSvgIcon(iconNode: MentionLucideIconNode): SVGSVGElement {
  const svg = document.createElementNS(SVG_NAMESPACE, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("width", "16");
  svg.setAttribute("height", "16");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.5");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  svg.setAttribute("focusable", "false");
  svg.classList.add("inline-block", "align-middle", "shrink-0");

  for (const [tagName, attributes] of iconNode) {
    const node = document.createElementNS(SVG_NAMESPACE, tagName);
    for (const [name, value] of Object.entries(attributes)) {
      node.setAttribute(name, value);
    }
    svg.append(node);
  }

  return svg;
}
