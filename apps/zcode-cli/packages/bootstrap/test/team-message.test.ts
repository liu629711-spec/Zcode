// ============================================================
// 队内直达消息（团队看板批3）的可运行检查。
// 端口的找队/投递依赖 runtime 全家桶，这里测纯判定与快照聚合两条腿：
//  1. isTargetInTeam：工号优先/精确名/跨队拒绝；
//  2. buildTeamMessageText：carrier 前缀 + 有界截断；
//  3. 快照 messages：台账通讯行归队、时间正序、200 字截断。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/test/team-message.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  buildTeamMessageText,
  isTargetInTeam,
  teamMessageRowId,
} from "../src/zcode-protocol/team-message.js";
import { buildTeamBoardSnapshotFromRows, type TeamBoardInputRow } from "../src/zcode-protocol/team-board.js";

let seq = 0;

function dispatchRow(input: {
  workOrderId: string;
  batchId: string;
  agentName: string;
  agentId?: string;
}): TeamBoardInputRow {
  seq += 1;
  return {
    id: `row-${seq}`,
    kind: "agentWorkOrderDispatch",
    status: "admitted",
    admittedSequence: seq,
    time: { created: seq, updated: seq },
    payload: {
      text: "任务",
      workOrderId: input.workOrderId,
      agentName: input.agentName,
      ...(input.agentId ? { agentId: input.agentId } : {}),
      envelope: {
        workOrderId: input.workOrderId,
        fromAgentName: "boss",
        fromSessionId: "sess-boss",
        task: "任务",
        batchId: input.batchId,
      },
    } as TeamBoardInputRow["payload"],
  };
}

test("队内校验：工号优先，跨队/非成员拒绝", () => {
  const rows: TeamBoardInputRow[] = [
    dispatchRow({ workOrderId: "wo-1", batchId: "b1", agentName: "小陈", agentId: "agent-1" }),
    dispatchRow({ workOrderId: "wo-2", batchId: "b2", agentName: "小王", agentId: "agent-2" }),
  ];
  assert.equal(isTargetInTeam(rows, "b1", { name: "小陈", agentId: "agent-1" }), true);
  // b2 队里没有小陈（他只在 b1 派过单）。
  assert.equal(isTargetInTeam(rows, "b2", { name: "小陈", agentId: "agent-1" }), false);
  // 只对名字、无工号：b1 按名命中。
  assert.equal(isTargetInTeam(rows, "b1", { name: "小陈" }), true);
  assert.equal(isTargetInTeam(rows, "b1", { name: "小李" }), false);
});

test("消息 carrier：自带来源与回话指引，超长有界截断", () => {
  const text = buildTeamMessageText("小陈", "login 接口的字段是不是改了？");
  assert.ok(text.startsWith("【队内消息】来自 小陈"));
  assert.ok(text.includes("login 接口的字段是不是改了？"));
  const long = buildTeamMessageText("小陈", "长".repeat(20000));
  assert.ok(long.length < 20000, "超长截断");
  assert.equal(teamMessageRowId("wo-x"), "agentWorkOrderTeamMessage:wo-x");
});

test("快照：通讯行归队、时间正序、200 字截断（批3 通讯区数据源）", () => {
  function messageRow(input: { batchId: string; from: string; to: string; text: string; created: number }): TeamBoardInputRow {
    return {
      id: `msg-${input.created}`,
      kind: "agentWorkOrderTeamMessage",
      status: "admitted",
      admittedSequence: input.created,
      time: { created: input.created, updated: input.created },
      payload: { batchId: input.batchId, from: input.from, to: input.to, text: input.text } as TeamBoardInputRow["payload"],
    };
  }
  const all: TeamBoardInputRow[] = [
    dispatchRow({ workOrderId: "wo-1", batchId: "b1", agentName: "小陈" }),
    messageRow({ batchId: "b1", from: "小陈", to: "小李", text: "接口字段对一下".padEnd(300, "字"), created: 10 }),
    messageRow({ batchId: "b1", from: "小李", to: "小陈", text: "没改，按老字段来", created: 11 }),
    dispatchRow({ workOrderId: "wo-9", batchId: "b2", agentName: "小王", agentId: "agent-2" }),
    messageRow({ batchId: "b2", from: "小王", to: "小陈", text: "b2 队的消息", created: 12 }),
  ];
  const snapshot = buildTeamBoardSnapshotFromRows({ sessionId: "s", rows: all, liveStatusOf: () => "idle" });
  const b1 = snapshot.teams.find((team) => team.batchId === "b1")!;
  assert.equal(b1.messages?.length, 2, "只归本队的消息");
  assert.equal(b1.messages?.[0]!.to, "小李", "时间正序");
  assert.ok((b1.messages?.[0]!.text.length ?? 0) <= 200, "200 字截断");
  assert.equal(snapshot.teams.find((team) => team.batchId === "b2")!.messages?.[0]!.from, "小王");
});
