---
slug: claude-code-for-backend-dev
title: Claude Code 后端实战：常用命令、Prompt 模板和 CLAUDE.md 配置建议
authors: [xiaoqu]
tags: [backend, troubleshooting, architecture]
---

如果一篇 Claude Code 分享里只有“AI 正在改变软件开发方式”这种话，后端开发大概率看两段就关了。

大家真正关心的是更实际的问题：**平时到底输什么命令、prompt 怎么写、CLAUDE.md 怎么配、什么场景最省时间、什么场景别乱用。**

这篇就只讲这些能直接落地的东西，默认场景是 Java / Spring Boot / MySQL / Redis / MQ 这类常见后端项目。

{/* truncate */}

## 先说结论：后端开发里最有用的不是“会写代码”，而是“会完成一条执行链”

Claude Code 真正适合后端的地方，不是补全几行代码，而是把下面这些动作串起来：

- 读代码
- 找调用链
- 看配置
- 改实现
- 补测试
- 跑命令
- 看报错
- 根据结果继续修

对后端来说，这种连续执行能力比“单次回答质量”更重要。

因为很多真实工作都不是一句“帮我写个函数”，而是这种任务：

- 把某个查询接口补上筛选条件和分页测试
- 修一个 token 过期后偶发 500 的 bug
- 把一个过胖的 service 拆成几个清晰方法
- 梳理订单创建链路中的事务边界
- 看看某个慢接口有没有 N+1 查询和串行 RPC

这类任务，本来就不是一个回答能解决的，而是需要一整段工程执行。

---

## 一、后端最常用的 Claude Code 命令，其实就这几个

很多人一听“命令”就以为有一大堆要记。其实日常高频用法非常少，真正常用的是下面这些。

## 1. 在项目里直接启动

```bash
cd your-project
claude
```

这是最基本的入口。

适合：

- 先看代码库结构
- 直接做小改动
- 跟着当前项目上下文连续聊

如果任务不复杂，这个就够了。

---

## 2. 先只分析，不直接修改：Plan Mode

```bash
claude --permission-mode plan
```

这在后端里特别有用。

因为很多后端任务不适合一上来就改，比如：

- 登录、支付、库存、账务
- 涉及事务边界的改动
- 跨多个模块的重构
- 带数据库 schema 风险的变更
- 线上问题排查

这时候更稳的方式是先让它分析：

```text
先阅读 order 模块，梳理创建订单的主调用链、事务边界、库存校验和 MQ 发送时机。
先不要改代码，给我一个修复/改造方案。
```

这个模式最大的价值是：**先拿方案，再决定要不要动手。**

---

## 3. 做一次性的只读分析

```bash
claude --permission-mode plan -p "Analyze the authentication system and suggest improvements"
```

这个特别适合快速看一个问题，不想进长会话时用。

比如：

```bash
claude --permission-mode plan -p "阅读 src/auth 和 src/user，梳理登录链路、token 校验、token 刷新和异常处理路径"
```

适合：

- 临时看一个模块
- 快速拿一个分析结果
- 避免 Claude 直接改代码

---

## 4. 把日志、diff、grep 结果直接喂给它

这个对后端非常实用。

```bash
cat error.log | claude
```

```bash
git diff | claude
```

```bash
rg "createOrder|deductStock|sendMessage" src | claude
```

为什么这招值钱？

因为后端很多问题本来就依赖这些中间材料：

- 错误栈
- 日志片段
- SQL 输出
- diff 变更
- grep 出来的调用点

比起你手动总结一段模糊描述，直接喂原始材料通常更有效。

---

## 5. 在会话里让它做工程任务，而不是让它“写代码”

进入 `claude` 后，最实用的不是花哨命令，而是这种话：

```text
找到处理用户登录的相关文件，解释 controller 到 service 到 repository 的调用链。
```

```text
修复这个空指针问题，先写失败测试复现，再修代码并跑测试。
```

```text
参考现有 AdminUserController 的风格，加一个分页查询接口。
```

核心不是语法，而是任务定义方式。

---

## 二、后端开发最值得收藏的 Prompt 模板

比命令更重要的是 prompt。

同一个 Claude Code，有的人觉得很好用，有的人觉得一般，差别往往就在这里。

---

## 1. 读模块 / 梳理链路

适合：刚接手一个模块、接线上问题、看老代码。

```text
阅读 payment 模块相关代码，帮我梳理：
1. 入口 controller
2. 核心 service
3. repository / mapper
4. 调用了哪些外部依赖（DB、Redis、MQ、RPC）
5. 哪些地方有事务边界
最后给我一个从请求进入到订单落库的主链路说明。
```

