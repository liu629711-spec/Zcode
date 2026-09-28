import type { AssetManifest } from "../types.js";

/**
 * 「3D 立体按钮」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/ThreeDButton/ThreeDButton.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiThreeDButtonAsset: AssetManifest = {
  id: "rareui-three-d-button",
  title: "3D 立体按钮",
  titleEn: "3D Button",
  description: "多层内阴影叠出真实厚度的深色立体按钮。",
  category: "control",
  tags: ["rareui","按钮","3d","阴影","framer-motion"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "ThreeDButton.tsx",
      "language": "tsx",
      "content": "/*!\n * 3D 立体按钮 · ThreeDButton.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/ThreeDButton/ThreeDButton.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/ThreeDButton/ThreeDButton.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n\"use client\";\r\n\r\nimport React from \"react\";\r\nimport { motion } from \"framer-motion\";\r\n\r\n// Inline utility if needed\r\nimport { clsx, type ClassValue } from \"clsx\";\r\nimport { twMerge } from \"tailwind-merge\";\r\n\r\nimport Link from \"next/link\";\r\n\r\nfunction cn(...inputs: ClassValue[]) {\r\n  return twMerge(clsx(inputs));\r\n}\r\n\r\nexport default function ThreeDButton({\r\n  text = \"Book a call\",\r\n  className,\r\n  href,\r\n}: {\r\n  text?: string;\r\n  className?: string;\r\n  href?: string;\r\n}) {\r\n  const buttonContent = (\r\n    <motion.button\r\n      initial=\"initial\"\r\n      whileHover=\"hover\"\r\n      whileTap={{ scale: 0.98, y: 1 }}\r\n      transition={{ type: \"spring\", stiffness: 400, damping: 15 }}\r\n      className={cn(\r\n        \"group relative flex items-center justify-center rounded-[10px] bg-[#060612] px-6 py-3 text-white transition-colors cursor-pointer\",\r\n        className\r\n      )}\r\n      style={{\r\n        boxShadow:\r\n          \"inset 0px 1px 0.75px 0px rgba(255, 255, 255, 0.07), 0px 4px 4px 0px rgba(0, 0, 0, 0.25), 0px 0px 0px 1px rgb(47, 47, 55), 0px 4px 4px 0px rgba(0, 0, 0, 0.25), 0px 47.62px 46.23px 0px rgba(15, 15, 15, 0.4), 0px 27.25px 26.45px 0px rgba(15, 15, 15, 0.34), 0px 16.54px 16.06px 0px rgba(15, 15, 15, 0.29), 0px 9.97px 9.68px 0px rgba(15, 15, 15, 0.25)\",\r\n      }}\r\n    >\r\n      <span className=\"block whitespace-nowrap text-[15px] font-medium leading-[20px] text-white/90\">\r\n        {text}\r\n      </span>\r\n    </motion.button>\r\n  );\r\n\r\n  if (href) {\r\n    return <Link href={href}>{buttonContent}</Link>;\r\n  }\r\n\r\n  return buttonContent;\r\n}\r\n"
    }
  ],
  prompt: "请把「3D 立体按钮」装进我的项目：一颗深色立体按钮——厚度感来自多层叠加阴影（内顶 1px 白高光 + 一圈 1px 描边 + 由近及远 6 层愈拉愈长的暗影，模拟悬浮在深底上的实心块）；悬停时按钮轻微上浮（y 位移）阴影随之拉长，按下时压回并缩到 0.98（spring 400/15）；文字保持高对比。link 形态（传 href 时渲染为链接）与按钮形态都要，键盘 focus-visible 有描边。先看现有按钮体系，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import ThreeDButton from \"./ThreeDButton\";\nexport default function Demo() { return <ThreeDButton text=\"Book a call\" />; }",
  },
};
