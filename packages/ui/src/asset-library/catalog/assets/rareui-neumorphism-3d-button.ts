import type { AssetManifest } from "../types.js";

/**
 * 「新拟态 3D 按钮」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/Neumorphism3DButton/Neumorphism3DButton.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiNeumorphism3DButtonAsset: AssetManifest = {
  id: "rareui-neumorphism-3d-button",
  title: "新拟态 3D 按钮",
  titleEn: "Neumorphism 3D Button",
  description: "按下时凹陷回弹的新拟态圆钮，尺寸随字号。",
  category: "control",
  tags: ["rareui","按钮","新拟态","3d","按下"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "Neumorphism3DButton.tsx",
      "language": "tsx",
      "content": "/*!\n * 新拟态 3D 按钮 · Neumorphism3DButton.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/Neumorphism3DButton/Neumorphism3DButton.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/Neumorphism3DButton/Neumorphism3DButton.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n'use client';\r\n\r\nimport React from 'react';\r\nimport { cn } from '@/lib/utils';\r\n\r\ninterface Neumorphism3DButtonProps {\r\n  children?: React.ReactNode;\r\n  onClick?: () => void;\r\n  disabled?: boolean;\r\n}\r\n\r\nexport const Neumorphism3DButton = ({\r\n  children = 'Click Me',\r\n  onClick,\r\n  disabled,\r\n}: Neumorphism3DButtonProps) => {\r\n  const [isActive, setIsActive] = React.useState(false);\r\n\r\n  return (\r\n    <button\r\n      onClick={onClick}\r\n      disabled={disabled}\r\n      onMouseDown={() => setIsActive(true)}\r\n      onMouseUp={() => setIsActive(false)}\r\n      onMouseLeave={() => setIsActive(false)}\r\n      className={cn(\r\n        '-webkit-tap-highlight-color-transparent relative cursor-pointer rounded-full transition-all duration-300 outline-none',\r\n        'bg-neutral-200 shadow-[-0.15em_-0.15em_0.15em_-0.075em_rgba(255,255,255,1),0.0375em_0.0375em_0.0675em_0_rgba(0,0,0,0.15)]',\r\n        'dark:bg-neutral-900 dark:shadow-[-0.15em_-0.15em_0.15em_-0.075em_rgba(255,255,255,0.05),0.0375em_0.0375em_0.0675em_0_rgba(0,0,0,0.5)]'\r\n      )}\r\n    >\r\n      <div\r\n        className={cn(\r\n          'pointer-events-none absolute inset-0 rounded-full',\r\n          'top-[-0.15em] left-[-0.15em] h-[calc(100%+0.3em)] w-[calc(100%+0.3em)]',\r\n          'opacity-25 mix-blend-multiply blur-[0.0125em]',\r\n          'bg-[linear-gradient(-135deg,rgba(0,0,0,0.2),transparent_20%,transparent_100%)]',\r\n          'dark:bg-[linear-gradient(-135deg,rgba(255,255,255,0.1),transparent_20%,transparent_100%)] dark:mix-blend-overlay'\r\n        )}\r\n      />\r\n\r\n      <div\r\n        className={cn(\r\n          'relative rounded-full transition-all duration-300',\r\n          isActive\r\n            ? 'shadow-[0_0_0_0_rgba(0,0,0,0.2)] dark:shadow-[0_0_0_0_rgba(0,0,0,0.5)]'\r\n            : 'shadow-[0_0.05em_0.05em_-0.01em_rgba(0,0,0,0.2),0_0.01em_0.01em_-0.01em_rgba(0,0,0,0.1),0.15em_0.3em_0.1em_-0.01em_rgba(0,0,0,0.05)] dark:shadow-[0_0.05em_0.05em_-0.01em_rgba(0,0,0,0.5),0_0.01em_0.01em_-0.01em_rgba(0,0,0,0.3),0.15em_0.3em_0.1em_-0.01em_rgba(0,0,0,0.2)]'\r\n        )}\r\n      >\r\n        <div\r\n          className={cn(\r\n            'relative overflow-hidden rounded-full px-6 py-4 transition-all duration-250',\r\n            'bg-[linear-gradient(135deg,rgba(240,240,240,1),rgba(210,210,210,1))]',\r\n            'dark:bg-[linear-gradient(135deg,rgba(40,40,40,1),rgba(20,20,20,1))]',\r\n            isActive ? 'scale-[0.975]' : 'scale-100',\r\n            isActive\r\n              ? 'shadow-[inset_0.1em_0.15em_0.05em_0_rgba(0,0,0,0.1),inset_-0.025em_-0.03em_0.05em_0.025em_rgba(255,255,255,0.5)] dark:shadow-[inset_0.1em_0.15em_0.05em_0_rgba(0,0,0,0.5),inset_-0.025em_-0.03em_0.05em_0.025em_rgba(255,255,255,0.1)]'\r\n              : 'shadow-[inset_-0.05em_-0.05em_0.05em_0_rgba(0,0,0,0.1),inset_0.025em_0.05em_0.1em_0_rgba(255,255,255,1)] dark:shadow-[inset_-0.05em_-0.05em_0.05em_0_rgba(0,0,0,0.5),inset_0.025em_0.05em_0.1em_0_rgba(255,255,255,0.1)]'\r\n          )}\r\n        >\r\n          <span\r\n            className={cn(\r\n              'relative block text-sm font-medium tracking-tight transition-transform duration-250 select-none',\r\n              isActive ? 'scale-[0.975]' : 'scale-100',\r\n              'bg-clip-text text-transparent',\r\n              'bg-linear-to-br from-neutral-800 to-neutral-600',\r\n              'dark:from-neutral-100 dark:to-neutral-300',\r\n              'drop-shadow-sm'\r\n            )}\r\n          >\r\n            {children}\r\n          </span>\r\n        </div>\r\n      </div>\r\n    </button>\r\n  );\r\n};\r\n"
    }
  ],
  prompt: "请把「新拟态 3D 按钮」装进我的项目：一颗圆形 neumorphism 按钮——静息时靠 -0.15em/-0.15em 白高光与右下黑影浮起；按下（mousedown/touch）时阴影反向/收小，视觉上一按就凹进去，松开弹回；尺寸全部用 em 因而跟随字体大小（放进不同容器自动适配）；深色模式一套对应阴影。带 disabled 态与 focus-visible 描边，键盘 Enter/Space 同样触发按下反馈。先看现有按钮体系，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import { Neumorphism3DButton } from \"./Neumorphism3DButton\";\nexport default function Demo() { return <Neumorphism3DButton>Click Me</Neumorphism3DButton>; }",
  },
};
