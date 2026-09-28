import type { AssetManifest } from "../types.js";

/**
 * 「动效标签页」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/AnimatedTab/AnimatedTab.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiAnimatedTabAsset: AssetManifest = {
  id: "rareui-animated-tab",
  title: "动效标签页",
  titleEn: "Animated Tabs",
  description: "选中项滑动高亮块的标签页，悬停有预滑提示。",
  category: "control",
  tags: ["rareui","标签页","动效","framer-motion","导航"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "AnimatedTab.tsx",
      "language": "tsx",
      "content": "/*!\n * 动效标签页 · AnimatedTab.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/AnimatedTab/AnimatedTab.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/AnimatedTab/AnimatedTab.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n'use client';\r\n\r\nimport React, { useState } from 'react';\r\nimport { motion } from 'framer-motion';\r\nimport { cn } from '@/lib/utils';\r\n\r\nexport interface Tab {\r\n  id: string;\r\n  label: string;\r\n}\r\n\r\ninterface AnimatedTabsProps {\r\n  tabs: Tab[];\r\n  activeTab: string;\r\n  onChange: (id: string) => void;\r\n  className?: string;\r\n}\r\n\r\nexport const AnimatedTabs: React.FC<AnimatedTabsProps> = ({\r\n  tabs,\r\n  activeTab,\r\n  onChange,\r\n  className,\r\n}) => {\r\n  const [hoveredTab, setHoveredTab] = useState<string | null>(null);\r\n\r\n  return (\r\n    <div\r\n      className={cn(\r\n        'flex flex-row flex-nowrap items-center justify-center gap-1 rounded-full p-1 sm:gap-0 sm:p-1.5',\r\n        'bg-white/60 dark:bg-zinc-900/60', // Light/Dark glass effect\r\n        'border border-black/5 dark:border-white/10', // Border adaptation\r\n        'backdrop-blur-xl', // Strong glass effect\r\n        'shadow-2xl', // Container shadow\r\n        'max-w-full', // Ensure it doesn't overflow\r\n        className\r\n      )}\r\n    >\r\n      {tabs.map((tab) => {\r\n        const isActive = activeTab === tab.id;\r\n        const isHovered = hoveredTab === tab.id;\r\n\r\n        return (\r\n          <motion.button\r\n            key={tab.id}\r\n            onClick={() => onChange(tab.id)}\r\n            onMouseEnter={() => setHoveredTab(tab.id)}\r\n            onMouseLeave={() => setHoveredTab(null)}\r\n            whileTap={{ scale: 0.95 }}\r\n            className={cn(\r\n              'relative z-10 cursor-pointer rounded-full px-3 py-2 text-xs font-semibold whitespace-nowrap transition-colors duration-200 outline-none sm:px-4 sm:py-2.5 sm:text-sm md:px-6',\r\n              isActive\r\n                ? 'text-white dark:text-black' // Active text\r\n                : 'text-zinc-600 hover:text-zinc-900 dark:text-zinc-500 dark:hover:text-zinc-300' // Inactive text\r\n            )}\r\n            style={{\r\n              WebkitTapHighlightColor: 'transparent',\r\n            }}\r\n          >\r\n            {/* Active Pill with \"Transferring\" Shadow */}\r\n            {isActive && (\r\n              <motion.div\r\n                layoutId=\"active-pill\"\r\n                className=\"absolute inset-0 z-[-1] rounded-full bg-black shadow-xl dark:bg-white dark:shadow-[0_0_20px_rgba(255,255,255,0.3)]\"\r\n                transition={{\r\n                  type: 'spring',\r\n                  stiffness: 320,\r\n                  damping: 32,\r\n                  mass: 0.9,\r\n                }}\r\n              />\r\n            )}\r\n\r\n            {/* Hover Background - Subtle highlight */}\r\n            {isHovered && !isActive && (\r\n              <motion.div\r\n                layoutId=\"hover-pill\"\r\n                className=\"absolute inset-0 z-[-1] rounded-full bg-black/5 dark:bg-white/5\"\r\n                initial={{ opacity: 0 }}\r\n                animate={{ opacity: 1 }}\r\n                exit={{ opacity: 0 }}\r\n                transition={{ duration: 0.15 }}\r\n              />\r\n            )}\r\n\r\n            <span className=\"relative z-10\">{tab.label}</span>\r\n          </motion.button>\r\n        );\r\n      })}\r\n    </div>\r\n  );\r\n};\r\n\r\nexport default AnimatedTabs;\r\n"
    }
  ],
  prompt: "请把「动效标签页」装进我的项目：一个受控的标签页组件（props: tabs[{id,label}] / activeTab / onChange）——选中项的圆角高亮块是独立元素，在标签间以 spring 平滑滑动到新位置（layoutId 共享布局）；鼠标悬停某一项时高亮块先预滑过去做提示，移开回到选中项；标签文字随选中状态变粗变色；容器是浅底描边的胶囊条，深色模式一套对应配色。先看现有的分段控件/标签页组件，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import { useState } from \"react\";\nimport { AnimatedTabs } from \"./AnimatedTab\";\nexport default function Demo() {\n  const [tab, setTab] = useState(\"overview\");\n  return <AnimatedTabs tabs={[{ id: \"overview\", label: \"概述\" }, { id: \"api\", label: \"接口\" }, { id: \"settings\", label: \"设置\" }]} activeTab={tab} onChange={setTab} />;\n}",
  },
};
