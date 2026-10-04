import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { MarkdownSelectionTooltip } from "@/v4/MarkdownSelectionTooltip.js";
import type { MarkdownSelectionTarget } from "@/lib/conversationSelectionReference.js";
import { MessageResponse } from "@/components/ai-elements/message.js";
import {
  PreviewOutlineCapsule,
  PreviewOutlineFloatingButton,
  buildPreviewOutlineTree,
  findActiveOutlineNodeKey,
} from "@/PreviewOutlineCapsule.js";
import { useZCodeIntl } from "@/i18n/IntlProvider.js";
import type { CodePreviewSettings } from "@/lib/codePreviewSettings.js";
import type { Theme } from "@/useTheme.js";

interface MarkdownPreviewContentProps {
  content: string;
  sourceKey?: string;
  sourceTitle?: string;
  sourcePath?: string;
  selectionTarget?: MarkdownSelectionTarget;
  workspacePath?: string;
  /** 应用主题（store 耦合剥离）：透传给 markdown 渲染，缺省按 "system" 兜底。 */
  theme?: Theme;
  /** 代码预览设置（store 耦合剥离）：透传给 markdown 渲染，需保持引用稳定。 */
  codePreviewSettings?: CodePreviewSettings;
  onOpenBrowserUrl?: (url: string) => void;
}

export function MarkdownPreviewContent({
  content,
  sourceKey,
  sourceTitle,
  sourcePath,
  selectionTarget,
  workspacePath,
  theme,
  codePreviewSettings,
  onOpenBrowserUrl,
}: MarkdownPreviewContentProps) {
  const { intl } = useZCodeIntl();
  const rootRef = useRef<HTMLDivElement>(null);
  const headingElementsRef = useRef<HTMLElement[]>([]);
  const [headings, setHeadings] = useState<{ level: number; text: string }[]>([]);
  const [activeHeadingIndex, setActiveHeadingIndex] = useState<number | null>(null);
  const [outlineOpen, setOutlineOpen] = useState(true);
  const outlineLabel = intl.formatMessage({ id: "preview.outline" });
  const outlineExpandAllLabel = intl.formatMessage({ id: "preview.outline.expandAll" });
  const outlineCollapseAllLabel = intl.formatMessage({ id: "preview.outline.collapseAll" });
  const selectionScope = useMemo(
    () => ({}),
    [content, sourceKey, sourcePath, selectionTarget?.workspaceKey, selectionTarget?.sessionId],
  );

  // 大纲提取（Yank Note 方式）：从渲染后的 DOM 抓 h1-h6，文本与跳转锚点天然一致。
  // MessageResponse 是异步渲染，用 MutationObserver 防抖重提取。
  useEffect(() => {
    const root = rootRef.current;
    if (!root) {
      return;
    }
    let timer: number | null = null;
    const extract = () => {
      const nodes = Array.from(root.querySelectorAll<HTMLElement>("h1, h2, h3, h4, h5, h6")).filter(
        (node) => (node.textContent ?? "").trim().length > 0,
      );
      headingElementsRef.current = nodes;
      setHeadings(
        nodes.map((node) => ({
          level: Number(node.tagName.slice(1)),
          text: (node.textContent ?? "").trim(),
        })),
      );
    };
    const schedule = () => {
      if (timer !== null) {
        window.clearTimeout(timer);
      }
      timer = window.setTimeout(extract, 120);
    };
    schedule();
    const observer = new MutationObserver(schedule);
    observer.observe(root, { childList: true, subtree: true, characterData: true });
    return () => {
      observer.disconnect();
      if (timer !== null) {
        window.clearTimeout(timer);
      }
      headingElementsRef.current = [];
    };
  }, [content, sourceKey, sourcePath]);

  // 滚动同步（Yank 的激活规则）：视口上部 40% 以内最近的标题是当前章节；
  // 大纲列表内部再按 SiYuan 的行为把当前项滚进可视区。
  useEffect(() => {
    const root = rootRef.current;
    if (!root) {
      return;
    }
    let frame = 0;
    const sync = () => {
      frame = 0;
      // 面板被收起（display:none）时所有矩形归零，会误判到最后一章，跳过。
      if (!root.isConnected || root.getClientRects().length === 0) {
        return;
      }
      const containerTop = root.getBoundingClientRect().top;
      const threshold = root.clientHeight * 0.4;
      const nodes = headingElementsRef.current;
      let current: number | null = null;
      for (let index = 0; index < nodes.length; index += 1) {
        const node = nodes[index];
        if (!node) {
          continue;
        }
        if (node.getBoundingClientRect().top - containerTop <= threshold) {
          current = index;
        } else {
          break;
        }
      }
      setActiveHeadingIndex(current);
    };
    const onScroll = () => {
      if (frame) {
        return;
      }
      frame = window.requestAnimationFrame(sync);
    };
    /*
     * 滚动可能发生在预览容器自身，也可能发生在外层面板（sticky 胶囊注释同款问题）。
     * scroll 事件不冒泡，但 capture 阶段能在 document 上收到所有元素的滚动，
     * 两种滚动归属都能触发同步；量法对两者都成立（容器与标题一起位移）。
     */
    document.addEventListener("scroll", onScroll, { capture: true, passive: true });
    sync();
    return () => {
      document.removeEventListener("scroll", onScroll, { capture: true });
      if (frame) {
        window.cancelAnimationFrame(frame);
      }
    };
  }, [headings]);

  const jumpToHeading = useCallback((index: number) => {
    headingElementsRef.current[index]?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, []);

  const outlineTree = useMemo(() => buildPreviewOutlineTree(headings), [headings]);
  const activeOutlineKey = useMemo(
    () =>
      activeHeadingIndex === null
        ? null
        : findActiveOutlineNodeKey(outlineTree, activeHeadingIndex),
    [activeHeadingIndex, outlineTree],
  );

  return (
    <div className="relative flex h-full w-full bg-background">
      <div ref={rootRef} data-markdown-preview="true" className="min-w-0 flex-1 overflow-auto">
        {selectionTarget && sourceKey ? (
          <MarkdownSelectionTooltip
            scopeKey={selectionScope}
            rootRef={rootRef}
            sourceKey={sourceKey}
            sourceTitle={sourceTitle ?? sourceKey}
            sourcePath={sourcePath}
            target={selectionTarget}
          />
        ) : null}
        <div className="min-h-full bg-background p-4">
          <MessageResponse
            className="min-w-0 break-words"
            workspacePath={workspacePath}
            theme={theme}
            codePreviewSettings={codePreviewSettings}
            onOpenExternalUrl={onOpenBrowserUrl}
          >
            {content}
          </MessageResponse>
        </div>
      </div>
      {outlineOpen && outlineTree.length > 0 ? (
        <PreviewOutlineCapsule
          label={outlineLabel}
          nodes={outlineTree}
          activeKey={activeOutlineKey}
          expandAllLabel={outlineExpandAllLabel}
          collapseAllLabel={outlineCollapseAllLabel}
          onJump={(node) => jumpToHeading(Number(node.data))}
          onClose={() => setOutlineOpen(false)}
        />
      ) : null}
      {!outlineOpen && outlineTree.length > 0 ? (
        <PreviewOutlineFloatingButton label={outlineLabel} onClick={() => setOutlineOpen(true)} />
      ) : null}
    </div>
  );
}
