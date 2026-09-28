import type { AssetManifest } from "../types.js";

/**
 * 「上下文引用卡」逐字收录（V3-3 素材库）。
 *
 * 来源：Beautiful UI · github.com/TurboKach/ai-native-react-components · components/context-cards.tsx
 * 作者：Turbo（beautifului.dev）
 * 许可：MIT（Copyright (c) 2026 Turbo）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const BeautifulUiContextCardsAsset: AssetManifest = {
  id: "beautifului-context-cards",
  title: "上下文引用卡",
  titleEn: "Context Cards",
  description: "智能体引用过的资料卡：来源徽标、正文片段、字符数与匹配度。",
  descriptionEn: "Context cards citing retrieved sources with badges and match scores.",
  category: "block",
  tags: ["ai-chat","引用","资料","beautifului","检索"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "context-cards.tsx",
      "language": "tsx",
      "content": "/*!\n * 上下文引用卡 · context-cards.tsx\n *\n * 来源：github.com/TurboKach/ai-native-react-components · components/context-cards.tsx\n * 原址：https://github.com/TurboKach/ai-native-react-components/blob/main/components/context-cards.tsx\n * 作者：Turbo（beautifului.dev）\n * 版权：Copyright (c) 2026 Turbo\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n\"use client\";\r\n\r\nimport { useEffect, useState } from \"react\";\r\n\r\n/* ─────────────────────────────────────────────────────────\r\n * CONTEXT CARDS\r\n * Retrieved chunks enter once, then remain available.\r\n * ───────────────────────────────────────────────────────── */\r\n\r\nconst CHUNKS = [\r\n  {\r\n    title: \"Vendor onboarding rule\",\r\n    chars: \"290 characters\",\r\n    body: \"Cold-chain certification must be verified before a new dairy can be added to the reorder workflow.\",\r\n    source: \"Dairy Onboarding SOP.pdf\",\r\n    badge: \"PDF\",\r\n    tone: \"bg-red\",\r\n  },\r\n  {\r\n    title: \"Seasonal demand row\",\r\n    chars: \"1,250 characters\",\r\n    body: \"Q4 velocity table: pistachio +18%, vanilla +6%, rocky road -11%; retire flavors below 40 scoops weekly.\",\r\n    source: \"Sales Velocity Export.csv\",\r\n    badge: \"CSV\",\r\n    tone: \"bg-green\",\r\n  },\r\n];\r\n\r\nexport default function ContextCards() {\r\n  const [chipsShown, setChipsShown] = useState(false);\r\n\r\n  useEffect(() => {\r\n    const chips = setTimeout(() => setChipsShown(true), 700);\r\n    return () => clearTimeout(chips);\r\n  }, []);\r\n\r\n  return (\r\n    <div className=\"flex w-full max-w-95 flex-col gap-2\">\r\n      <div\r\n        className=\"flex items-center gap-2 px-0.5\"\r\n        style={{ animation: \"fade-in 400ms ease-out both\" }}\r\n      >\r\n        <span className=\"text-[13px] font-semibold text-ink\">All chunks</span>\r\n        <span className=\"inline-flex h-5 items-center rounded-md bg-inset px-1.5 text-[11.5px] font-medium text-ink-2 shadow-hairline tabular-nums\">\r\n          32\r\n        </span>\r\n      </div>\r\n\r\n      {CHUNKS.map((chunk, i) => (\r\n        <div\r\n          key={chunk.title}\r\n          className=\"overflow-hidden rounded-card bg-surface shadow-card\"\r\n          style={{\r\n            animation: `fade-up 400ms cubic-bezier(0.23,1,0.32,1) ${i * 100}ms both`,\r\n          }}\r\n        >\r\n          <div className=\"primitive-card-bar flex items-center gap-2.5 border-b border-line\">\r\n            <span className=\"flex min-w-0 items-center gap-1.5 text-[13px] font-medium text-ink\">\r\n              <svg width=\"11\" height=\"11\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" strokeWidth=\"2.5\" strokeLinecap=\"round\"><path d=\"M4 6h16M4 12h16M4 18h10\" /></svg>\r\n              <span className=\"truncate\">{chunk.title}</span>\r\n            </span>\r\n            <span className=\"ml-auto shrink-0 text-[12px] text-ink-3 tabular-nums\">{chunk.chars}</span>\r\n          </div>\r\n          <p className=\"px-3 pt-2 pb-1 text-[12.5px] leading-relaxed text-ink-2\">\r\n            {chunk.body}\r\n          </p>\r\n          <div className=\"px-3 pb-3\">\r\n            <span\r\n              className=\"inline-flex h-6 items-center gap-1.5 rounded-full bg-inset px-2\r\n                text-[12px] font-medium text-ink-2 shadow-btn\r\n                transition-[opacity,transform,background-color] duration-300 hover:bg-hover\"\r\n              style={{\r\n                opacity: chipsShown ? 1 : 0,\r\n                transform: chipsShown ? \"scale(1)\" : \"scale(0.95)\",\r\n                transitionTimingFunction: \"cubic-bezier(0.23, 1, 0.32, 1)\",\r\n                transitionDelay: `${i * 80}ms`,\r\n              }}\r\n            >\r\n              <span className={`flex size-3.5 items-center justify-center rounded-[4px] ${chunk.tone} text-[7px] font-bold text-white`}>\r\n                {chunk.badge}\r\n              </span>\r\n              {chunk.source}\r\n              <svg width=\"9\" height=\"9\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" strokeWidth=\"2.5\" strokeLinecap=\"round\" strokeLinejoin=\"round\"><path d=\"M7 17L17 7M7 7h10v10\" /></svg>\r\n            </span>\r\n          </div>\r\n        </div>\r\n      ))}\r\n    </div>\r\n  );\r\n}\r\n"
    }
  ],
  prompt: "请把「上下文引用卡」装进我的项目：展示智能体检索到的资料——每张卡含紧挨标题上方的元信息行（匹配度百分比 + 操作按钮组，hover 才显现）、标题、两三行正文片段、底部一行带彩色圆点的来源（文件名 + 徽标 PDF/CSV，按类型配色）；卡片错时淡入（fade-up 依次进场）。用现有卡片的描边/圆角语汇。先看现有引用/来源展示组件，融入而不是覆盖。",
  source: { site: "Beautiful UI", url: "https://github.com/TurboKach/ai-native-react-components", license: "MIT" },
  preview: {
    theme: "beautifului",
    demo: "import Comp from \"./context-cards.tsx\";\nexport default function Demo() { return <Comp />; }",
  },
};
