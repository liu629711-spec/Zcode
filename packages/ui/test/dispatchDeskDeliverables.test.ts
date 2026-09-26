import assert from "node:assert/strict";
import test from "node:test";
import { mapDispatchDeskDeliverables } from "../src/workspace-grouped-tasks/kanban-deliverables.js";

// ============================================================
// 派活台卡片成卡行映射的可运行检查（纯函数表驱动，不碰 React）
// ============================================================
// deliverables 无 schema（unknown JSON）：钉的是防御分类三分——
// undefined 缺席不渲染 / 可辨认文件项（path|file 非空字符串）成迷你卡 / 其余任意 JSON 一行摘要就地截断。
// 运行：npx tsx --tsconfig packages/ui/tsconfig.json --test packages/ui/test/dispatchDeskDeliverables.test.ts

test("deliverables undefined 缺席返回 null；其余任意 JSON 不抛且必有分类", () => {
  assert.equal(mapDispatchDeskDeliverables(undefined), null);
  const samples: unknown[] = [
    null,
    "",
    0,
    false,
    3.14,
    "text",
    [],
    [1, "a", { x: 1 }],
    {},
    { a: { b: { c: [1, { d: 2 }] } } },
  ];
  for (const sample of samples) {
    const rows = mapDispatchDeskDeliverables(sample);
    assert.ok(rows && rows.length >= 1, `样本必有分类：${JSON.stringify(sample)}`);
  }
});

test("可辨认文件项（path/file 非空字符串）成卡；数组须整体由文件项构成", () => {
  assert.deepEqual(mapDispatchDeskDeliverables({ path: "src/a.ts" }), [
    { kind: "file", title: "src/a.ts", subtitle: null },
  ]);
  // file 字段同源可辨；除 path/file 外的标量字段进副标题。
  assert.deepEqual(mapDispatchDeskDeliverables({ file: "b.md", lines: 3 }), [
    { kind: "file", title: "b.md", subtitle: "lines: 3" },
  ]);
  assert.deepEqual(mapDispatchDeskDeliverables({ path: "c.ts", kind: "report", pass: true }), [
    { kind: "file", title: "c.ts", subtitle: "kind: report · pass: true" },
  ]);
  assert.deepEqual(mapDispatchDeskDeliverables([{ path: "a.ts" }, { file: "b.md" }]), [
    { kind: "file", title: "a.ts", subtitle: null },
    { kind: "file", title: "b.md", subtitle: null },
  ]);
});

test("变异自查：辨认只认 path/file 非空字符串字段，其余降级摘要——辨认写错一条这里必红", () => {
  // 近似字段名不认（防过宽）。
  assert.deepEqual(mapDispatchDeskDeliverables({ pathname: "a.ts" }), [
    { kind: "summary", text: '{"pathname":"a.ts"}' },
  ]);
  // 非字符串 / 空白 path 不认。
  for (const bad of [{ path: 123 }, { path: "   " }, { file: null }, { path: ["a.ts"] }]) {
    const rows = mapDispatchDeskDeliverables(bad);
    assert.equal(rows?.[0]?.kind, "summary", `非文件项降级摘要：${JSON.stringify(bad)}`);
  }
  // 混入非文件项的数组整体降级摘要（"对象或其数组"，不部分成卡）；空数组同。
  assert.equal(mapDispatchDeskDeliverables([{ path: "a.ts" }, 1])?.[0]?.kind, "summary");
  assert.equal(mapDispatchDeskDeliverables([])?.[0]?.kind, "summary");
});

test("摘要行 = JSON 原文，超封顶就地截断", () => {
  assert.deepEqual(mapDispatchDeskDeliverables(null), [{ kind: "summary", text: "null" }]);
  assert.deepEqual(mapDispatchDeskDeliverables(42), [{ kind: "summary", text: "42" }]);
  assert.deepEqual(mapDispatchDeskDeliverables({ a: { b: 1 } }), [
    { kind: "summary", text: '{"a":{"b":1}}' },
  ]);
  const [row] = mapDispatchDeskDeliverables({ blob: "x".repeat(500) }) ?? [];
  assert.equal(row?.kind, "summary");
  if (row?.kind === "summary") {
    assert.ok(row.text.length <= 200, "摘要必须就地截断，不能把巨型 JSON 塞进 DOM");
  }
});
