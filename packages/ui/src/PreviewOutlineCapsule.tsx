/* eslint-disable max-lines -- 大纲胶囊一个功能单元：树构建器 + 轨道线 + 胶囊 + 唤回按钮，拆文件只为凑行数。 */
import { useEffect, useMemo, useRef, useState } from "react";
import { motion, useReducedMotion } from "motion/react";
import {
  ChevronDownIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  ListTreeIcon,
  Maximize2Icon,
  Minimize2Icon,
} from "lucide-react";

import { cn } from "@/components/lib/utils.js";

export interface PreviewOutlineNode {
  key: string;
  text: string;
  children: PreviewOutlineNode[];
  /** 附带数据（MD 的平铺下标 / PDF 书签 dest），由调用方在 onJump 里消费。 */
  data?: unknown;
}

export interface PreviewOutlineSourceHeading {
  level: number;
  text: string;
}

export interface PreviewOutlineTreeNode extends PreviewOutlineNode {
  children: PreviewOutlineTreeNode[];
  /** 节点在源标题平铺序列中的起止下标（含端点），用于滚动同步定位。 */
  start: number;
  end: number;
}

/** 平铺标题（按顺序）→ 层级树；dataFor 可为每个标题附加跳转数据（如源码行号）。 */
export function buildPreviewOutlineTree(
  headings: PreviewOutlineSourceHeading[],
  dataFor?: (index: number) => unknown,
): PreviewOutlineTreeNode[] {
  const roots: PreviewOutlineTreeNode[] = [];
  const stack: { node: PreviewOutlineTreeNode; level: number }[] = [];
  headings.forEach((heading, index) => {
    const node: PreviewOutlineTreeNode = {
      key: `${index}:${heading.text}`,
      text: heading.text,
      children: [],
      start: index,
      end: index,
      ...(dataFor ? { data: dataFor(index) } : null),
    };
    while (stack.length > 0) {
      const top = stack[stack.length - 1];
      if (!top || top.level < heading.level) {
        break;
      }
      stack.pop();
    }
    const parent = stack[stack.length - 1];
    if (!parent) {
      roots.push(node);
    } else {
      parent.node.children.push(node);
    }
    stack.push({ node, level: heading.level });
    for (let depth = stack.length - 1; depth >= 0; depth -= 1) {
      const frame = stack[depth];
      if (frame) {
        frame.node.end = index;
      }
    }
  });
  return roots;
}

/** 滚动同步：定位覆盖给定平铺下标的最深节点。 */
export function findActiveOutlineNodeKey(
  nodes: PreviewOutlineTreeNode[],
  index: number,
): string | null {
  for (const node of nodes) {
    if (index >= node.start && index <= node.end) {
      return findActiveOutlineNodeKey(node.children, index) ?? node.key;
    }
  }
  return null;
}

/** 所有带子级的节点 key（全部展开/收起用）。 */
export function collectOutlineParentKeys(
  nodes: PreviewOutlineNode[],
  out: string[] = [],
): string[] {
  for (const node of nodes) {
    if (node.children.length > 0) {
      out.push(node.key);
      collectOutlineParentKeys(node.children, out);
    }
  }
  return out;
}

/*
 * 预览大纲胶囊卡（悬浮在预览内容上方，不占布局列）：
 * - 默认只展开最大一级标题，子标题按文件树的方式点击展开（行揭示动画）；
 * - 列表左侧是轨道线：品牌色短轨带圆角拐弯指向当前章节（spring 跟随），
 *   悬停其他行时显示虚线轨（HookSidebar 式 Rail 的自实现）；
 * - 当前章节的祖先链自动展开（SiYuan 行为），点击行跳转（Yank 行为）。
 */

const CORNER = 6;
const DASH = "repeating-linear-gradient(to top, transparent 0 2px, currentColor 2px 4px)";

function collectTrail(nodes: PreviewOutlineNode[], key: string, trail: string[]): string[] | null {
  for (const node of nodes) {
    const nextTrail = [...trail, node.key];
    if (node.key === key) {
      return nextTrail;
    }
    const found = collectTrail(node.children, key, nextTrail);
    if (found) {
      return found;
    }
  }
  return null;
}

interface FlatRow {
  node: PreviewOutlineNode;
  depth: number;
}

function flattenVisible(
  nodes: PreviewOutlineNode[],
  expandedKeys: Set<string>,
  depth = 0,
  out: FlatRow[] = [],
): FlatRow[] {
  for (const node of nodes) {
    out.push({ node, depth });
    if (node.children.length > 0 && expandedKeys.has(node.key)) {
      flattenVisible(node.children, expandedKeys, depth + 1, out);
    }
  }
  return out;
}

