// ============================================================
// 预置班底数据的可运行检查（tsx --test）：这份名单同时被 services 名册
// （source "built-in" 虚拟列出）与 bootstrap 运行时档案装配（虚拟注入）消费，
// 名字重复/非法字符/缺描述都会让建档校验、点名派单、文件覆盖判重三处一起歪。
//
// 运行：npx tsx --test packages/shared/src/preset-crew.test.ts
// ============================================================

import assert from "node:assert/strict";
import { test } from "node:test";

import { PRESET_CREW_MEMBERS } from "./preset-crew.js";

test("预置班底五人齐全且名字唯一（大小写不敏感）", () => {
  assert.equal(PRESET_CREW_MEMBERS.length, 5);
  const seen = new Set<string>();
  for (const member of PRESET_CREW_MEMBERS) {
    const key = member.name.toLowerCase();
    assert.ok(!seen.has(key), `duplicate preset name: ${member.name}`);
    seen.add(key);
  }
});

test("每个成员都满足建档校验（名字 3-50 且仅字母数字连字符、描述与人设非空）", () => {
  for (const member of PRESET_CREW_MEMBERS) {
    assert.ok(member.name.length >= 3 && member.name.length <= 50, member.name);
    assert.match(member.name, /^[a-zA-Z0-9-]+$/u, member.name);
    assert.ok(member.description.trim().length > 0, member.name);
    assert.ok(member.systemPrompt.trim().length > 0, member.name);
    assert.ok(member.role.trim().length > 0, member.name);
  }
});
