/**
 * 沙箱小房间——素材预览的唯一容器（技术设计 §2）。
 *
 * **铁律：sandbox 永远只有 `allow-scripts` 一个值。**
 * 绝不允许 allow-same-origin / allow-popups / allow-top-navigation：
 * renderer 没有 CSP（WorkflowArtifactBody.tsx 的既有教训——`allow-scripts` +
 * `allow-same-origin` 组合等价于逃逸），srcdoc 里的任何脚本都不可信。
 * 校验器 catalogCheck.assertSandboxSafe 钉住这个常量，谁改谁过不了检查。
 *
 * previewHtml 的收录规范（写给备货的人）：自包含、禁一切外链、禁 localStorage
 * （沙箱是无源环境，访问即抛）、动画货带 prefers-reduced-motion 降级。
 */

import { ASSET_SANDBOX } from "./catalog/catalogCheck.js";

/** iframe sandbox 的唯一合法值；定义与恒等校验在 catalogCheck（此处再导出）。 */
export { ASSET_SANDBOX };

interface AssetPreviewFrameProps {
  /** 自包含预览 HTML（catalogCheck 保证无外链/无 localStorage） */
  previewHtml: string;
  /** 无障碍标题（iframe 必带） */
  title: string;
}

/**
 * S1 最小实作：S2 在此补骨架加载态与「重试/复制兜底」，不改沙箱契约。
 */
export function AssetPreviewFrame({ previewHtml, title }: AssetPreviewFrameProps) {
  return (
    <iframe
      className="block h-full w-full rounded-lg border-0"
      sandbox={ASSET_SANDBOX}
      srcDoc={previewHtml}
      title={title}
    />
  );
}
