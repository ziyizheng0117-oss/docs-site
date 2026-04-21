---
slug: claude-code-for-backend-dev
title: 我在后端项目里怎么用 Claude Code：命令、探索方式、Prompt 模板和项目配置
authors: [xiaoqu]
tags: [backend, troubleshooting, architecture]
---

如果一篇 Claude Code 分享只停留在“AI 正在改变软件开发方式”，那对后端开发来说通常不够用。

大家真正关心的是更具体的问题：**平时怎么启动、什么时候用 Plan Mode、怎么做代码探索、Prompt 怎么写、`CLAUDE.md` 怎么配、哪些场景最适合、哪些地方不要乱交给它。**

这篇文章不讲泛泛概念，只讲我认为对后端最实用的一套用法。默认场景是 Java / Spring Boot / MySQL / Redis / MQ 这一类典型服务端项目。

{/* truncate */}

## 先说结论：Claude Code 在后端里最有价值的，不是“写代码”，而是“推进任务”

后端研发里，很多工作都不是一句“帮我写个函数”能解决的，而是一条完整执行链：

- 读代码
- 找入口
- 梳理调用链
- 看配置
- 理解数据流
- 改实现
- 补测试
- 跑命令
- 看报错
- 继续修

Claude Code 真正适合的，就是这种带上下文、可验证、可以连续推进的工程任务。

所以如果只把它当成一个“高级补全工具”，其实会低估它；但如果把它当成“自动替你做所有技术决策的工程师”，又会高估它。

更准确一点的理解应该是：

> **Claude Code 最适合做后端研发里的工程执行工作，而不是替你决定业务本质。**

---

## 一、后端里最常用的 Claude Code 用法，其实没那么多

很多人第一次接触 Claude Code，会以为要背很多命令。实际上日常高频的也就几种。

### 1. 直接在项目里启动

```bash
cd your-project
claude
```

这适合：

- 先了解代码库
- 做小改动
- 连续在当前项目上下文里协作

如果任务不复杂，直接这样进就够了。

---

### 2. 用 Plan Mode 先分析，不直接改

```bash
claude --permission-mode plan
```

这是我最推荐后端优先掌握的用法。

因为很多后端任务并不适合上来就改代码，比如：

- 登录、支付、库存、账务
- 涉及事务边界的逻辑
- 跨多个模块的重构
- 带 schema 风险的改动
- 线上 bug 排查

这时候更稳的方式不是“直接写”，而是先让 Claude Code 做只读分析。

比如：

```text
先阅读 order 模块，梳理创建订单的主调用链、事务边界、库存校验和 MQ 发送时机。
先不要改代码，给我一个修复/改造方案。
```

Plan Mode 的核心价值不是更慢，而是更稳：**先拿结构化理解，再决定要不要动手。**

---

### 3. 用 headless plan 做一次性分析

```bash
claude --permission-mode plan -p "Analyze the authentication system and suggest improvements"
```

如果你只是想快速看一个问题，不想进入长会话，这个很好用。

例如后端场景里可以这么用：

```bash
claude --permission-mode plan -p "阅读 auth 模块，梳理登录链路、token 校验、token 刷新和异常处理路径"
```

它特别适合：

- 临时看一个模块
- 快速拿一份分析结果
- 避免会话里不小心直接改代码

---

### 4. 把日志、diff、grep 结果直接喂给它

这招对后端非常实用。

```bash
cat error.log | claude
```

```bash
git diff | claude
```

```bash
rg "createOrder|deductStock|sendMessage" src | claude
```

为什么有用？

因为很多后端问题本来就不是“凭空思考”，而是依赖这些中间材料：

- 错误栈
- 日志片段
- SQL 输出
- 配置片段
- diff 变更
- grep 出来的调用点

把原始上下文直接给 Claude Code，通常比你自己再转述一遍更高效。

---

## 二、先澄清一个容易误解的点：Claude Code 官方并没有一个独立的 `explore` 命令

这点值得专门说清楚。

很多人在聊 Claude Code 时会说“先 explore 一下代码库”，这没问题；但按目前官方公开文档，更准确的说法是：

- 用 **Plan Mode** 做探索
- 用 **`claude --permission-mode plan`** 做只读分析
- 在会话里直接输入探索型 prompt，让 Claude Code 去理解代码库

