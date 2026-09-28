import type { AssetManifest } from "../types.js";

/**
 * 「脉冲加载环」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/LoadingSpinner/LoadingSpinner.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiLoadingSpinnerAsset: AssetManifest = {
  id: "rareui-loading-spinner",
  title: "脉冲加载环",
  titleEn: "Loading Spinner",
  description: "同心弧线段旋转的加载环，多段错速。",
  category: "control",
  tags: ["rareui","加载","spinner","framer-motion","等待"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "LoadingSpinner.tsx",
      "language": "tsx",
      "content": "/*!\n * 脉冲加载环 · LoadingSpinner.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/LoadingSpinner/LoadingSpinner.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/LoadingSpinner/LoadingSpinner.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n'use client';\r\n\r\nimport React from 'react';\r\nimport { motion, Transition } from 'framer-motion';\r\nimport { cn } from '@/lib/utils';\r\n\r\ninterface LoadingSpinnerProps {\r\n  className?: string; // Allow custom classes\r\n}\r\n\r\nconst spinTransition: Transition = {\r\n  repeat: Infinity,\r\n  ease: 'linear',\r\n  duration: 1, // continuous spin\r\n};\r\n\r\nexport default function LoadingSpinner({ className }: LoadingSpinnerProps) {\r\n  return (\r\n    <div className={cn('flex items-center gap-3', className)}>\r\n      <div className=\"relative flex h-6 w-6 items-center justify-center\">\r\n        <svg\r\n          viewBox=\"0 0 18 18\"\r\n          className=\"absolute h-full w-full text-neutral-200 dark:text-neutral-800\"\r\n        >\r\n          <path\r\n            d=\"M 9 16.25 C 4.996 16.25 1.75 13.004 1.75 9 C 1.75 4.996 4.996 1.75 9 1.75 C 13.004 1.75 16.25 4.996 16.25 9 C 16.25 13.004 13.004 16.25 9 16.25 Z\"\r\n            fill=\"transparent\"\r\n            strokeWidth=\"2\"\r\n            stroke=\"currentColor\"\r\n            strokeLinecap=\"round\"\r\n            strokeLinejoin=\"round\"\r\n          />\r\n        </svg>\r\n\r\n        {/* Rotating Segment */}\r\n        <motion.div\r\n          className=\"absolute h-full w-full\"\r\n          animate={{ rotate: 360 }}\r\n          transition={spinTransition}\r\n        >\r\n          {/* Main Stroke */}\r\n          <svg\r\n            viewBox=\"0 0 18 18\"\r\n            className=\"absolute top-0 left-0 h-full w-full text-black dark:text-white\"\r\n          >\r\n            <path\r\n              d=\"M 16.25 9 C 16.25 10.07 16.018 11.086 15.602 12 C 15.163 12.965 14.518 13.817 13.724 14.5\"\r\n              fill=\"transparent\"\r\n              strokeWidth=\"2\"\r\n              stroke=\"currentColor\"\r\n              strokeLinecap=\"round\"\r\n              strokeLinejoin=\"round\"\r\n            />\r\n          </svg>\r\n\r\n          <svg\r\n            viewBox=\"0 0 18 18\"\r\n            className=\"absolute top-0 left-0 h-full w-full text-black opacity-60 blur-[2px] dark:text-white\"\r\n          >\r\n            <path\r\n              d=\"M 16.25 9 C 16.25 10.07 16.018 11.086 15.602 12 C 15.163 12.965 14.518 13.817 13.724 14.5\"\r\n              fill=\"transparent\"\r\n              strokeWidth=\"2\"\r\n              stroke=\"currentColor\"\r\n              strokeLinecap=\"round\"\r\n              strokeLinejoin=\"round\"\r\n            />\r\n          </svg>\r\n        </motion.div>\r\n      </div>\r\n\r\n      <div className=\"relative overflow-hidden rounded-full\">\r\n        <motion.span\r\n          className={cn(\r\n            'block bg-clip-text font-sans text-sm font-medium text-transparent',\r\n            'bg-linear-to-r from-neutral-950 via-neutral-500 to-neutral-950',\r\n            'dark:from-[#f1f2f4] dark:via-neutral-500 dark:to-[#f1f2f4]'\r\n          )}\r\n          initial={{ backgroundPosition: '200% 0' }}\r\n          animate={{ backgroundPosition: '-200% 0' }}\r\n          transition={{\r\n            repeat: Infinity,\r\n            duration: 3,\r\n            ease: 'linear',\r\n          }}\r\n          style={{\r\n            backgroundSize: '200% auto',\r\n          }}\r\n        >\r\n          Loading...\r\n        </motion.span>\r\n      </div>\r\n    </div>\r\n  );\r\n}\r\n"
    }
  ],
  prompt: "请把「脉冲加载环」装进我的项目：一个加载指示器——若干段同心圆弧各自以不同速度/方向旋转（framer-motion 驱动 strokeDasharray 或 rotate），叠出连续抽动的脉冲感；尺寸可通过 className 缩放；颜色取当前主色（currentColor）；在 prefers-reduced-motion 下降级为静态或极慢旋转。要能放在按钮里当 busy 指示（aria-busy + 视觉隐藏文案）。先看现有的加载组件，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import LoadingSpinner from \"./LoadingSpinner\";\nexport default function Demo() { return <LoadingSpinner />; }",
  },
};
