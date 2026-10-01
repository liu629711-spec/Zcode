// ============================================================
// 班底进图纸（2026-10-01）的可运行检查：persona.profile 的名册解析、
// 身份拼装、模型优先级插档。
// 运行：npx tsx --test apps/zcode-cli/packages/bootstrap/src/app/workflow-actor-profile.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";
import type { AgentProfile } from "@zcode/core";
import type { ModelSelection } from "@zcode/shared/model-selection";
import { workflowActorModelPolicy } from "./workflow-actor-model.js";
import {
  composeWorkflowActorPersona,
  resolveWorkflowActorProfile,
  WorkflowActorProfileError,
} from "./workflow-actor-profile.js";

function profile(overrides: Partial<AgentProfile> & { name: string }): AgentProfile {
  return {
    description: "test crew member",
    source: "built-in",
    systemPrompt: `你是 ${overrides.name}。`,
    ...overrides,
  } as AgentProfile;
}

const ROSTER = [
  profile({ name: "code-reviewer", systemPrompt: "你是审查员：对照合格标准逐条复核。" }),
  profile({ name: "code-builder" }),
  profile({ name: "同名", source: "project" as const }),
  profile({ name: "同名", source: "user" as const }),
  profile({ name: "插件工", source: "plugin" as const }),
];

test("resolveWorkflowActorProfile：没点名（缺席/空串）→ undefined，不抛", () => {
  assert.equal(resolveWorkflowActorProfile({}, ROSTER), undefined);
  assert.equal(resolveWorkflowActorProfile({ profile: "  " }, ROSTER), undefined);
});

test("resolveWorkflowActorProfile：点名班底成员 → 按名命中（大小写不敏感）", () => {
  const resolved = resolveWorkflowActorProfile({ profile: "Code-Reviewer" }, ROSTER);
  assert.equal(resolved?.name, "code-reviewer");
});

test("resolveWorkflowActorProfile：名册没有 → 大声抛错，绝不静默退化成匿名工人", () => {
  assert.throws(
    () => resolveWorkflowActorProfile({ profile: "不存在的员工" }, ROSTER),
    (error: unknown) =>
      error instanceof WorkflowActorProfileError &&
      error.kind === "not_found" &&
      error.message.includes("不存在的员工"),
  );
});

test("resolveWorkflowActorProfile：撞名 → 歧义抛错并带双方名字", () => {
  assert.throws(
    () => resolveWorkflowActorProfile({ profile: "同名" }, ROSTER),
    (error: unknown) =>
      error instanceof WorkflowActorProfileError &&
      error.kind === "ambiguous" &&
      (error.matchedNames?.length ?? 0) === 2,
  );
});

test("resolveWorkflowActorProfile：插件档不是身份，点名它按缺失处理（与派单点名同纪律）", () => {
  assert.throws(
    () => resolveWorkflowActorProfile({ profile: "插件工" }, ROSTER),
    (error: unknown) => error instanceof WorkflowActorProfileError && error.kind === "not_found",
  );
});

test("composeWorkflowActorPersona：档案说明书是主体，脚本 system 是步骤附加要求", () => {
  const composed = composeWorkflowActorPersona(ROSTER[0]!, {
    system: "只审登录页这一个文件。",
  });
  assert.match(composed ?? "", /^你是审查员：对照合格标准逐条复核。/);
  assert.match(composed ?? "", /Step-specific requirements/);
  assert.match(composed ?? "", /只审登录页这一个文件。/);
  // 身份在前、附加要求在后：脚本能加要求，不能改写员工性格。
  assert.ok((composed ?? "").indexOf("Step-specific requirements") > (composed ?? "").indexOf("你是"));
});

test("composeWorkflowActorPersona：只点名没加要求 → 原样用档案说明书", () => {
  assert.equal(composeWorkflowActorPersona(ROSTER[0]!, {}), "你是审查员：对照合格标准逐条复核。");
});

const selection = (modelId: string): ModelSelection => ({ providerId: "prov", modelId });

test("workflowActorModelPolicy：档案模型插在 run 选择之下、pin 之上", () => {
  const profileModel = selection("crew-model");
  const runModel = selection("run-model");
  // run 显式选择永远最高。
  assert.deepEqual(
    workflowActorModelPolicy(
      { profileSelection: profileModel, runSelection: runModel },
      "prov/old",
    ).configOverrides,
    { modelSelection: runModel },
  );
  // 没 run 选择时档案模型压过 pin：员工自己的模型是身份的一部分。
  assert.deepEqual(
    workflowActorModelPolicy({ profileSelection: profileModel }, "prov/old").configOverrides,
    { modelSelection: profileModel },
  );
  // 没点名时优先级表完全不变（基线回归）。
  assert.deepEqual(workflowActorModelPolicy({}, undefined).configOverrides, {});
});
