import type { AssetManifest } from "../types.js";

/**
 * 「复古像素按钮」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/RetroPixelButton/RetroPixelButton.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiRetroPixelButtonAsset: AssetManifest = {
  id: "rareui-retro-pixel-button",
  title: "复古像素按钮",
  titleEn: "Retro Pixel Button",
  description: "悬停时像素方块从底沿滚过的等宽字体按钮。",
  category: "control",
  tags: ["rareui","按钮","像素","复古","等宽"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "RetroPixelButton.tsx",
      "language": "tsx",
      "content": "/*!\n * 复古像素按钮 · RetroPixelButton.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/RetroPixelButton/RetroPixelButton.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/RetroPixelButton/RetroPixelButton.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n'use client';\r\n\r\nimport React, { useState } from 'react';\r\nimport { motion } from 'framer-motion';\r\n\r\ninterface RetroPixelButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {\r\n  children?: React.ReactNode;\r\n  className?: string;\r\n  pixelColor?: string; // The color of the moving pixel block\r\n  baseColor?: string; // The background of the button\r\n  textColor?: string;\r\n}\r\n\r\nexport default function RetroPixelButton({\r\n  children = 'TRY FOR FREE',\r\n  className,\r\n  pixelColor = 'orange', // The color of the moving pixel block (can be a tailwind class like 'bg-orange-500')\r\n  baseColor, // Optional override, otherwise uses theme colors\r\n  textColor, // Optional override\r\n  ...props\r\n}: RetroPixelButtonProps) {\r\n  const [isHovered, setIsHovered] = useState(false);\r\n\r\n  return (\r\n    <motion.button\r\n      className=\"group bg-background relative flex h-16 cursor-pointer items-center overflow-hidden rounded-lg border border-orange-300 px-8 font-mono font-medium shadow-sm transition-colors hover:shadow-md dark:border-orange-300 dark:bg-neutral-900\"\r\n      whileTap={{ scale: 0.98 }}\r\n      style={{\r\n        backgroundColor: baseColor,\r\n        color: textColor,\r\n      }}\r\n      onMouseEnter={() => setIsHovered(true)}\r\n      onMouseLeave={() => setIsHovered(false)}\r\n      {...(props as any)}\r\n    >\r\n      {/* Sliding Icon Block */}\r\n      <motion.div\r\n        className=\"absolute top-1 bottom-1 left-1 z-10 flex items-center justify-center overflow-hidden rounded\"\r\n        style={{ backgroundColor: pixelColor }}\r\n        animate={{\r\n          width: isHovered ? '97%' : '55px',\r\n        }}\r\n        transition={{\r\n          type: 'spring',\r\n          stiffness: 400,\r\n          damping: 30,\r\n        }}\r\n      >\r\n        {/* Inner container for the icon */}\r\n        <motion.div className=\"absolute right-0 flex h-full w-16 items-center justify-center\">\r\n          <motion.svg\r\n            animate={{ rotate: isHovered ? 180 : 0 }}\r\n            transition={{ type: 'spring', stiffness: 300, damping: 20 }}\r\n            width=\"40\"\r\n            height=\"40\"\r\n            viewBox=\"0 0 16 34\"\r\n            fill=\"currentColor\" // Uses current text color (which is forced to white inside this block)\r\n            xmlns=\"http://www.w3.org/2000/svg\"\r\n            className=\"text-white\"\r\n          >\r\n            <rect x=\"7\" y=\"6\" width=\"5\" height=\"5\" />\r\n            <rect x=\"11\" y=\"10\" width=\"5\" height=\"5\" />\r\n            <rect x=\"15\" y=\"14\" width=\"5\" height=\"5\" />\r\n            <rect x=\"11\" y=\"18\" width=\"5\" height=\"5\" />\r\n            <rect x=\"7\" y=\"22\" width=\"5\" height=\"5\" />\r\n          </motion.svg>\r\n        </motion.div>\r\n      </motion.div>\r\n\r\n      {/* Text Label */}\r\n      <motion.div\r\n        className=\"relative z-20 w-full text-center whitespace-nowrap\"\r\n        initial={{ paddingLeft: 64, paddingRight: 0, color: 'currentColor' }}\r\n        animate={{\r\n          paddingLeft: isHovered ? 0 : 64,\r\n          paddingRight: isHovered ? 64 : 0,\r\n          color: isHovered ? '#ffffff' : 'currentColor',\r\n          opacity: isHovered ? [1, 0.5, 1] : [1, 0.5, 1], // Subtle blink/morph effect during transit\r\n          filter: isHovered\r\n            ? ['blur(0px)', 'blur(2px)', 'blur(0px)']\r\n            : ['blur(0px)', 'blur(2px)', 'blur(0px)'],\r\n        }}\r\n        transition={{\r\n          type: 'spring',\r\n          stiffness: 200,\r\n          damping: 25,\r\n          mass: 1,\r\n          paddingLeft: { duration: 0.4 },\r\n          paddingRight: { duration: 0.4 },\r\n          color: { duration: 0.2 },\r\n          opacity: { duration: 0.3, times: [0, 0.5, 1] },\r\n          filter: { duration: 0.3, times: [0, 0.5, 1] },\r\n        }}\r\n        style={{ color: isHovered ? '#ffffff' : undefined }}\r\n      >\r\n        {children}\r\n      </motion.div>\r\n    </motion.button>\r\n  );\r\n}\r\n"
    }
  ],
  prompt: "请把「复古像素按钮」装进我的项目：一颗复古像素风按钮——等宽字体全大写文字，浅底细描边圆角矩形；悬停时一小块方形像素（可配颜色）从左侧沿底沿滚到右侧，同时文字轻微位移（像被像素块推着走）；按下缩到 0.98。像素块颜色/底色/文字色都可通过 props 覆盖。先看现有按钮体系，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import RetroPixelButton from \"./RetroPixelButton\";\nexport default function Demo() { return <RetroPixelButton />; }",
  },
};
