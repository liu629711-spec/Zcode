import type { AssetManifest } from "../types.js";

/**
 * 「烟雾散字」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/VaporSmokeText/VaporSmokeText.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiVaporSmokeTextAsset: AssetManifest = {
  id: "rareui-vapor-smoke-text",
  title: "烟雾散字",
  titleEn: "Vapor Smoke Text",
  description: "逐字从模糊烟雾中凝聚成字的标题。",
  category: "text-animation",
  tags: ["rareui","文字","烟雾","入场","标题"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "VaporSmokeText.tsx",
      "language": "tsx",
      "content": "/*!\n * 烟雾散字 · VaporSmokeText.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/VaporSmokeText/VaporSmokeText.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/VaporSmokeText/VaporSmokeText.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n'use client';\r\nimport React from 'react';\r\nimport { motion, Variants } from 'framer-motion';\r\nimport { cn } from '@/lib/utils';\r\n\r\ninterface VaporSmokeTextProps {\r\n  text: string;\r\n  className?: string;\r\n  trigger?: boolean;\r\n}\r\n\r\nexport const VaporSmokeText: React.FC<VaporSmokeTextProps> = ({\r\n  text,\r\n  className,\r\n  trigger = true,\r\n}) => {\r\n  const letters = text.split('');\r\n\r\n  const container: Variants = {\r\n    hidden: { opacity: 0 },\r\n    visible: (i = 1) => ({\r\n      opacity: 1,\r\n      transition: { staggerChildren: 0.08, delayChildren: 0.2 * i },\r\n    }),\r\n  };\r\n\r\n  const child: Variants = {\r\n    visible: {\r\n      opacity: 1,\r\n      filter: 'blur(0px)',\r\n      y: 0,\r\n      scale: 1,\r\n      rotate: 0,\r\n      transition: {\r\n        type: 'spring',\r\n        damping: 12,\r\n        stiffness: 100,\r\n        duration: 1.5,\r\n      },\r\n    },\r\n    hidden: {\r\n      opacity: 0,\r\n      filter: 'blur(20px)',\r\n      y: 20,\r\n      scale: 1.5,\r\n      rotate: 5,\r\n      transition: {\r\n        type: 'spring',\r\n        damping: 12,\r\n        stiffness: 100,\r\n      },\r\n    },\r\n  };\r\n\r\n  return (\r\n    <motion.div\r\n      className={cn('flex flex-wrap overflow-hidden', className)}\r\n      variants={container}\r\n      initial=\"hidden\"\r\n      animate={trigger ? 'visible' : 'hidden'}\r\n    >\r\n      {letters.map((letter, index) => (\r\n        <motion.span\r\n          key={index}\r\n          variants={child}\r\n          className=\"inline-block origin-bottom font-serif italic\"\r\n        >\r\n          {letter === ' ' ? '\\u00A0' : letter}\r\n        </motion.span>\r\n      ))}\r\n    </motion.div>\r\n  );\r\n};\r\n"
    }
  ],
  prompt: "请把「烟雾散字」装进我的项目：一段标题入场动效——每个字母初始是放大且高度模糊的「烟雾」态（filter: blur + 透明度 0 + 轻微上移/旋转），随后逐个（staggerChildren 0.08）凝聚到清晰原位，整段依次成形；字形上可加一点跟踪（letter-spacing）微调收束感；trigger 关闭时整体回到烟雾态便于重放。尊重 prefers-reduced-motion（直接显示）。先看现有的文字入场组件，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import { VaporSmokeText } from \"./VaporSmokeText\";\nexport default function Demo() { return <VaporSmokeText text=\"Smoke\" />; }",
  },
};
