// ============================================================
// 工位退役反查（agentDispatch/checkTaskRetired）的可运行检查：
// retired=true → 不复用隐身工位；旧 Host（-32601）与查询故障 fail-open，
// 按未退役继续（守可见性不守授权，不能让派单炸）。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/src/zcode-protocol/agent-dispatch-port.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import { zcodeProtocolMethods } from "@zcode/shared";
import { isReuseSessionRetired } from "./agent-dispatch-port.js";
import { ProtocolRequestError } from "./server-types.js";

type RetiredCheckContext = Parameters<typeof isReuseSessionRetired>[0];

function contextReturning(outcome: () => Promise<unknown>) {
  const seen: { method?: string; params?: unknown } = {};
  const context = {
    requestClient: (async (method: string, params: unknown) => {
      seen.method = method;
      seen.params = params;
      return await outcome();
    }) as unknown as RetiredCheckContext["requestClient"],
  };
  return { context: context as RetiredCheckContext, seen };
}

test("isReuseSessionRetired：Host 报 retired=true 时不再复用该工位", async () => {
  const { context, seen } = contextReturning(async () => ({ retired: true, known: true }));
  assert.equal(await isReuseSessionRetired(context, "sess_worker"), true);
  assert.equal(seen.method, zcodeProtocolMethods.agentDispatchCheckTaskRetired);
  assert.deepEqual(seen.params, { targetTaskId: "sess_worker" });
});

test("isReuseSessionRetired：任务在册未退役（known=true, retired=false）照旧复用", async () => {
  const { context } = contextReturning(async () => ({ retired: false, known: true }));
  assert.equal(await isReuseSessionRetired(context, "sess_worker"), false);
});

test("isReuseSessionRetired：旧 Host 没有该方法（-32601）按未退役继续", async () => {
  const { context } = contextReturning(async () => {
    throw new ProtocolRequestError(-32601, "Method not found");
  });
  assert.equal(await isReuseSessionRetired(context, "sess_worker"), false);
});

test("isReuseSessionRetired：查询故障 fail-open（退化为修复前的复用行为，不炸派单）", async () => {
  const warnings: unknown[] = [];
  const { context } = contextReturning(async () => {
    throw new Error("db down");
  });
  const contextWithLogger = {
    ...context,
    logger: {
      warn: (message: string) => {
        warnings.push(message);
      },
    } as unknown as NonNullable<RetiredCheckContext["logger"]>,
  } as RetiredCheckContext;
  assert.equal(await isReuseSessionRetired(contextWithLogger, "sess_worker"), false);
  assert.equal(warnings.length, 1);
});
