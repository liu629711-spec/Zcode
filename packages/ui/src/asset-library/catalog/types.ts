/**
 * 素材卡格式（技术设计《ZCode-交互素材库-技术设计.md》§1）。
 *
 * 语言口径：title/description/prompt 以中文为准（本产品 locale 真源是 zh-CN，仓库
 * 只有 zh-CN/en-US 两种）；可选 titleEn/descriptionEn 供 S3 展厅按 locale 回退。
 * 这沿的是插件商店"基串 + 可选译文"的既有模式（shared 的 resolveLocalizedText(
 * locale, base, i18n)）——automationTemplateCatalog 的 {cn,en} 对象形状是给远端
 * scene 数据用的，改用它会把 §1 的 string 契约改掉，S2~S4 还没消费方，不做。
 */

export type AssetCategory =
  | "text-animation"
  | "background"
  | "control"
  | "block"
  | "prompt"
  /** V3-3 设计风格库：整套设计规范说明书（open-design 收录），不是组件。 */
  | "design-style";

/** 图纸文件：用户复制/发走的就是 content 原文。 */
export interface AssetFile {
  /** 图纸文件名，如 MeteorBackground.tsx */
  name: string;
  /** tsx | ts | css | html | javascript | md */
  language: string;
  /** 图纸原文 */
  content: string;
}

export interface AssetManifest {
  /** kebab-case 全局唯一，如 meteor-background */
  id: string;
  /** 展示名（中文为准） */
  title: string;
  titleEn?: string;
  /** 一句话描述（中文为准） */
  description: string;
  descriptionEn?: string;
  category: AssetCategory;
  /** 展示标签，每件 3~5 个 */
  tags: string[];
  /**
   * 自包含预览 HTML，运行时直接进 srcdoc。铁律见 AssetPreviewFrame.tsx：
   * 禁一切外链（`src="http…"`）、禁 localStorage（沙箱无源环境会抛）。
   * V3-3 逐字收录的 React 货由 catalog/index.ts 用 previewAssemble 惰性接线
   * （首次读取时才把运行时块 + CSS + 组件体拼成 HTML，模块级缓存）。
   *
   * 瘦身拆分（2026-10-05）：bodyFrom 在场的货此字段是空串——真身按 bodyFrom
   * 经 loadAssetBody（assetBodies.ts）按需解析，不再静态吸进行包/展厅 chunk。
   */
  previewHtml: string;
  /** 图纸；prompt 类货允许为空（口令本身即货） */
  files: AssetFile[];
  /**
   * 图纸真身条数（瘦身拆分 2026-10-05）：galaxy meta 货的 files 恒为空数组
   * （真身按 bodyFrom 按需解析），此字段保留真计数供 isPromptAsset 判定。
   * 正文就地的货缺席，调用方按 files.length 算。
   */
  filesCount?: number;
  /**
   * 正文下放标记（瘦身拆分 2026-10-05）：缺席 = previewHtml/files 就地是真身；
   * "preview-html" = 仅预览 HTML 在 preview-html.ts 惰性 map（files 仍就地）；
   * "galaxy:<源分类>" = 预览+图纸都在 generated/galaxy-bodies/<源分类>.ts
   * （源分类 = UIverse 收录的 11 个文件名，与货架六分类不是一回事，一个货架
   * 分类可能对应多个源分类 chunk）。统一经 loadAssetBody（assetBodies.ts）解析，
   * resolve 后与 meta 合并成完整 AssetManifest 再进交互面（iframe/代码面板/递活）。
   */
  bodyFrom?: "preview-html" | `galaxy:${string}`;
  /** 点单口令：发到会话时组装进消息的"要求"段（中文） */
  prompt: string;
  /** 出处+许可；自制货可省 */
  source?: { site: string; url: string; license: string };
  /**
   * V3-3 逐字收录（React+Tailwind 货）：构建期预览配置，只在
   * scripts/build-asset-previews.mjs 里消费，运行时不用（不是给用户看的字段）。
   *
   * files 是逐字交付的图纸原文；预览另有一份副本，因为沙箱禁外链（patches 把上游
   * 硬编码的远程图片换成内联 data URI），也因为多数组件要 props 才有画面（demo）。
   */
  preview?: {
    /** 预览主题 = 上游仓库 app/globals.css 抽出的 token（scripts/asset-preview-themes/）。 */
    theme: "beautifului" | "rareui";
    /** 演示 harness（tsx）：给组件喂 props，让预览有画面。 */
    demo: string;
    /** 预览副本的离线化改写；每条必须在源码里命中，否则构建期报错。 */
    patches?: { from: string; to: string; note: string }[];
    /** html 根节点是否挂 .dark（上游 dark 变体是祖先类）。 */
    dark?: boolean;
    /** 预览定位：center（默认居中）/ top（宽组件贴顶）。 */
    stage?: "center" | "top";
  };
}
