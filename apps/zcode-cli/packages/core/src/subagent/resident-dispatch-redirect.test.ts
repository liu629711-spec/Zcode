// 驻场派遣改判的纯判据测试（2026-09-29 真机教训的钉子）。
// 核心不变量：@ 点将优先于改判；只有驻场身份档（项目档/用户级档）命中才改判；其余一律原样放行。

import assert from "node:assert/strict";
import { test } from "node:test";
import { resolveResidentDispatchRedirect } from "./resident-dispatch-redirect.js";
import type { AgentProfile } from "./profile.js";

function profile(name: string, source: AgentProfile["source"], agentId?: string): AgentProfile {
  return {
    name,
    description: `${name} description`,
    systemPrompt: `${name} prompt`,
    source,
    ...(agentId ? { agentId } : {}),
  } as AgentProfile;
}

const resident = profile("UI-plus", "project", "agent_21837dbb-2419-433a-9071-37345896a249");
// 插件子代理档在装配处改标 source "plugin"（bootstrap namespacePluginAgentProfile），
// 与用户级身份档（source "user"）分开——改判只认身份档。
const pluginAgent = profile("pptx", "plugin");
const globalWorker = profile("doc-writer", "user");
const builtin = profile("Explore", "built-in");
const profiles = [resident, globalWorker, pluginAgent, builtin];

test("项目作用域档案命中 → 改判（真机场景：模型调 Agent(type=UI-plus)）", () => {
  const decision = resolveResidentDispatchRedirect("UI-plus", [], profiles);
  assert.equal(decision.kind, "redirect");
  if (decision.kind === "redirect") assert.equal(decision.profile.name, "UI-plus");
});

test("大小写不敏感匹配（档案名落盘小写，模型可能传原名）", () => {
  assert.equal(resolveResidentDispatchRedirect("ui-plus", [], profiles).kind, "redirect");
  assert.equal(resolveResidentDispatchRedirect("UI-PLUS", [], profiles).kind, "redirect");
});

test("@ 点将优先：本轮有名单时一律放行，点将语义不被改判覆盖", () => {
  assert.equal(resolveResidentDispatchRedirect("UI-plus", ["UI-plus"], profiles).kind, "pass");
  assert.equal(resolveResidentDispatchRedirect("UI-plus", ["别的智能体"], profiles).kind, "pass");
});

test("用户级全局员工档命中 → 改判（身份轴终局 §九③：名册全局化后 Model 直呼也要转工单）", () => {
  const decision = resolveResidentDispatchRedirect("doc-writer", [], profiles);
  assert.equal(decision.kind, "redirect");
  if (decision.kind === "redirect") assert.equal(decision.profile.name, "doc-writer");
});

test("插件档案（source plugin）不改判：它们是可被派遣的子代理，不是驻场智能体", () => {
  assert.equal(resolveResidentDispatchRedirect("pptx", [], profiles).kind, "pass");
});

test("内置智能体（Explore/general-purpose）不改判", () => {
  assert.equal(resolveResidentDispatchRedirect("Explore", [], profiles).kind, "pass");
  assert.equal(resolveResidentDispatchRedirect("general-purpose", [], profiles).kind, "pass");
  assert.equal(resolveResidentDispatchRedirect("", [], profiles).kind, "pass");
});

test("未知名字不改判（保持原行为，不误伤既有无名派遣）", () => {
  assert.equal(resolveResidentDispatchRedirect("someone-else", [], profiles).kind, "pass");
});

test("空档案列表不改判（普通仓库无驻场智能体时行为零变化）", () => {
  assert.equal(resolveResidentDispatchRedirect("UI-plus", [], []).kind, "pass");
});

test("无号老档案也命中（工单端口按名字解析，号是加成不是门槛）", () => {
  const legacy = profile("code-plus", "project");
  assert.equal(resolveResidentDispatchRedirect("code-plus", [], [legacy]).kind, "redirect");
});