function Rail({
  from = 0,
  y,
  visible,
  color,
  dashed = false,
  className,
}: {
  from?: number;
  y: number | null;
  visible: boolean;
  color?: string;
  dashed?: boolean;
  className?: string;
}) {
  const reduced = useReducedMotion();
  const travel = reduced
    ? { duration: 0 }
    : { type: "spring" as const, stiffness: 420, damping: 34, mass: 0.7 };

  return (
    <motion.span
      aria-hidden="true"
      initial={false}
      style={{ color }}
      animate={{ opacity: visible && y !== null ? 1 : 0 }}
      transition={reduced ? { duration: 0 } : { duration: 0.2 }}
      className={cn("pointer-events-none absolute inset-0", className)}
    >
      <motion.span
        initial={false}
        animate={{ top: from, height: Math.max(0, (y ?? 0) - CORNER - from) }}
        transition={travel}
        style={dashed ? { backgroundImage: DASH } : { backgroundColor: "currentColor" }}
        className="absolute left-0.5 w-px"
      />
      <motion.svg
        initial={false}
        animate={{ top: (y ?? 0) - CORNER }}
        transition={travel}
        width="12"
        height="7"
        viewBox="0 0 12 7"
        fill="none"
        className="absolute left-0.5"
      >
        <path
          d="M0.5 0a6 6 0 0 0 6 6H12"
          stroke="currentColor"
          strokeDasharray={dashed ? "2 2" : undefined}
        />
      </motion.svg>
    </motion.span>
  );
}

