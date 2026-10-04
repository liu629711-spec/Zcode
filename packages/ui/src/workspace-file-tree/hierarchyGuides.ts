import type { CSSProperties } from "react";

/* 层级引导线：每个缩进层级一条 1px 实线（对齐参考树形样式的向导线），
 * 配合行内的横向连接线组成"竖线 + 横枝"的目录树骨架。 */
const WORKSPACE_FILE_TREE_HIERARCHY_GUIDE_BACKGROUND =
  "repeating-linear-gradient(to right, var(--color-border) 0 1px, transparent 1px 0.75rem)";

export function getWorkspaceFileTreeHierarchyGuideStyle(depth: number): CSSProperties | null {
  if (depth <= 0) {
    return null;
  }

  return {
    width: `calc(${depth} * 0.75rem)`,
    backgroundImage: WORKSPACE_FILE_TREE_HIERARCHY_GUIDE_BACKGROUND,
  };
}
