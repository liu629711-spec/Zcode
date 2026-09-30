/**
 * 换班第一步的生成指令（对齐稿 docs/ZCode-员工工作流与上下文换班-可行性分析.md §2.4 十段模板）：
 * 让**当班会话自己**在换班瞬间把交接单写成文件——它的上下文此刻最全最新，
 * 绝不做压缩残骸的二次总结。产物落 <workspace>/.zcode/handovers/<sessionId>.md，
 * 老板可打开修改；接班会话的首条消息只带文件引用，由员工自己读入（弱模型抄错
 * 全文的风险收敛成"读文件"，路径是对的就不会搬错内容）。
 *
 * 纯字符串构造，无 React 依赖——测试直接喂参数断言关键约束。
 */
export function buildHandoverFilePath(workspacePath: string, sessionId: string): string {
  const normalized = workspacePath.replace(/[\\/]+$/, "");
  return `${normalized}/.zcode/handovers/${sessionId}.md`;
}

export function buildHandoverGenerationInstruction(
  workspacePath: string,
  sessionId: string,
): string {
  const filePath = buildHandoverFilePath(workspacePath, sessionId);
  return `你现在要交班了：这个会话的上下文已经用到需要换新会话继续的程度。请把**交接单**写入文件 \`${filePath}\`（用 Write 工具，目录不存在就先建），写完后只回复一句"交接单已写好：<文件路径>"，不要在回复里重复全文。

交接单用中文写（除非本会话一直是其他语言），按下面十段组织，务实、可判定，不写客套话：

0. **三句话老板版**：干到哪了 / 卡在哪 / 下一步是什么。
1. **老板的目标与原话要点**：列出老板每次提的要求；老板改过方向的，写明"最终口径"是哪一条。
2. **已经干完的**：每件附证据（文件路径 / 提交号 / 会话号）。
3. **干到一半的**：卡在哪一步、缺什么、最后一条命令或编辑是什么。
4. **还没干的**：待办清单，按优先级排序。
5. **死路与陷阱**：试过不行的方案、会踩的坑——接班的人不要再走一遍。
6. **关键决定及理由**：为什么这么做，防止接班的人自作聪明翻案。
7. **关键文件与坐标 + 复验方法**：路径:行号，以及"跑什么命令、看到什么结果算对"。
8. **与记忆柜的分工**：哪些长期知识已经写进你的记忆（交接单不重复它们），本单临时状态只有交接单里有。
9. **红线**：老板交代过"不要做/不要动"的事项，逐字保留。

写完自查四条，不满足就改完再收笔：老板每条消息都有着落吗；文件路径都真实存在吗；第 4 段和老板最后一条消息对齐吗；第 9 段红线一条不少吗。整单控制在 3000 字内：超了就砍第 2 段的细节，保第 4、5 段。`;
}

/** 接班会话的首条消息：不搬运交接单全文，只指路——员工自己读文件，路径对就不会搬错。 */
export function buildHandoverFirstInput(
  workspacePath: string,
  previousSessionId: string,
): string {
  const filePath = buildHandoverFilePath(workspacePath, previousSessionId);
  return `你是上一班次的接班人。上一班次的交接单在 \`${filePath}\`：先用 Read 把它完整读一遍，再按交接单继续工作——先做第 4 段（还没干的）里优先级最高的事，避开第 5 段的死路，遵守第 9 段的红线。读完不要复述全文，直接开始干活；有拿不准的先问老板。`;
}