export function PreviewOutlineCapsule({
  nodes,
  activeKey = null,
  onJump,
  onClose,
  label,
  side = "right",
  expandAllLabel = "Expand all",
  collapseAllLabel = "Collapse all",
  className,
}: {
  nodes: PreviewOutlineNode[];
  activeKey?: string | null;
  onJump: (node: PreviewOutlineNode) => void;
  onClose: () => void;
  label: string;
  /** 胶囊停靠侧：right 在预览内容右上角，left 在左上角（PDF）。只影响收起箭头方向。 */
  side?: "right" | "left";
  expandAllLabel?: string;
  collapseAllLabel?: string;
  className?: string;
}) {
  const [expandedKeys, setExpandedKeys] = useState<Set<string>>(() => new Set());
  const listRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<Map<string, HTMLElement | null>>(new Map());
  const [centers, setCenters] = useState<Map<string, number>>(new Map());
  const [hoverKey, setHoverKey] = useState<string | null>(null);

  const rows = useMemo(() => flattenVisible(nodes, expandedKeys), [nodes, expandedKeys]);
  const parentKeys = useMemo(() => collectOutlineParentKeys(nodes), [nodes]);
  const allExpanded = parentKeys.length > 0 && parentKeys.every((key) => expandedKeys.has(key));

  // 当前章节的祖先链自动展开（含当前节点自身，子标题随之揭示）。
  useEffect(() => {
    if (!activeKey) {
      return;
    }
    const trail = collectTrail(nodes, activeKey, []);
    if (!trail) {
      return;
    }
    setExpandedKeys((previous) => {
      const next = new Set(previous);
      for (const key of trail) {
        next.add(key);
      }
      return next;
    });
  }, [activeKey, nodes]);

  // 当前章节变化时把对应行滚进可视区（SiYuan 跟随行为）：只补不可见的偏差，
  // 不打断用户手动浏览胶囊列表；依赖 rows 让自动展开的新行先落位再滚。
  useEffect(() => {
    if (!activeKey) {
      return;
    }
    const list = listRef.current;
    const row = itemRefs.current.get(activeKey);
    if (!list || !row) {
      return;
    }
    const rowTop = row.offsetTop;
    const rowBottom = rowTop + row.offsetHeight;
    if (rowTop < list.scrollTop) {
      list.scrollTop = rowTop;
    } else if (rowBottom > list.scrollTop + list.clientHeight) {
      list.scrollTop = rowBottom - list.clientHeight;
    }
  }, [activeKey, rows]);

  // 行中心测量：轨道线的端点对齐到行垂直中心（HookSidebar 同款量法）。
  useEffect(() => {
    const list = listRef.current;
    if (!list) {
      return;
    }
    const measure = () => {
      const next = new Map<string, number>();
      for (const [key, element] of itemRefs.current) {
        if (element?.isConnected) {
          next.set(key, element.offsetTop + element.offsetHeight / 2);
        }
      }
      setCenters(next);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(list);
    measure();
    return () => {
      observer.disconnect();
    };
  }, [rows]);

  const toggleKey = (key: string) => {
    setExpandedKeys((previous) => {
      const next = new Set(previous);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  const activeY = activeKey ? (centers.get(activeKey) ?? null) : null;
  const hoverY = hoverKey ? (centers.get(hoverKey) ?? null) : null;
  const hoverFrom =
    activeY !== null && hoverY !== null && hoverY <= activeY
      ? Math.max(0, hoverY - CORNER)
      : (activeY ?? 0);
  const CloseIcon = side === "right" ? ChevronRightIcon : ChevronLeftIcon;

  return (
    /*
     * 外层是贯穿滚动内容的绝对定位条（pointer-events 穿透），胶囊本体用 sticky
     * 吸附在最近滚动视口的顶部：MD 预览的实际滚动可能发生在外层面板，
     * absolute 直接定位会跟随文档滚出视野，sticky 在两种滚动归属下都成立。
     */
    <div
      className={cn(
        "pointer-events-none absolute inset-y-0 z-20 w-64",
        // 右侧停靠时让开 14px 的内容滚动条，避免胶囊与滚动条重叠
        side === "right" ? "right-6" : "left-3",
        className,
      )}
    >
      <div
        role="navigation"
        aria-label={label}
        className="pointer-events-auto sticky top-3 flex max-h-[min(26rem,60vh)] w-full flex-col overflow-hidden rounded-2xl border border-popover-border bg-popover/95 shadow-lg backdrop-blur-sm"
      >
        <div className="flex h-9 shrink-0 items-center justify-between border-b border-border/60 px-3">
          <span className="text-ui-sm font-medium text-foreground-subtle">{label}</span>
          <div className="flex items-center gap-0.5">
            {parentKeys.length > 0 ? (
              <button
                type="button"
                aria-label={allExpanded ? collapseAllLabel : expandAllLabel}
                title={allExpanded ? collapseAllLabel : expandAllLabel}
                onClick={() => {
                  if (allExpanded) {
                    /*
                     * 全部收起保留第一大类：多根文档根本身即大类，收到根为止；
                     * 单根文档（根是全文标题，如 DESIGN.md）根下那层才是大类，
                     * 保留根的展开让大类可见，否则列表只剩一行等于全收没了。
                     */
                    const firstRoot = nodes[0];
                    setExpandedKeys(
                      nodes.length === 1 && firstRoot && firstRoot.children.length > 0
                        ? new Set([firstRoot.key])
                        : new Set<string>(),
                    );
                  } else {
                    setExpandedKeys(new Set(parentKeys));
                  }
                }}
                className="flex size-6 items-center justify-center rounded-md text-foreground-subtlest transition-colors hover:bg-hover hover:text-foreground"
              >
                {allExpanded ? (
                  <Minimize2Icon className="size-3.5" aria-hidden="true" />
                ) : (
                  <Maximize2Icon className="size-3.5" aria-hidden="true" />
                )}
              </button>
            ) : null}
            <button
              type="button"
              aria-label={label}
              title={label}
              onClick={onClose}
              className="flex size-6 items-center justify-center rounded-md text-foreground-subtlest transition-colors hover:bg-hover hover:text-foreground"
            >
              <CloseIcon className="size-3.5" aria-hidden="true" />
            </button>
          </div>
        </div>
        <div
          ref={listRef}
          className="relative min-h-0 flex-1 overflow-auto px-1.5 py-1.5"
          onMouseLeave={() => setHoverKey(null)}
        >
          <Rail
            from={hoverFrom}
            y={hoverY}
            visible={hoverY !== null && hoverKey !== activeKey}
            dashed
            className="text-foreground/30"
          />
          <Rail from={0} y={activeY} visible={activeY !== null} color="var(--color-brand)" />{" "}
          {rows.map(({ node, depth }) => {
            const active = node.key === activeKey;
            const hasChildren = node.children.length > 0;
            const expanded = expandedKeys.has(node.key);
            return (
              <button
                key={node.key}
                ref={(element) => {
                  itemRefs.current.set(node.key, element);
                }}
                type="button"
                data-outline-active={active ? "true" : undefined}
                aria-current={active ? "true" : undefined}
                title={node.text}
                onMouseEnter={() => setHoverKey(node.key)}
                onClick={() => {
                  onJump(node);
                  if (hasChildren && !expandedKeys.has(node.key)) {
                    toggleKey(node.key);
                  }
                }}
                style={{
                  paddingLeft: `${20 + depth * 12}px`,
                  ...(depth > 0 ? { animationDelay: `${Math.min(depth, 4) * 30}ms` } : null),
                }}
                className={cn(
                  "flex h-7 w-full min-w-0 items-center gap-1 rounded-lg pr-2 text-left text-ui-sm transition-colors duration-200 motion-reduce:transition-none",
                  depth > 0 && "tree-row-reveal",
                  active
                    ? "bg-brand/10 font-medium text-foreground"
                    : "text-foreground-subtle hover:text-foreground",
                )}
              >
                {hasChildren ? (
                  <span
                    role="button"
                    tabIndex={-1}
                    aria-label={node.text}
                    onClick={(event) => {
                      event.stopPropagation();
                      toggleKey(node.key);
                    }}
                    className="flex size-4 shrink-0 items-center justify-center rounded text-foreground-subtlest transition-colors hover:text-foreground"
                  >
                    <ChevronDownIcon
                      aria-hidden="true"
                      className={cn(
                        "size-3 transition-transform",
                        expanded ? "rotate-0" : "-rotate-90",
                      )}
                    />
                  </span>
                ) : (
                  <span className="size-4 shrink-0" />
                )}
                <span className="truncate">{node.text}</span>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

/** 收起状态下的悬浮唤回按钮（默认停靠右上、避开滚动条）。 */
export function PreviewOutlineFloatingButton({
  label,
  onClick,
}: {
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className="absolute right-6 top-2 z-10 flex size-7 items-center justify-center rounded-lg border border-border bg-card text-foreground-subtle shadow-xs transition-colors hover:text-foreground"
    >
      <ListTreeIcon className="size-4" aria-hidden="true" />
    </button>
  );
}
