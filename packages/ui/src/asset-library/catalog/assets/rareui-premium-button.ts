import type { AssetManifest } from "../types.js";

/**
 * 「高级渐变按钮」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/PremiumButton/PremiumButton.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiPremiumButtonAsset: AssetManifest = {
  id: "rareui-premium-button",
  title: "高级渐变按钮",
  titleEn: "Premium Button",
  description: "柠檬绿多段渐变按钮，悬停时渐变流动+箭头内推。",
  category: "control",
  tags: ["rareui","按钮","渐变","css","悬停"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "PremiumButton.tsx",
      "language": "tsx",
      "content": "/*!\n * 高级渐变按钮 · PremiumButton.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/PremiumButton/PremiumButton.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/PremiumButton/PremiumButton.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n'use client';\r\n\r\nimport React from 'react';\r\n\r\nexport const PremiumButton = () => {\r\n  return (\r\n    <button className=\"group /* Gradient Background */ /* Shadows */ /* Transitions */ /* Hover States */ flex w-fit cursor-pointer items-center gap-[0.4rem] rounded-[30px] border-none bg-[linear-gradient(15deg,#ddff00,#b8d100,#93a300,#6e7500,#ddff00,#b8d100,#93a300,#6e7500)] bg-size-[300%] bg-left px-10 py-[1.2em] font-bold text-black shadow-[0_30px_10px_-20px_rgba(221,255,0,0.2)] transition-[background,color] duration-300 ease-out [text-shadow:2px_2px_3px_rgba(221,255,0,0.3)] hover:bg-size-[320%] hover:bg-right\">\r\n      <svg\r\n        viewBox=\"0 0 36 24\"\r\n        xmlns=\"http://www.w3.org/2000/svg\"\r\n        className=\"w-[23px] fill-black transition-all duration-300 ease-out group-hover:fill-black\"\r\n      >\r\n        <path d=\"m18 0 8 12 10-8-4 20H4L0 4l10 8 8-12z\"></path>\r\n      </svg>\r\n      Premium\r\n    </button>\r\n  );\r\n};\r\n\r\nexport default PremiumButton;\r\n"
    }
  ],
  prompt: "请把「高级渐变按钮」装进我的项目：一颗柠檬绿渐变按钮——背景是多段重复的 linear-gradient（300% 尺寸）停在左侧，悬停时 background-position 移到右侧，形成「渐变往里流」的观感；文字带同色系 text-shadow 做发光；左侧是一枚自绘 SVG 图标，悬停时图标位移/放大；整颗按钮阴影是同色低透明度扩散。先看现有的主按钮体系，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import PremiumButton from \"./PremiumButton\";\nexport default function Demo() { return <PremiumButton />; }",
  },
};
