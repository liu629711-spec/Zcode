import type { AssetManifest } from "../types.js";

/**
 * 「发光徽标」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/FeatureBadge/FeatureBadge.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiFeatureBadgeAsset: AssetManifest = {
  id: "rareui-feature-badge",
  title: "发光徽标",
  titleEn: "Feature Badge",
  description: "胶囊徽标：白色小签+说明文字，一道高光周期扫过。",
  category: "control",
  tags: ["rareui","徽标","高光","framer-motion","标签"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "FeatureBadge.tsx",
      "language": "tsx",
      "content": "/*!\n * 发光徽标 · FeatureBadge.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/FeatureBadge/FeatureBadge.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/FeatureBadge/FeatureBadge.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n\"use client\";\r\n\r\nimport React from \"react\";\r\nimport { motion } from \"framer-motion\";\r\n\r\nexport default function FeatureBadge({\r\n  badgeText = \"New\",\r\n  children = \"Multi-currency account\",\r\n  href = \"#\",\r\n}: {\r\n  badgeText?: string;\r\n  children?: React.ReactNode;\r\n  href?: string;\r\n}) {\r\n  return (\r\n    <a\r\n      href={href}\r\n      className=\"group relative inline-flex items-center gap-2 rounded-full bg-white/5 border border-white/10 pl-1 pr-3 py-1 backdrop-blur-md cursor-pointer overflow-hidden transition-all hover:bg-white/10 hover:border-white/20\"\r\n    >\r\n      {/* Continuous Glare Effect */}\r\n      <motion.div\r\n        className=\"absolute top-0 w-[50%] h-full bg-linear-to-r from-transparent via-white/20 to-transparent -skew-x-12 pointer-events-none z-0\"\r\n        initial={{ left: \"-100%\" }}\r\n        animate={{ left: \"200%\" }}\r\n        transition={{\r\n          repeat: Infinity,\r\n          repeatType: \"loop\",\r\n          duration: 3,\r\n          ease: \"linear\",\r\n          repeatDelay: 1, // Pause between shines\r\n        }}\r\n      />\r\n\r\n      <span className=\"relative z-10 rounded-full bg-white px-2.5 py-0.5 text-xs font-bold tracking-tight text-black shadow-sm\">\r\n        {badgeText}\r\n      </span>\r\n      <span className=\"relative z-10 text-sm font-medium text-neutral-300 group-hover:text-white transition-colors\">\r\n        {children}\r\n      </span>\r\n    </a>\r\n  );\r\n}\r\n"
    }
  ],
  prompt: "请把「发光徽标」装进我的项目：一个「新功能」提示徽标——毛玻璃胶囊（半透明底 + backdrop-blur + 细描边），左侧白色小圆角签写短标签（如 New），右侧说明文字；一道 50% 宽的透明-白-透明渐变高光带以 skewX 斜切，从左侧周期扫到右侧（带 repeatDelay 停顿），悬停时底色与描边略提亮、文字变白；整件可作为链接。先看现有的徽标/标签组件，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import FeatureBadge from \"./FeatureBadge\";\nexport default function Demo() { return <FeatureBadge badgeText=\"New\">Multi-currency account</FeatureBadge>; }",
  },
};
