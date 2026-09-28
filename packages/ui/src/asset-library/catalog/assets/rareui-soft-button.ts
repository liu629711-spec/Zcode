import type { AssetManifest } from "../types.js";

/**
 * 「新拟态软按钮」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/SoftButton/SoftButton.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiSoftButtonAsset: AssetManifest = {
  id: "rareui-soft-button",
  title: "新拟态软按钮",
  titleEn: "Soft Button",
  description: "新拟态（neumorphism）按钮：外凸阴影，悬停变实底并拉字距。",
  category: "control",
  tags: ["rareui","按钮","新拟态","阴影","悬停"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "SoftButton.tsx",
      "language": "tsx",
      "content": "/*!\n * 新拟态软按钮 · SoftButton.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/SoftButton/SoftButton.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/SoftButton/SoftButton.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\nimport React from 'react';\r\nimport { clsx, type ClassValue } from 'clsx';\r\nimport { twMerge } from 'tailwind-merge';\r\n\r\nfunction cn(...inputs: ClassValue[]) {\r\n  return twMerge(clsx(inputs));\r\n}\r\n\r\ninterface SoftButtonProps extends React.HTMLAttributes<HTMLDivElement> {\r\n  children?: React.ReactNode;\r\n}\r\n\r\nconst SoftButton = ({ children = 'Button', className, ...props }: SoftButtonProps) => {\r\n  return (\r\n    // Outer Wrapper\r\n    <div\r\n      className={cn(\r\n        'group relative top-0 left-0 m-0 flex h-[50px] w-[160px] cursor-pointer items-center justify-center',\r\n        className\r\n      )}\r\n      {...props}\r\n    >\r\n      {/* Inner Div */}\r\n      <div className=\"/* Base Styles */ text-foreground /* Borders */ /* SHADOW EXPLANATION: We use arbitrary values because Neumorphism requires very specific multi-layer shadows. Light Mode Shadow: 1. 4px 4px... White (Bottom-Right Highlight) 2. -4px -4px... Gray (Top-Left Shadow) 3. Inset -4px -4px... White (Inner Top-Left Highlight) 4. Inset 4px 4px... Black (Inner Bottom-Right Shadow) Dark Mode Shadow (dark: prefix): 1. 4px 4px... Faint White (Bottom-Right Highlight, opacity reduced to 0.05) 2. -4px -4px... Deep Black (Top-Left Shadow, opacity 0.5) 3. Inset -4px -4px... Faint White (Inner Top-Left Highlight) 4. Inset 4px 4px... Deep Black (Inner Bottom-Right Shadow) */ /* Transitions */ /* Hover States */ /* Active State */ /* Enhanced Hover Shadow */ /* Dark Mode Hover Adjustment */ z-10 flex h-full w-full items-center justify-center rounded-[30px] border-t border-b border-white/10 bg-transparent font-medium tracking-[1px] shadow-[4px_4px_6px_0_rgba(255,255,255,0.5),-4px_-4px_6px_0_rgba(116,125,136,0.5),inset_-4px_-4px_6px_0_rgba(255,255,255,0.2),inset_4px_4px_6px_0_rgba(0,0,0,0.4)] transition-all duration-[600ms] group-hover:scale-[1.05] group-hover:bg-black group-hover:tracking-[2px] group-hover:text-white group-hover:shadow-[0_0_20px_hsl(var(--primary)/0.6),4px_4px_8px_0_rgba(255,255,255,0.3),-4px_-4px_8px_0_rgba(116,125,136,0.3)] group-active:scale-[0.98] dark:border-black/20 dark:shadow-[4px_4px_6px_0_rgba(255,255,255,0.05),-4px_-4px_6px_0_rgba(0,0,0,0.5),inset_-4px_-4px_6px_0_rgba(255,255,255,0.05),inset_4px_4px_6px_0_rgba(0,0,0,0.6)] dark:group-hover:shadow-[0_0_20px_hsl(var(--primary)/0.6),4px_4px_8px_0_rgba(255,255,255,0.08),-4px_-4px_8px_0_rgba(0,0,0,0.6)] dark:hover:bg-white dark:hover:text-black\">\r\n        {children}\r\n      </div>\r\n    </div>\r\n  );\r\n};\r\n\r\nexport default SoftButton;\r\n"
    }
  ],
  prompt: "请把「新拟态软按钮」装进我的项目：一颗 neumorphism 按钮——多层阴影叠出外凸立体（右下白高光 + 左上灰影 + 两道内阴影），整体随 600ms 缓动；悬停时背景变实黑、文字变白、字距从 1px 拉到 2px 并放大 5%，阴影换成同色发光；按下缩到 0.98。深色模式一套对应阴影（白高光降到 0.05，黑影加重）。先看现有按钮体系，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import SoftButton from \"./SoftButton\";\nexport default function Demo() { return <SoftButton>Press me</SoftButton>; }",
  },
};
