import type { AssetManifest } from "../types.js";

/**
 * 「玻璃流光按钮」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/GlassShimmerButton/GlassShimmerButton.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiGlassShimmerButtonAsset: AssetManifest = {
  id: "rareui-glass-shimmer-button",
  title: "玻璃流光按钮",
  titleEn: "Glass Shimmer Button",
  description: "毛玻璃胶囊按钮，一道高光周期性斜切扫过。",
  category: "control",
  tags: ["rareui","按钮","毛玻璃","流光","css"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "GlassShimmerButton.tsx",
      "language": "tsx",
      "content": "/*!\n * 玻璃流光按钮 · GlassShimmerButton.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/GlassShimmerButton/GlassShimmerButton.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/GlassShimmerButton/GlassShimmerButton.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n'use client';\r\n\r\nimport type React from 'react';\r\nimport { cn } from '@/lib/utils';\r\n\r\ninterface GlassShimmerButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {\r\n  children: React.ReactNode;\r\n}\r\n\r\nexport function GlassShimmerButton({ children, className, ...props }: GlassShimmerButtonProps) {\r\n  return (\r\n    <>\r\n      <style jsx>{`\r\n        @keyframes glass-shimmer {\r\n          0% {\r\n            transform: translateX(-150%) skewX(-20deg);\r\n          }\r\n          100% {\r\n            transform: translateX(150%) skewX(-20deg);\r\n          }\r\n        }\r\n      `}</style>\r\n      <button\r\n        className={cn(\r\n          'group relative inline-flex items-center justify-center overflow-hidden rounded-full',\r\n          'border border-white/10 px-8 py-3',\r\n          'text-base font-medium text-white transition-all duration-300 dark:text-black',\r\n          'hover:scale-105 active:scale-95',\r\n          'shadow-[0_0_20px_rgba(0,0,0,0.1)] backdrop-blur-md hover:shadow-[0_0_0_0.4rgba(255,255,255,0.2)]',\r\n          'bg-neutral-800 hover:bg-neutral-950 dark:bg-neutral-100 dark:hover:bg-white',\r\n          className\r\n        )}\r\n        {...props}\r\n      >\r\n        {/* Shimmer Animation Element */}\r\n        <div\r\n          className=\"pointer-events-none absolute inset-0 -z-10 h-full w-full\"\r\n          style={{\r\n            background:\r\n              'linear-gradient(to right, transparent, rgba(255, 255, 255, 1) 50%, transparent)',\r\n            animation: 'glass-shimmer 2.5s infinite linear',\r\n            width: '200%',\r\n            left: '-50%',\r\n          }}\r\n        />\r\n\r\n        {/* Static Glass Gloss Top */}\r\n        <div className=\"absolute inset-x-0 top-0 h-px bg-linear-to-r from-transparent via-white/40 to-transparent opacity-50\" />\r\n\r\n        {/* Static Glass Gloss Bottom */}\r\n        <div className=\"absolute inset-x-0 bottom-0 h-px bg-linear-to-r from-transparent via-white/10 to-transparent opacity-30\" />\r\n\r\n        {/* Content */}\r\n        <span className=\"relative z-10 tracking-wide drop-shadow-sm\">{children}</span>\r\n\r\n        {/* Local Glow on Hover */}\r\n        <div className=\"absolute inset-0 -z-10 bg-white/5 opacity-0 transition-opacity duration-300 group-hover:opacity-100\" />\r\n      </button>\r\n    </>\r\n  );\r\n}\r\n"
    }
  ],
  prompt: "请把「玻璃流光按钮」装进我的项目：一颗毛玻璃胶囊按钮——半透明深底 + backdrop-blur + 内描边，顶部一条 1px 白色渐变高光模拟玻璃棱；一道斜切（skewX）的白色高光带以 2.5s 周期从右向左扫过（宽度 200%、left 从 -50% 推），悬停时按钮轻微放大、阴影加重，按下缩回；深色模式反色（浅底深字）。要保留键盘 focus-visible 描边与 disabled 态。先看现有的按钮体系，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import { GlassShimmerButton } from \"./GlassShimmerButton\";\nexport default function Demo() { return <GlassShimmerButton>Get Started</GlassShimmerButton>; }",
  },
};
