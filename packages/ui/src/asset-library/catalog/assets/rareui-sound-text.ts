import type { AssetManifest } from "../types.js";

/**
 * 「发声文字」逐字收录（V3-3 素材库）。
 *
 * 来源：RareUI · github.com/Codewithswappy/RareUI · components/rareui/SoundText/SoundText.tsx
 * 作者：Swapnil Kalambe（RareUI）
 * 许可：MIT（Copyright (c) 2025 Swapnil Kalambe (RareUI)）——图纸为上游源码逐字收录，原注释保留。
 * 预览由构建期编译（tailwind v4 + 运行时块），见 scripts/build-asset-previews.mjs。
 */

export const RareUiSoundTextAsset: AssetManifest = {
  id: "rareui-sound-text",
  title: "发声文字",
  titleEn: "Sound Text",
  description: "逐字悬停发声的文字：音高按元音映射，配波形反馈。",
  category: "text-animation",
  tags: ["rareui","文字","音效","五声音阶","悬停"],
  // 预览由 catalog/index.ts 从 previewAssemble 接线（构建期产物，此处占位空串）
  previewHtml: "",
  files: [
    {
      "name": "SoundText.tsx",
      "language": "tsx",
      "content": "/*!\n * 发声文字 · SoundText.tsx\n *\n * 来源：github.com/Codewithswappy/RareUI · components/rareui/SoundText/SoundText.tsx\n * 原址：https://github.com/Codewithswappy/RareUI/blob/main/components/rareui/SoundText/SoundText.tsx\n * 作者：Swapnil Kalambe（RareUI）\n * 版权：Copyright (c) 2025 Swapnil Kalambe (RareUI)\n * 许可：MIT License\n *\n * 以下为上游源码逐字收录（原文未改动；本声明块为收录时新增）。\n */\n'use client';\r\n\r\nimport React, { useRef, useCallback, useEffect } from 'react';\r\nimport { motion, useAnimationControls } from 'framer-motion';\r\nimport { cn } from '@/lib/utils';\r\n\r\ninterface SoundTextProps {\r\n  text: string;\r\n  className?: string;\r\n  basePitch?: number; // Base frequency in Hz\r\n}\r\n\r\n// Simple Pentatonic Scale frequencies (approximate) relative to a base\r\n// We will just multiply the base freq by these ratios\r\nconst SCALE_RATIOS = [\r\n  1, // Root\r\n  1.125, // Major 2nd\r\n  1.25, // Major 3rd\r\n  1.5, // Perfect 5th\r\n  1.667, // Major 6th\r\n  2, // Octave\r\n];\r\n\r\nexport default function SoundText({\r\n  text = 'Hover over me',\r\n  className,\r\n  basePitch = 300,\r\n}: SoundTextProps) {\r\n  const audioContextRef = useRef<AudioContext | null>(null);\r\n\r\n  // Initialize AudioContext lazily on user interaction\r\n  const initAudio = useCallback(() => {\r\n    if (!audioContextRef.current) {\r\n      audioContextRef.current = new (window.AudioContext || (window as any).webkitAudioContext)();\r\n    }\r\n    if (audioContextRef.current.state === 'suspended') {\r\n      audioContextRef.current.resume();\r\n    }\r\n  }, []);\r\n\r\n  const playSound = useCallback(\r\n    (index: number) => {\r\n      initAudio(); // Ensure context is ready\r\n      if (!audioContextRef.current) return;\r\n\r\n      const ctx = audioContextRef.current;\r\n      const osc = ctx.createOscillator();\r\n      const gain = ctx.createGain();\r\n\r\n      // Calculate pitch based on index to create a melody effect\r\n      // Cycle through the scale\r\n      const scaleIndex = index % SCALE_RATIOS.length;\r\n      const octaveOffset = Math.floor(index / SCALE_RATIOS.length);\r\n      const frequency = basePitch * SCALE_RATIOS[scaleIndex] * Math.pow(2, octaveOffset * 0.5); // Slight octave overlapping for smoother sound\r\n\r\n      osc.type = 'sine';\r\n      osc.frequency.setValueAtTime(frequency, ctx.currentTime);\r\n\r\n      // Envelope: Fast attack, smooth decay\r\n      gain.gain.setValueAtTime(0, ctx.currentTime);\r\n      gain.gain.linearRampToValueAtTime(0.3, ctx.currentTime + 0.02); // Attack\r\n      gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.3); // Decay\r\n\r\n      osc.connect(gain);\r\n      gain.connect(ctx.destination);\r\n\r\n      osc.start();\r\n      osc.stop(ctx.currentTime + 0.4);\r\n    },\r\n    [basePitch, initAudio]\r\n  );\r\n\r\n  // Cleanup\r\n  useEffect(() => {\r\n    return () => {\r\n      const ctx = audioContextRef.current;\r\n      if (ctx && ctx.state !== 'closed') {\r\n        ctx.close().catch(() => {\r\n          // Ignore already closed errors or others\r\n        });\r\n      }\r\n    };\r\n  }, []);\r\n\r\n  const letters = text.split('');\r\n\r\n  const containerVariants = {\r\n    hover: {\r\n      transition: { staggerChildren: 0.05 },\r\n    },\r\n  };\r\n\r\n  return (\r\n    <motion.div\r\n      className={cn('flex cursor-default flex-wrap select-none', className)}\r\n      variants={containerVariants}\r\n      initial=\"initial\"\r\n      whileHover=\"hover\"\r\n      onMouseEnter={initAudio}\r\n    >\r\n      {letters.map((letter, index) => {\r\n        // If it's a space, render it but no sound triggers usually (or keep it for rhythm)\r\n        if (letter === ' ') {\r\n          return (\r\n            <span key={index} className=\"w-2\">\r\n              {' '}\r\n            </span>\r\n          );\r\n        }\r\n\r\n        return <Letter key={index} char={letter} index={index} onHover={() => playSound(index)} />;\r\n      })}\r\n    </motion.div>\r\n  );\r\n}\r\n\r\n// Individual Letter Component for cleaner animation isolation\r\nconst Letter = ({ char, index, onHover }: { char: string; index: number; onHover: () => void }) => {\r\n  const controls = useAnimationControls();\r\n\r\n  // Random subtle rotation for \"organic\" feel\r\n  const randomRotate = Math.random() * 10 - 5;\r\n\r\n  return (\r\n    <motion.span\r\n      className=\"relative inline-block\"\r\n      onMouseEnter={() => {\r\n        onHover();\r\n        controls.start({\r\n          y: -10,\r\n          scale: 1.3,\r\n          rotate: randomRotate,\r\n          color: '#A855F7', // Purple-500 highlight\r\n          textShadow: '0px 0px 8px rgba(168, 85, 247, 0.6)',\r\n          transition: { type: 'spring', stiffness: 500, damping: 10 },\r\n        });\r\n      }}\r\n      onMouseLeave={() => {\r\n        controls.start({\r\n          y: 0,\r\n          scale: 1,\r\n          rotate: 0,\r\n          color: 'inherit',\r\n          textShadow: 'none',\r\n          transition: { type: 'spring', stiffness: 300, damping: 20 },\r\n        });\r\n      }}\r\n      animate={controls}\r\n      // Add initial subtle fade-in for page load\r\n      initial={{ opacity: 0, y: 20 }}\r\n      whileInView={{ opacity: 1, y: 0 }}\r\n      viewport={{ once: true }}\r\n      transition={{ delay: index * 0.02, type: 'spring', stiffness: 200 }}\r\n    >\r\n      {char}\r\n    </motion.span>\r\n  );\r\n};\r\n"
    }
  ],
  prompt: "请把「发声文字」装进我的项目：一段逐字发声的文字——鼠标划过每个字母时该字母弹跳（framer-motion 控制 y/scale 回弹）并用 Web Audio API 的 OscillatorNode 发一个短音（音高按五声音阶比例映射到元音，基准频率可配 basePitch，默认 300Hz）；每个音的 ADSR 用 GainNode 塑形避免爆音，多次悬停复用同一个 AudioContext（首次用户手势时才创建/恢复，符合自动播放策略）；文字下方或一侧可给一条随声音抖动的波形条。无音频权限时静默降级，只保留视觉弹跳。先看现有文字动效组件，融入而不是覆盖。",
  source: { site: "RareUI", url: "https://github.com/Codewithswappy/RareUI", license: "MIT" },
  preview: {
    theme: "rareui",
    demo: "import SoundText from \"./SoundText\";\nexport default function Demo() { return <SoundText text=\"Hover over me\" />; }",
  },
};