也就是说，**“explore”更像一种工作方式，不是一个必须写成 `cc explore ...` 的独立命令。**

这个区别很重要，不然很容易把概念和命令混在一起。

---

## 三、后端里怎么做“探索”，才算真正有用

后端场景里，探索不是“先随便看看代码”，而是系统性地把一个任务相关的局部地图摸清楚。

我一般会让 Claude Code 优先搞清楚下面这些东西。

### 1. 入口在哪

先找到请求从哪里进来：

- Controller / API handler
- RPC 接口实现
- MQ consumer
- 定时任务入口

例如：

```text
找出和用户登录相关的入口代码，告诉我 controller/handler 在哪里，请求进入后的第一层 service 是什么。
```

---

### 2. 主调用链怎么走

这是后端最重要的一层。

你要知道：

- controller -> service -> repository 是怎么串起来的
- 中间有没有 Redis / MQ / RPC / DB
- 哪一步最可能出问题

例如：

```text
梳理“用户登录”从 controller 到 service 到 repository 的主调用链。
输出关键类、主要方法、每一步职责，以及涉及的外部依赖。
```

---

### 3. 数据流怎么走

除了方法调用，还要看数据怎么流：

- 请求 DTO 怎么转内部对象
- 数据在哪层被补充、覆盖、裁剪
- 最终写入哪些表
- 返回值是怎么组装出来的

例如：

```text
追踪“创建用户”接口里的数据流：入参 DTO、内部对象转换、最终落库字段，以及返回值组装路径。
```

---

### 4. 事务边界和外部依赖在哪

后端很多坑都在这里。

要让 Claude Code 帮你标出来：

- 哪些方法有事务
- 事务包住了哪些操作
- 数据库、Redis、MQ、RPC 哪些在主链路上
- 是否存在一致性和幂等风险

例如：

```text
阅读订单创建链路，重点帮我找事务边界、数据库写入和 MQ 发送的关系，以及潜在一致性风险。
```

---

### 5. 配置和异常路径在哪

很多线上问题不是代码写错，而是配置或异常处理不一致。

比如：

- 超时和重试参数
- feature flag
- token 过期时间
- 熔断、限流、开关逻辑
- 异常是在哪里抛、在哪里 catch、最后返回什么

例如：

```text
帮我找登录相关配置项：token 过期时间、Redis key 配置、重试和超时设置，以及异常最终返回路径。
```

---

### 一个更完整的探索型 prompt 示例

如果我要改“创建订单”链路，我通常会先这样问：

```text
先不要改代码，先帮我完整探索“创建订单”链路。

我想知道：
1. 请求入口在哪
2. 主调用链怎么走
3. 涉及哪些核心对象
4. 哪些地方访问 MySQL、Redis、MQ、下游 RPC
5. 事务边界在哪
6. 异常路径和错误码怎么处理
7. 现有测试覆盖了哪些 case
8. 如果要加“设备维度限流”，最可能需要改哪些地方
9. 哪些地方改动风险最高

最后请输出：
- 关键文件清单
- 主链路步骤
- 风险点
- 建议改动点
```

这个比一句“帮我看看订单模块”实用得多。

---

## 四、后端开发里最值得收藏的 Prompt 模板

比命令更重要的，往往是 prompt 的组织方式。

同样是 Claude Code，有的人觉得很好用，有的人觉得一般，差别很多时候就在于：你给它的是一句模糊需求，还是一个工程任务。

### 1. 读模块 / 梳理链路

适合：刚接手模块、接线上问题、看老代码。

```text
阅读 payment 模块相关代码，帮我梳理：
1. 入口 controller
2. 核心 service
3. repository / mapper
4. 调用了哪些外部依赖（DB、Redis、MQ、RPC）
5. 哪些地方有事务边界
最后给我一个从请求进入到订单落库的主链路说明。
```

---

### 2. 修 bug

适合：500、NPE、边界 case、状态流转异常。

```text
用户反馈登录接口在 token 过期后偶发 500。
请先阅读 src/auth 下相关代码，找出可能异常路径。
要求：
1. 先写一个失败测试复现问题
2. 不要通过吞异常来“修复”
3. 修复后运行相关测试
4. 最后总结根因、修复点和潜在风险
```

