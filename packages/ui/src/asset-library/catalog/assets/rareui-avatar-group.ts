import type { AssetManifest } from "../types.js";

/**
 * 「头像堆叠」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/AvatarGroup/AvatarGroup.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiAvatarGroupAsset: AssetManifest = {
  id: "rareui-avatar-group",
  title: "头像堆叠",
  titleEn: "Avatar Group",
  description: "一排头像依次跳起并放大，选中者高亮前移。",
  category: "block",
  tags: ["rareui","头像","动效","framer-motion","团队"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "AvatarGroup.tsx",
      "language": "tsx",
      "content": "/*!\n * 头像堆叠 · AvatarGroup.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/AvatarGroup/AvatarGroup.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/AvatarGroup/AvatarGroup.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n\"use client\";\r\n\r\nimport React, { useState, useEffect } from \"react\";\r\nimport { motion, AnimatePresence } from \"framer-motion\";\r\n\r\nconst avatars = [\r\n  {\r\n    src: \"https://api.dicebear.com/9.x/notionists/svg?seed=Robert\",\r\n    alt: \"Robert\",\r\n  },\r\n  {\r\n    src: \"https://api.dicebear.com/9.x/notionists/svg?seed=Sophia\",\r\n    alt: \"Sophia\",\r\n  },\r\n  {\r\n    src: \"https://api.dicebear.com/9.x/notionists/svg?seed=Liliana\",\r\n    alt: \"Liliana\",\r\n  },\r\n  {\r\n    src: \"https://api.dicebear.com/9.x/notionists/svg?seed=Brian\",\r\n    alt: \"Brian\",\r\n  },\r\n];\r\n\r\nexport default function AvatarGroup() {\r\n  const [activeIndex, setActiveIndex] = useState(0);\r\n\r\n  useEffect(() => {\r\n    const interval = setInterval(() => {\r\n      setActiveIndex((prev) => (prev + 1) % avatars.length);\r\n    }, 2000);\r\n    return () => clearInterval(interval);\r\n  }, []);\r\n\r\n  return (\r\n    <div className=\"flex justify-center items-start  gap-1\">\r\n      {/* Avatar Row */}\r\n      <div className=\"flex items-end h-[54px]\">\r\n        {\" \"}\r\n        {/* Fixed height for jumping room */}\r\n        <div className=\"flex -space-x-4\">\r\n          {avatars.map((avatar, index) => {\r\n            const isActive = index === activeIndex;\r\n            return (\r\n              <div key={avatar.alt} className=\"relative group\">\r\n                {/* Floating Tooltip */}\r\n                <AnimatePresence mode=\"wait\">\r\n                  {isActive && (\r\n                    <motion.div\r\n                      initial={{ opacity: 0, y: 10, scale: 0.8 }}\r\n                      animate={{ opacity: 1, y: 0, scale: 1 }}\r\n                      exit={{ opacity: 0, y: 5, scale: 0.9 }}\r\n                      className=\"absolute -top-9 left-1/2 -translate-x-1/2 bg-neutral-900 text-white text-[10px] font-bold px-2 py-1 rounded-md shadow-xl whitespace-nowrap z-50 flex items-center gap-1\"\r\n                    >\r\n                      {avatar.alt}\r\n                      <div className=\"absolute -bottom-1 left-1/2 -translate-x-1/2 w-2 h-2 bg-neutral-900 rotate-45\" />{\" \"}\r\n                      {/* Arrow */}\r\n                    </motion.div>\r\n                  )}\r\n                </AnimatePresence>\r\n\r\n                {/* Avatar Circle */}\r\n                <motion.div\r\n                  className={`relative h-10 w-10  rounded-full border-2 border-white shadow-sm bg-white overflow-hidden z-20`}\r\n                  animate={{\r\n                    y: isActive ? -8 : 0,\r\n                    scale: isActive ? 1.1 : 1,\r\n                    zIndex: isActive ? 50 : avatars.length - index,\r\n                  }}\r\n                  transition={{\r\n                    type: \"spring\",\r\n                    stiffness: 300,\r\n                    damping: 20,\r\n                  }}\r\n                >\r\n                  <img\r\n                    src={avatar.src}\r\n                    alt={avatar.alt}\r\n                    className=\"h-full w-full object-cover\"\r\n                  />\r\n                </motion.div>\r\n\r\n                {/* Shadow underneath jumping avatar */}\r\n                {isActive && (\r\n                  <motion.div\r\n                    initial={{ opacity: 0, scale: 0 }}\r\n                    animate={{ opacity: 0.2, scale: 1 }}\r\n                    exit={{ opacity: 0 }}\r\n                    className=\"absolute bottom-0 left-2 right-2 h-1 bg-black rounded-full blur-[2px] z-10\"\r\n                  />\r\n                )}\r\n              </div>\r\n            );\r\n          })}\r\n          {/* Add button placeholder */}\r\n         \r\n        </div>\r\n      </div>\r\n\r\n      {/* Static Label */}\r\n      <div className=\"pl-3 flex flex-col items-start justify-center pt-4\">\r\n        {/* <div className=\"flex gap-0.5\">\r\n          {[1, 2, 3, 4, 5].map((i) => (\r\n            <svg\r\n              key={i}\r\n              className=\"w-3 h-3 text-orange-300 fill-current\"\r\n              viewBox=\"0 0 24 24\"\r\n            >\r\n              <path d=\"M12 17.27L18.18 21l-1.64-7.03L22 9.24l-7.19-.61L12 2 9.19 8.63 2 9.24l5.46 4.73L5.82 21z\" />\r\n            </svg>\r\n          ))}\r\n        </div> */}\r\n        <span className=\"text-[12px] font-semibold text-neutral-200\">\r\n          2k+ Users\r\n        </span>\r\n        <span className=\"text-[10px] font-semibold text-neutral-400 uppercase tracking-wide\">\r\n          &hearts; by many developers.\r\n        </span>\r\n      </div>\r\n    </div>\r\n  );\r\n}\r\n"
    }
  ],
  prompt: "请把「头像堆叠」装进我的项目：一排头像的动效展示——若干圆头像横向相邻（重叠一点），每隔 2s 轮换一位：被选中的头像上跳并放大（spring 回弹，底部对齐因此有跳跃空间）、外圈加一圈高亮描边，未选中的缩回原尺寸；头像用对象数组配置（src/alt，可换成我们自己的图片或首字母兜底）；视觉上是无框的干净横排，可加在团队列表/在线成员处。先看现有的头像组件，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import AvatarGroup from \"./AvatarGroup\";\nexport default function Demo() { return <AvatarGroup />; }",
  },
};
