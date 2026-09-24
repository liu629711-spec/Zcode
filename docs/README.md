# 项目文档

## 当前口径

| 层级 | 文件 | 写什么 |
| --- | --- | --- |
| 产品方向 | [product/核心.md](product/核心.md) | ZCode 的长期产品目标和衡量方式 |
| 工程模式规格 | [product/project-space/目标与边界.md](product/project-space/目标与边界.md) | 工程模式要解决的问题、产品规则和边界 |
| 工程能力基线 | [product/project-space/当前能力与缺口.md](product/project-space/当前能力与缺口.md) | 当前源码和验证证据，以及距离目标的缺口 |
| 验收场景 | [product/project-space/验收场景.md](product/project-space/验收场景.md) | 工程模式必须通过的用户场景 |
| ZCode 实现事实 | [zcode/现状.md](zcode/现状.md) | 当前源码、协议和运行时事实 |
| 动态工作流验证 | [product/当前能力验证报告.md](product/当前能力验证报告.md) | 特定基线下的实际运行证据和检查结果 |

`.workbuddy/`、扫描配置和临时笔记不是产品口径。产品决策以本目录和对应 spec 为准，代码事实以当前检出源码为准。

## 文档规则

- 产品目标、实现事实和一次性验证证据分开记录。
- 行为改动先更新对应 spec，再设计和实现代码。
- “源码存在”不等于“用户入口可用”；“运行完成”不等于“交付完成”。
- 新能力必须说明状态所有者、事件顺序、持久化事实和验收方式。