这个 prompt 的价值是把“随便看看代码”变成了“输出结构化理解结果”。

---

## 2. 修 bug

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

这类 prompt 的重点是：

- 先复现
- 再修复
- 跑验证
- 输出原因

这比一句“帮我修 bug”强得多。

---

## 3. 做中小型重构

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

后端重构最怕额外改动，所以边界一定要写清楚。

---

## 4. 新增接口

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

这已经很接近真实后端需求了。

---

## 5. 做性能 review / 看慢接口

适合：RT 高、慢 SQL、串行依赖太多。

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

这种 prompt 对性能分析很实用，因为它把“看看有没有问题”变成了明确检查项。

---

## 三、比 Skill 更通用、也更推荐先配的是 CLAUDE.md

很多人会先问 skill 怎么写。

我的建议是：**先把 `CLAUDE.md` 配好，再谈 skill。**

因为 `CLAUDE.md` 是项目级说明书，Claude Code 进入项目时会自动读。对后端项目尤其有价值，因为你可以把团队约束直接固化进去。

最值得放进去的内容包括：

- 项目结构说明
- 构建和测试命令
- 常用模块和职责
- 代码风格约束
- 明确不能乱动的地方
- 提交前必须做的验证

---

## 一个后端项目里的 CLAUDE.md 示例

下面这个例子可以直接放到博客里。

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

这个文件的本质不是“写给 AI 看”，而是把团队的工程习惯结构化。

写得越清楚，Claude Code 越不容易在后端项目里乱发挥。

---

## 四、如果要讲 Skill，后端最值得做成模板的是哪几种

Skill 适合做那些“重复发生、处理步骤比较固定”的任务。

如果你在博客里要讲 skill，我建议别讲得太抽象，直接讲下面这几类。

---

## 1. bugfix skill

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

这类 skill 对后端团队特别实用，因为 bugfix 的套路其实很稳定。

---

## 2. api-implementation skill

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

这类重复活标准化以后，后端使用体验会很好。

---

## 3. refactor skill

适合：

- 抽公共逻辑
- 去重复代码
- 方法拆分
- 迁移旧模式

推荐约束：

- 保持行为不变
- 不改接口契约
- 小步改动
- 每步都可验证

---

## 4. code-review / risk-review skill

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

这个特别适合后端团队做成固定套路。

---

## 五、一个真实后端工作流示例：修 token 过期后接口 500

这部分非常适合放到博客里，因为它比讲概念更能说明问题。

### 第一步：先用 Plan Mode 看链路

```bash
claude --permission-mode plan
```

输入：

```text
阅读 src/auth 下相关代码，梳理登录、token 校验、token 刷新、异常处理路径。
我怀疑 token 过期后某个分支没有正确处理，导致 500。
先不要改代码，先给我定位路径和修复方案。
```

Claude 这一步最适合做的是：

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

这里最重要的是“先写失败测试”。

因为后端修 bug 最怕的是：

- 看起来修了
- 实际只是绕过去了
- 下次换个输入又炸

---

### 第三步：让它输出变更说明

```text
总结：
1. 根因是什么
2. 改了哪些文件
3. 为什么这样修
4. 还有哪些潜在风险
```

这个收尾动作很重要，因为它能把“改完了”变成“可复核、可交接、可回顾”。

---

## 六、后端开发里，什么场景不建议直接让 Claude Code 硬改

Claude Code 很实用，但也别神化。

下面这些场景，我建议慎用直接改代码：

### 1. 资金、库存、账务等核心事务路径

规则如果你自己都没讲清楚，它大概率只能“合理猜测”。这类任务更适合先做方案、梳理逻辑、补测试，不适合直接放手改。

### 2. 高风险 schema 变更

删字段、改主键、改索引、历史数据回填，这些最多让它辅助分析，不建议直接自动推进。

### 3. 业务知识大量隐含在线下经验里

很多后端坑不在代码，而在团队约定。比如某个事件不能重放、某个字段虽然可空但线上脏数据很多、某个状态流转有隐藏前提。这些如果你不告诉它，它不可能自己知道。

---

## 七、给后端团队的几条实用建议

如果你是想在团队里推广 Claude Code，我觉得最值得强调的是下面这些：

1. **最常用的命令其实很少，重点是 Plan Mode**
2. **真正影响效果的不是命令，而是任务定义是否清楚**
3. **优先配好 `CLAUDE.md`，比研究花哨 prompt 更有长期价值**
4. **Skill 最适合标准化重复任务：bugfix、接口开发、重构、PR review**
5. **后端最该让 Claude Code 干的是“执行链条”，不是“替你想业务”**
6. **涉及核心事务、schema、强隐式规则的地方，先 plan 再改**

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
