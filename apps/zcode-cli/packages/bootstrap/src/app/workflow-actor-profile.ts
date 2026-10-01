// ============================================================
// 班底进图纸（2026-10-01）：actor persona 的员工引用解析与身份拼装。
// 脚本作者在 agent() 的 persona 里点名员工（profile: "code-reviewer"），
// 引擎只透传字符串（app-free 纪律）；本模块在宿主侧把它展开成身份：
// 名册档案的 systemPrompt 成为 actor 的人设（脚本 system 变步骤附加要求），
// 档案模型插进模型优先级（workflow-actor-model.ts）。
// 名字匹配复用 resolveWorkOrderTarget（与派单点名同一套大小写/来源/歧义纪律）。
// ============================================================

import type { AgentProfile } from "@zcode/core";
import { resolveWorkOrderTarget } from "@zcode/core";
import type { ModelSelection } from "@zcode/shared/model-selection";
import type { PersonaSpec } from "@zcode/dynamic-workflow";

/** 报错里最多列多少个可用员工名（全名单可能很长，bounded 诚实）。 */
const AVAILABLE_NAMES_MAX = 12;

/** 点名了员工但解析失败时抛出；not_found 带可用员工名单（照派单同名错误的先例，截断亮明），歧义带撞名双方。 */
export class WorkflowActorProfileError extends Error {
  readonly profileName: string;
  readonly kind: "not_found" | "ambiguous";
  /** not_found = 名册里可点名的员工名（截断诚实）；ambiguous = 撞名的双方。 */
  readonly names?: readonly string[];
  /** 名册总人数（not_found 时在场）；名单被截断时报错亮明「只列前 N」。 */
  readonly totalNames?: number;

  constructor(
    kind: "not_found" | "ambiguous",
    profileName: string,
    names?: readonly string[],
    totalNames?: number,
  ) {
    const listed =
      kind === "not_found" && totalNames !== undefined && totalNames > (names?.length ?? 0)
        ? `${(names ?? []).join("、")}……（名册共 ${totalNames} 人，只列前 ${names?.length ?? 0}）`
        : (names ?? []).join("、");
    super(
      kind === "not_found"
        ? `图纸里点名的员工「${profileName}」不在名册里（可能被改名或删除）。可点名的员工：${listed || "（名册是空的）"}。请改用其中一员，或删掉这个 profile 引用改用普通 persona。`
        : `图纸里点名的员工「${profileName}」在名册里撞了名（${(names ?? []).join("、")}）。请改用唯一的员工名，或按工号点名。`,
    );
    this.name = "WorkflowActorProfileError";
    this.profileName = profileName;
    this.kind = kind;
    if (names !== undefined) this.names = names;
    if (totalNames !== undefined) this.totalNames = totalNames;
  }
}

/**
 * persona 上的员工引用 → 名册档案。没点名返回 undefined；点名了但解析不出
 * （缺失/歧义）抛 {@link WorkflowActorProfileError}——**宁可让这一步响亮失败，
 * 也绝不悄悄退化成匿名工人**（那等于脚本作者点将点了个寂寞还不知道）。
 * 匹配口径与派单点名完全同源（大小写不敏感、限身份档三来源、歧义拒绝）。
 * 注意失败时机：解析发生在该 actor 首次派发建会话时（不是引擎 createActor），
 * 节点失败语义照旧——未被脚本接住时整个 run 以人话报错收场。
 */
export function resolveWorkflowActorProfile(
  persona: PersonaSpec,
  profiles: readonly AgentProfile[],
): AgentProfile | undefined {
  const wanted = persona.profile?.trim();
  if (wanted === undefined || wanted.length === 0) return undefined;
  const resolution = resolveWorkOrderTarget(wanted, profiles);
  if (resolution.kind === "resolved") return resolution.profile;
  if (resolution.kind === "ambiguous") {
    throw new WorkflowActorProfileError("ambiguous", wanted, resolution.matchedNames);
  }
  const identityProfiles = profiles.filter(
    (profile) =>
      profile.source === "project" ||
      profile.source === "user" ||
      profile.source === "built-in",
  );
  const availableNames = identityProfiles
    .map((profile) => profile.name)
    .slice(0, AVAILABLE_NAMES_MAX);
  throw new WorkflowActorProfileError(
    "not_found",
    wanted,
    availableNames,
    identityProfiles.length,
  );
}

/**
 * 员工档案模型 → actor 的模型覆盖（派单侧同款护栏，评审 B1/A5）：档案配的模型
 * 已下线/不在注册表时**不覆盖**（actor 自然继承父会话当前模型——与派单回落老板
 * 当前模型同一口径），返回 fellBack=true 让调用方留一条告警日志；注册表无法
 * 校验（缺 registry）时原样放行，失败在第一次 ask 时响亮浮出。
 */
export function profileModelSelection(input: {
  profile: AgentProfile | undefined;
  isModelAvailable?: (selection: ModelSelection) => boolean;
}): { modelSelection?: ModelSelection; fellBack: boolean } {
  const selection = input.profile?.modelSelection;
  if (selection === undefined) return { fellBack: false };
  if (input.isModelAvailable?.(selection) === false) return { fellBack: true };
  return { modelSelection: selection, fellBack: false };
}

/**
 * 有名员工 + 脚本人设 → actor 的 system 文本。员工档案的说明书是身份主体；
 * 脚本的 system（若给了）是**本步骤的附加要求**，追加在身份之后——图纸作者
 * 不该（也不能）改写员工的性格说明书，只能给这一步加要求。整体为空（无引用
 * 且脚本没给 system）返回 undefined，配置侧按「字段缺席」处理。
 * ponytail: 员工的**记忆本**（profile.memory）与**权限模式**不随行（2026-10-01
 * 拍板）：actor 会话还没有记忆身份管道（projectAgentPersona + 记忆工具白名单
 * 联动），且图纸并行扇出会让同一本记忆多写者互踩——真有需求先做只读；
 * 工具限制已随行（workflow-actor-tools.ts 的合成，含控制管线豁免）。
 */
export function composeWorkflowActorPersona(
  profile: AgentProfile,
  persona: PersonaSpec,
): string | undefined {
  const stepRequirements = persona.system?.trim();
  const base = profile.systemPrompt;
  const composed =
    stepRequirements === undefined || stepRequirements.length === 0
      ? base
      : [base, "## Step-specific requirements (from the workflow script)", "", stepRequirements].join(
          "\n",
        );
  return composed.trim().length > 0 ? composed : undefined;
}
