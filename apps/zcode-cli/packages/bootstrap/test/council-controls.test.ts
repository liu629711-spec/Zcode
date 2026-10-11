// ============================================================
// 圆桌会控场命令（人工控场批）的可运行检查。
// 驱动真实 handler（council-controls.ts），桩掉 host/record/runtime：
//  1. councilPause 把 councilId/paused 原样带给 runtime，ACK 带最终旗标；
//  2. councilInterject 透传 text 与点名座位，ACK 带点名席位数（缺省=0 全体可见）；
//  3. 会话不在册（冷会话未恢复）→ V4SessionNotFoundError，不吞错。
// 串行链/暂停闸门/终态忽略在 core（council-meeting.test.ts 已盖），这里不重复。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/council-controls.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import type { CommandEnvelope } from "@zcode/shared/zcode-protocol-v4";
import { councilControlHandlers } from "../src/zcode-protocol-v4/commands/handlers/council-controls.js";
import { V4SessionNotFoundError } from "../src/zcode-protocol-v4/commands/record-access.js";
import type {
  V4CommandCoreHost,
  V4SessionRecordView,
} from "../src/zcode-protocol-v4/commands/types.js";

interface RuntimeCallLog {
  pauses: Array<{ councilId: string; paused: boolean }>;
  interjections: Array<{
    councilId: string;
    text: string;
    targetSeatIndexes?: readonly number[];
  }>;
}

function makeHost(runtimeLog: RuntimeCallLog, sessionId: string | null): V4CommandCoreHost {
  const record = {
    app: {
      runtime: {
        async setCouncilMeetingPaused(input: { councilId: string; paused: boolean }) {
          runtimeLog.pauses.push({ councilId: input.councilId, paused: input.paused });
        },
        async addCouncilInterjection(input: {
          councilId: string;
          text: string;
          targetSeatIndexes?: readonly number[];
        }) {
          runtimeLog.interjections.push({
            councilId: input.councilId,
            text: input.text,
            ...(input.targetSeatIndexes ? { targetSeatIndexes: input.targetSeatIndexes } : {}),
          });
        },
      },
    },
    traceContext: { traceId: "trace-1" },
  } as unknown as V4SessionRecordView;
  return {
    getRecord: (id: string) => (sessionId !== null && id === sessionId ? record : undefined),
  } as V4CommandCoreHost;
}

function envelope(
  type: "councilPause" | "councilInterject",
  payload: Record<string, unknown>,
  sessionId: string | null,
): CommandEnvelope {
  return { type, payload, sessionId, commandId: "cmd-1" } as unknown as CommandEnvelope;
}

test("councilPause：councilId/paused 原样进 runtime，ACK 带旗标", async () => {
  const log: RuntimeCallLog = { pauses: [], interjections: [] };
  const host = makeHost(log, "sess-1");
  const ack = await councilControlHandlers.councilPause(
    host,
    envelope("councilPause", { councilId: "council-9", paused: true }, "sess-1"),
  );
  assert.deepEqual(log.pauses, [{ councilId: "council-9", paused: true }]);
  assert.deepEqual(ack, { type: "councilPause", paused: true });
});

test("councilInterject：text 与点名座位透传，ACK 带点名数；缺省=0", async () => {
  const log: RuntimeCallLog = { pauses: [], interjections: [] };
  const host = makeHost(log, "sess-1");
  const ack = await councilControlHandlers.councilInterject(
    host,
    envelope(
      "councilInterject",
      { councilId: "council-9", text: "注意成本", targetSeatIndexes: [0, 2] },
      "sess-1",
    ),
  );
  assert.deepEqual(log.interjections, [
    { councilId: "council-9", text: "注意成本", targetSeatIndexes: [0, 2] },
  ]);
  assert.deepEqual(ack, { type: "councilInterject", targetedSeats: 2 });

  const ackAll = await councilControlHandlers.councilInterject(
    host,
    envelope("councilInterject", { councilId: "council-9", text: "全体注意" }, "sess-1"),
  );
  assert.deepEqual(ackAll, { type: "councilInterject", targetedSeats: 0 });
  assert.equal(log.interjections[1]?.targetSeatIndexes, undefined);
});

test("会话不在册：抛 V4SessionNotFoundError，不触 runtime", async () => {
  const log: RuntimeCallLog = { pauses: [], interjections: [] };
  const host = makeHost(log, "sess-1");
  await assert.rejects(
    councilControlHandlers.councilPause(
      host,
      envelope("councilPause", { councilId: "council-9", paused: true }, "sess-other"),
    ),
    V4SessionNotFoundError,
  );
  await assert.rejects(
    councilControlHandlers.councilInterject(
      host,
      envelope("councilInterject", { councilId: "council-9", text: "x" }, "sess-other"),
    ),
    V4SessionNotFoundError,
  );
  assert.deepEqual(log, { pauses: [], interjections: [] });
});