这个模板的重点是：

- 先复现
- 再修复
- 跑验证
- 输出原因

---

### 3. 做中小型重构

适合：参数校验下沉、方法拆分、公共逻辑抽取。

```text
请重构 OrderService 中重复的参数校验逻辑。
要求：
1. 保持外部行为不变
2. 不修改接口返回结构
3. 尽量复用现有异常码
4. 不引入新依赖
5. 补必要测试并运行
```

后端重构最怕顺手改出额外变化，所以边界一定要写清楚。

---

### 4. 新增接口

适合：分页查询、后台管理接口、标准 CRUD。

```text
参考现有 AdminUserController、UserQueryService、UserRepository 的风格，
新增一个后台用户分页查询接口。

要求支持：
- 用户状态筛选
- 注册时间范围
- 关键词搜索（用户名/手机号）

限制：
- 不修改数据库 schema
- 不引入新依赖
- 保持现有响应结构风格一致

完成后：
- 补 controller/service 层测试
- 给我 curl 示例
- 说明索引风险和可能的慢 SQL 点
```

这就已经很接近一个真实后端需求了。

---

### 5. 看性能 / 做代码级性能 review

适合：RT 高、慢 SQL、串行依赖多。

```text
阅读这个接口相关代码和调用链，分析为什么它可能 RT 高。
重点看：
- 是否有 N+1 查询
- 是否有大对象组装
- 是否有串行 RPC
- 是否有不必要的锁/同步
- 是否有缓存缺失
最后按“高概率 / 中概率 / 低概率”列出问题点。
```

对后端来说，这类 prompt 很值钱，因为它把“看看有没有问题”变成了明确检查表。

---

## 五、比 Skill 更值得先配好的，是 `CLAUDE.md`

很多人会先问 skill 怎么写。

我的建议是：**先把 `CLAUDE.md` 配好，再谈 skill。**

因为 `CLAUDE.md` 是项目级说明书，Claude Code 进入项目时会自动读。对后端项目来说，它能显著减少“乱发挥”的概率。

这里最值得写进去的内容包括：

- 项目结构说明
- 构建和测试命令
- 模块职责
- 代码风格约束
- 明确禁止乱动的地方
- 提交前必须做的验证

### 一个后端项目里的 `CLAUDE.md` 示例

```md
# CLAUDE.md

## Project Overview
This is a Spring Boot backend service for order and payment processing.

## Modules
- order-api: controller / dto
- order-core: service / domain logic
- order-infra: repository / mysql / redis / mq

## Common Commands
- Build: `mvn -q -DskipTests package`
- Unit tests: `mvn -q test`
- Run single test: `mvn -q -Dtest=OrderServiceTest test`
- Checkstyle: `mvn -q checkstyle:check`

## Coding Rules
- Reuse existing DTO/VO naming style
- Do not introduce new dependencies unless explicitly requested
- Prefer existing repository pattern over direct mapper calls
- Keep exception codes consistent with ErrorCode enum

## Safety Rules
- Do not modify DB schema unless explicitly requested
- Do not remove backward-compatible fields from API responses
- For bug fixes, prefer writing a failing test first
- For refactors, preserve behavior and run relevant tests

## Before Finishing
Always:
1. summarize changed files
2. explain why the change is needed
3. run relevant tests or provide exact commands if not runnable
4. mention remaining risks
```

这个文件的本质不是“写几句提示词给 AI”，而是把团队的工程习惯结构化。

---

## 六、如果要讲 Skill，后端最值得做成模板的是哪几种

Skill 适合处理那些“重复发生、处理流程相对固定”的任务。

如果你在团队里长期用 Claude Code，我认为下面几类最值得沉淀。

### 1. bugfix skill

适合：

- 线上 bug
- 偶发 500
- NPE
- 边界 case 异常

推荐流程：

- 先找调用链
- 再写失败测试
- 再修代码
- 跑测试
- 输出根因和风险

---

### 2. api-implementation skill

适合：

- 新增 CRUD 接口
- 新增后台分页查询
- 增加筛选条件
- 给接口补鉴权 / 校验 / 审计日志

推荐约束：

