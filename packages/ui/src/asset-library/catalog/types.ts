/**
 * 素材卡格式（技术设计《ZCode-交互素材库-技术设计.md》§1）。
 *
 * 语言口径：title/description/prompt 以中文为准（本产品 locale 真源是 zh-CN，仓库
 * 只有 zh-CN/en-US 两种）；可选 titleEn/descriptionEn 供 S3 展厅按 locale 回退。
 * 这沿的是插件商店"基串 + 可选译文"的既有模式（shared 的 resolveLocalizedText(
 * locale, base, i18n)）——automationTemplateCatalog 的 {cn,en} 对象形状是给远端
 * scene 数据用的，改用它会把 §1 的 string 契约改掉，S2~S4 还没消费方，不做。
 */

export type AssetCategory = "text-animation" | "background" | "control" | "block" | "prompt";

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
   */
  previewHtml: string;
  /** 图纸；prompt 类货允许为空（口令本身即货） */
  files: AssetFile[];
  /** 点单口令：发到会话时组装进消息的"要求"段（中文） */
  prompt: string;
  /** 出处+许可；自制货可省 */
  source?: { site: string; url: string; license: string };
}
