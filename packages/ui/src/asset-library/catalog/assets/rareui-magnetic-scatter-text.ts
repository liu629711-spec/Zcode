import type { AssetManifest } from "../types.js";

/**
 * 「磁性散字」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/MagneticScatterText/MagneticScatterText.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiMagneticScatterTextAsset: AssetManifest = {
  id: "rareui-magnetic-scatter-text",
  title: "磁性散字",
  titleEn: "Magnetic Scatter Text",
  description: "悬停时字母被磁力吸引聚拢的标题文字。",
  category: "text-animation",
  tags: ["rareui","文字","磁力","framer-motion","标题"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "MagneticScatterText.tsx",
      "language": "tsx",
      "content": "/*!\n * 磁性散字 · MagneticScatterText.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/MagneticScatterText/MagneticScatterText.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/MagneticScatterText/MagneticScatterText.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n'use client';\r\n\r\nimport React, { useState, useEffect } from 'react';\r\nimport { motion, useAnimation } from 'framer-motion';\r\nimport { cn } from '@/lib/utils';\r\n\r\ninterface MagneticScatterTextProps {\r\n  text: string;\r\n  className?: string;\r\n  trigger?: boolean;\r\n}\r\n\r\nexport const MagneticScatterText: React.FC<MagneticScatterTextProps> = ({\r\n  text,\r\n  className,\r\n  trigger = true,\r\n}) => {\r\n  const controls = useAnimation();\r\n  const [isHovering, setIsHovering] = useState(false);\r\n\r\n  useEffect(() => {\r\n    if (trigger) {\r\n      controls.start('visible');\r\n    } else {\r\n      controls.start('hidden');\r\n    }\r\n  }, [trigger, controls]);\r\n\r\n  useEffect(() => {\r\n    if (isHovering) {\r\n      controls.start('scatter');\r\n    } else {\r\n      controls.start('visible');\r\n    }\r\n  }, [isHovering, controls]);\r\n\r\n  const letters = text.split('');\r\n\r\n  // Deterministic random for consistent scattering based on index\r\n  const getRandom = (index: number) => {\r\n    const seed = index * 42;\r\n    const x = Math.sin(seed) * 100; // -100 to 100\r\n    const y = Math.cos(seed) * 100; // -100 to 100\r\n    const rotate = Math.sin(seed * 2) * 180; // -180 to 180\r\n    return { x, y, rotate };\r\n  };\r\n\r\n  return (\r\n    <motion.div\r\n      className={cn('flex cursor-default select-none', className)}\r\n      onMouseEnter={() => setIsHovering(true)}\r\n      onMouseLeave={() => setIsHovering(false)}\r\n    >\r\n      {letters.map((letter, index) => {\r\n        const randoms = getRandom(index);\r\n        return (\r\n          <motion.span\r\n            key={index}\r\n            className=\"inline-block font-sans font-bold text-black dark:text-white\"\r\n            initial=\"hidden\"\r\n            animate={controls}\r\n            variants={{\r\n              hidden: {\r\n                x: randoms.x * 5,\r\n                y: randoms.y * 5,\r\n                rotate: randoms.rotate,\r\n                opacity: 0,\r\n                scale: 0.5,\r\n              },\r\n              visible: {\r\n                x: 0,\r\n                y: 0,\r\n                rotate: 0,\r\n                opacity: 1,\r\n                scale: 1,\r\n                transition: {\r\n                  type: 'spring',\r\n                  damping: 12,\r\n                  stiffness: 80,\r\n                  mass: 0.8,\r\n                  delay: index * 0.02,\r\n                },\r\n              },\r\n              scatter: {\r\n                x: randoms.x * 0.5, // Slight scatter on hover\r\n                y: randoms.y * 0.5,\r\n                rotate: randoms.rotate * 0.2,\r\n                scale: 1.1,\r\n                transition: {\r\n                  type: 'spring',\r\n                  damping: 15,\r\n                  stiffness: 200,\r\n                },\r\n              },\r\n            }}\r\n          >\r\n            {letter === ' ' ? '\\u00A0' : letter}\r\n          </motion.span>\r\n        );\r\n      })}\r\n    </motion.div>\r\n  );\r\n};\r\n"
    }
  ],
  prompt: "请把「磁性散字」装进我的项目：一段标题文字——默认状态每个字母各自小幅散开（随机偏移与旋转），鼠标进入文字区域时所有字母以磁力吸引的观感向鼠标位置聚拢并轻微放大，移开后弹回散开态；用 framer-motion 的动画序列驱动（visible/hidden 两套 variant，错峰 stagger）；键盘聚焦（tabindex + focus）同样触发聚拢，保证非鼠标可用。先看现有的标题/文字动效组件，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import { MagneticScatterText } from \"./MagneticScatterText\";\nexport default function Demo() { return <MagneticScatterText text=\"MAGNETIC\" />; }",
  },
};
