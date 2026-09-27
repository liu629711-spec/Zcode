// ============================================================
// persona-session 的可运行检查（node --test，Node 24 原生剥离类型直跑）
// ============================================================
// 驻场智能体主会话的两条纯拼装规则：system prompt 多段拼接、标题记账。
//
// 运行：node --test apps/zcode-cli/packages/core/src/subagent/persona-session.test.ts
// （core 的 tsconfig 已排除 **/*.test.ts，tsc 构建不受影响。）

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  buildProjectAgentSessionTitle,
  joinPersonaSystemPrompt,
} from "./persona-session.ts";

test("joinPersonaSystemPrompt：persona 与记忆段按 \\n\\n 拼接", () => {
  assert.equal(
    joinPersonaSystemPrompt(["你是代码审查员", "## Persistent Agent Memory\n..."]),
    "你是代码审查员\n\n## Persistent Agent Memory\n...",
  );
});

test("joinPersonaSystemPrompt：空段跳过，全空返回 undefined", () => {
  assert.equal(joinPersonaSystemPrompt([undefined, "", "persona"]), "persona");
  assert.equal(joinPersonaSystemPrompt([undefined, ""]), undefined);
  assert.equal(joinPersonaSystemPrompt([]), undefined);
});

test("buildProjectAgentSessionTitle：智能体名 · 首条输入", () => {
  assert.equal(
    buildProjectAgentSessionTitle("code-reviewer", "修复登录超时"),
    "code-reviewer · 修复登录超时",
  );
});