- 参考现有 controller / service / repository 风格
- 不引入新依赖
- 不改 schema
- 自动补测试
- 自动给 curl 示例
- 自动提示索引 / SQL 风险

---

### 3. refactor skill

适合：

- 抽公共逻辑
- 去重
- 方法拆分
- 迁移旧模式

推荐约束：

- 保持行为不变
- 不改接口契约
- 小步改动
- 每步都可验证

---

### 4. code-review / risk-review skill

适合：

- PR review
- 上线前检查
- 核心链路风险复核

建议检查项：

- 空指针风险
- 事务边界
- 幂等性
- 并发问题
- SQL 风险
- 缓存一致性
- RPC 超时与重试放大

这类 skill 对后端团队尤其有价值，因为很多 review 点本来就是重复检查。

---

## 七、一个真实后端工作流示例：修 token 过期后接口 500

这部分比讲概念更能说明问题。

### 第一步：先只读探索，不急着改

```bash
claude --permission-mode plan
```

输入：

```text
阅读 src/auth 下相关代码，梳理登录、token 校验、token 刷新、异常处理路径。
我怀疑 token 过期后某个分支没有正确处理，导致 500。
先不要改代码，先给我定位路径和修复方案。
```

这一步 Claude Code 最适合做的是：

- 找入口 controller
- 找 token 校验逻辑
- 找异常兜底逻辑
- 输出可能问题点

---

### 第二步：确认方案后再让它动手

```text
按你的方案修复。
要求先写失败测试复现问题，再修改代码，并运行相关测试。
不要通过吞异常来掩盖问题。
```

这里最重要的是“先写失败测试”，因为后端修 bug 最怕的是：

- 看起来修了
- 实际只是绕过去了
- 下次换个输入又炸

---

### 第三步：让它输出结构化变更说明

```text
总结：
1. 根因是什么
2. 改了哪些文件
3. 为什么这样修
4. 还有哪些潜在风险
```

这个收尾动作很重要，因为它能把“改完了”变成“可复核、可交接、可回顾”。

---

## 八、哪些后端场景不建议直接让 Claude Code 硬改

Claude Code 很实用，但也别神化。

下面这些场景，我建议慎用直接改代码：

### 1. 资金、库存、账务等核心事务路径

如果规则边界你自己都没讲清楚，它大概率只能合理猜测。这类任务更适合先做方案、梳理逻辑、补测试，不适合直接放手改。

### 2. 高风险 schema 变更

删字段、改主键、改索引、历史数据回填，这些最多让它辅助分析，不建议直接自动推进。

### 3. 业务知识大量隐含在线下经验里

很多后端坑不在代码，而在团队约定。比如某个事件不能重放、某个字段虽然可空但线上脏数据很多、某个状态流转有隐藏前提。如果你不告诉它，它不可能自己知道。

---

## 九、给后端团队的几条实用建议

如果你是想在团队里推广 Claude Code，我觉得最值得强调的是这些：

1. **最常用的命令其实很少，重点是 Plan Mode**
2. **“探索”更像工作方式，不是一个必须单独记住的 `explore` 命令**
3. **真正影响效果的不是命令，而是任务定义是否清楚**
4. **优先配好 `CLAUDE.md`，比研究花哨 prompt 更有长期价值**
5. **Skill 最适合标准化重复任务：bugfix、接口开发、重构、PR review**
6. **后端最该让 Claude Code 干的是“执行链条”，不是“替你想业务”**
7. **涉及核心事务、schema、强隐式规则的地方，先 plan 再改**

---

## 最后总结

如果你问我：**Claude Code 值不值得后端开发用？**

答案是值得，但别把它当成“会自动写代码的神奇机器人”。更准确的理解应该是：

> Claude Code 最适合做后端研发里的工程执行工作：读代码、补测试、跑命令、解释风险、推动一个任务从分析到落地。

真想把它用好，重点不是背多少命令，而是把下面几件事做好：

- 任务说清楚
- 边界说清楚
- 验证方式说清楚
- 用 `CLAUDE.md` 固化团队约束
- 把高频任务沉淀成可复用的 skill 或模板

做到这一步，它就不是一个“回答问题的 AI”，而是一个真正能在后端项目里干活的执行助手。
