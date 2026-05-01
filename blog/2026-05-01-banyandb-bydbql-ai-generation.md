---
slug: banyandb-bydbql-ai-generation
title: BanyanDB BYDBQL 的 AI 自动生成方案：从 MCP Prompt 到生产级 Agent
authors: [xiaoqu]
tags: [banyandb, skywalking, bydbql, mcp, llm, ai-agent]
---

最近看了一下 Apache SkyWalking BanyanDB 里 BYDBQL 的实现，以及仓库里已经出现的 MCP 相关代码。这个方向很有意思：它不是简单地让大模型“写一段 SQL”，而是围绕 BanyanDB 自己的查询语言 BYDBQL，做了一套面向 AI 生成的上下文注入和安全执行机制。

先说结论：

> 当前 BanyanDB 仓库里已经有一个 **MCP Prompt Generator + Query Executor** 的雏形，可以把自然语言转成 BYDBQL，再调用 BanyanDB 执行。但它还不是完整的生产级 NL → BYDBQL Agent。

如果要把它做成真正可靠的 AI 查询助手，关键不在于 prompt 写得多花，而在于四件事：

1. 把 BanyanDB 的 live schema 注入给模型；
2. 让模型只生成受限的只读 BYDBQL；
3. 在执行前做 parse / transform / explain 校验；
4. 基于错误信息做自动修复闭环。

{/* truncate */}

## 1. BYDBQL 是什么

BYDBQL，全称 BanyanDB Query Language，是 BanyanDB 自己的 SQL-like 查询 DSL。

它的目标是用接近 SQL 的形式统一查询 BanyanDB 的几类数据模型：

- `STREAM`：日志、事件、trace span 这类时序元素；
- `MEASURE`：指标、聚合型数值时序数据；
- `TRACE`：分布式追踪数据；
- `PROPERTY`：元数据、KV 信息；
- `TOPN`：针对 measure 的 Top-N 查询。

几个典型例子：

```sql
SELECT * FROM STREAM sw IN default TIME > '-30m'
```

```sql
SELECT region, SUM(latency)
FROM MEASURE metrics IN default
TIME > '-1h'
GROUP BY region
```

```sql
SELECT trace_id
FROM TRACE sw_trace IN group1
TIME > '-30m'
WHERE service_id = 'payment-service'
```

```sql
SHOW TOP 10
FROM MEASURE service_latency IN default
TIME > '-30m'
ORDER BY DESC
```

它看起来像 SQL，但不是通用 SQL。它的 `FROM` 子句里必须显式写出资源类型，比如：

```sql
FROM STREAM xxx IN group
FROM MEASURE xxx IN group
FROM TRACE xxx IN group
FROM PROPERTY xxx IN group
```

这点对 AI 生成很重要。大模型如果只知道“生成 SQL”，很容易写出 BanyanDB 不接受的语句。

## 2. BYDBQL 在 BanyanDB 里的执行链路

核心代码在这些位置：

```text
pkg/bydbql/grammar.go
pkg/bydbql/parser.go
pkg/bydbql/transformer.go
banyand/liaison/grpc/bydbql.go
api/proto/banyandb/bydbql/v1/query.proto
api/proto/banyandb/bydbql/v1/rpc.proto
```

服务入口是：

```go
func (b *bydbQLService) Query(ctx context.Context, req *bydbqlv1.QueryRequest)
```

位于：

```text
banyand/liaison/grpc/bydbql.go
```

整体链路大致是：

```text
HTTP/gRPC /v1/bydbql/query
  ↓
bydbQLService.Query
  ↓
bydbql.ParseQuery(req.Query)
  ↓
生成 Grammar AST
  ↓
Transformer.Transform(ctx, query)
  ↓
转换成原生 protobuf request
  ↓
streamSvc / measureSvc / traceSvc / propertyServer
  ↓
返回 QueryResponse
```

Parser 使用的是：

```text
github.com/alecthomas/participle/v2
```

不是 ANTLR。

顶层 grammar 非常直接：

```go
type Grammar struct {
    Select *GrammarSelectStatement
    TopN   *GrammarTopNStatement
}
```

也就是说，BYDBQL 当前主要分两类：

```text
SELECT ...
SHOW TOP ...
```

## 3. Transformer 才是语义核心

很多人看查询语言会先盯 parser，但 BYDBQL 里更关键的是：

```text
pkg/bydbql/transformer.go
```

Parser 只负责把字符串变成 AST。Transformer 才负责结合 BanyanDB metadata schema，把 AST 变成真正可执行的 protobuf request。

核心方法包括：

```go
Transform
transformStreamQuery
transformMeasureQuery
transformTraceQuery
transformPropertyQuery
transformTopNMeasureQuery
convertSelectCriteria
convertTimeRange
convertTagAndField
convertSelectOrderBy
convertTopNOrderBy
convertAggregation
convertGroupBy
```

这一步会做几类重要事情：

- 校验 group / resource name；
- 从 metadata repo 拉 stream / measure / trace / property schema；
- 判断 SELECT projection 里的列是 tag 还是 field；
- 把 WHERE 转成 BanyanDB 的 criteria tree；
- 把 TIME 转成 `TimeRange`；
- 把 aggregation / group by / order by / limit 转成对应 request 字段。

所以 AI 生成 BYDBQL 的核心难点不是“写出看起来像 SQL 的字符串”，而是：

> 生成出来的 query 必须符合 BanyanDB 当前真实 schema。

如果没有 schema context，模型很容易胡编：

- 不存在的 group；
- 不存在的 measure；
- 不存在的 tag；
- 不能排序的字段；
- 对非数值字段做聚合；
- 用错误的资源类型查询。

## 4. 仓库里已有的 AI 生成方案

和 AI 生成相关的代码主要在：

```text
mcp/
```

关键文件：

```text
mcp/src/query/llm-prompt.ts
mcp/src/query/context.ts
mcp/src/query/validation.ts
mcp/src/server/mcp.ts
```

它的思路不是 BanyanDB server 内置调用某个 LLM API，而是通过 MCP 暴露能力。

整体流程是：

```text
自然语言描述
  ↓
MCP prompt: generate_BydbQL
  ↓
加载 BanyanDB 当前 groups/resources/index rules
  ↓
拼成完整 LLM prompt
  ↓
外部 MCP 客户端 / LLM 生成 JSON
  ↓
得到 { "BydbQL": "SELECT ..." }
  ↓
调用 list_resources_bydbql 执行
```

这是一种比较合理的架构：BanyanDB 负责 schema、query 执行和工具暴露；具体 LLM 由 MCP 客户端负责。

## 5. Prompt 生成：不是一句“帮我写 SQL”

核心函数在：

```ts
export function generateBydbQL(
  description: string,
  args: Record<string, unknown>,
  groups: string[] = [],
  resourcesByGroup: ResourcesByGroup = {},
): string
```

位于：

```text
mcp/src/query/llm-prompt.ts
```

这个 prompt 做了几件很工程化的事情。

### 5.1 注入可用 groups

`mcp/src/query/context.ts` 里有：

```ts
loadQueryContext(banyandbClient)
```

它会调用：

```ts
banyandbClient.listGroups()
```

然后把 group 列表写进 prompt：

```text
Available Groups in BanyanDB:
- default
- metricsMinute
- ...
```

这样模型在生成 `IN group` 时，不至于完全瞎猜。

### 5.2 注入 resource 到 group 的映射

它还会按 group 拉资源：

```ts
listStreams(group)
listMeasures(group)
listTraces(group)
listProperties(group)
listTopN(group)
```

然后生成类似这样的上下文：

```text
MEASUREs:
  "service_cpm_minute" -> Group "metricsMinute"

TRACEs:
  "zipkin_span" -> Group "default"
```

这个设计非常关键。

比如用户只说：

```text
查 service_cpm_minute 最近三天
```

模型可以从 mapping 推断出：

```sql
SELECT * FROM MEASURE service_cpm_minute IN metricsMinute TIME > '-3d'
```

如果没有这个 mapping，它很可能不知道 `service_cpm_minute` 是 measure，也不知道属于哪个 group。

### 5.3 注入可排序 index fields

它还会调用：

```ts
listIndexRule(group)
```

然后筛出：

```ts
!resource.noSort && resource.metadata?.name
```

这些字段会进入 prompt，作为 `ORDER BY` 的候选。

这个点也很重要。BYDBQL 不是所有字段都适合排序，prompt 里专门强调：

> ORDER BY can ONLY use fields that exist in the Available Indexed Fields list.

如果用户说：

```text
按 time 倒序
```

但实际可排序字段叫：

```text
timestamp_millis
```

prompt 会要求模型做相似匹配，生成：

```sql
ORDER BY timestamp_millis DESC
```

如果找不到可用索引字段，则不要加 `ORDER BY`。

## 6. Prompt 里最值得借鉴的规则

我觉得当前 prompt 里最有价值的是几个“防幻觉”规则。

### 6.1 区分时间范围和数据条数

比如：

```text
last 3 days
```

应该生成：

```sql
TIME > '-3d'
```

不能生成：

```sql
LIMIT 3
```

而：

```text
last 30 zipkin spans
```

才应该是：

```sql
ORDER BY timestamp_millis DESC LIMIT 30
```

这类歧义是 NL2SQL / NL2DSL 里非常常见的坑。

### 6.2 不要主动加 ORDER BY / LIMIT

prompt 明确要求：

```text
Do NOT add ORDER BY or LIMIT unless explicitly requested.
```

这能减少模型为了“看起来完整”而乱加排序和限制。

### 6.3 TOPN 单独处理

TopN 不是普通 SELECT + LIMIT，而是专门的：

```sql
SHOW TOP N FROM MEASURE ...
```

prompt 里规定：

- ranking 语义，比如 top / highest / lowest / best / worst + number；
- 资源类型必须是 MEASURE；
- TOPN 里不要加 LIMIT；
- `ORDER BY` 只能是 `ORDER BY DESC` 或 `ORDER BY ASC`，不带字段名。

### 6.4 显式 hint 优先

MCP tool 支持这些可选参数：

```text
resource_type
resource_name
group
```

prompt 要求如果用户显式提供，就优先使用这些值，而不是再从自然语言里猜。

这对产品化很重要，因为 UI 或上游系统经常已经知道用户当前选中的资源上下文。

## 7. 安全与校验

基础校验在：

```text
mcp/src/query/validation.ts
```

关键限制包括：

```ts
allowedQueryPrefixPatterns = [
  /^\s*SELECT\b/i,
  /^\s*SHOW\s+TOP\b/i,
]
```

也就是只允许只读查询：

```text
SELECT
SHOW TOP
```

同时禁止：

```text
;
--
/*
*/
```

避免多语句和注释注入。

还有长度限制：

```text
description <= 2048
BydbQL <= 4096
identifier <= 256
```

以及 identifier pattern：

```ts
/^[A-Za-z0-9][A-Za-z0-9._:-]{0,255}$/
```

所以当前的安全模型是：

```text
MCP 参数校验
  ↓
只允许 SELECT / SHOW TOP
  ↓
禁止多语句和注释 token
  ↓
BanyanDB ParseQuery 语法校验
  ↓
Transformer 结合 schema 做语义校验
  ↓
执行原生查询
```

这已经比“LLM 直接拼字符串然后执行”安全很多。

## 8. 当前方案还缺什么

我认为当前实现更像是一个 AI 查询能力的雏形，还不是生产级 Agent。

主要缺口有几个。

### 8.1 没有自动 dry-run / explain

LLM 输出 BYDBQL 后，最好先经过一个轻量校验接口：

```text
parse only
transform only
explain query
```

现在更多是执行时才暴露 parser / transformer 错误。

更理想的是 BanyanDB server 提供：

```text
POST /v1/bydbql/validate
POST /v1/bydbql/explain
```

返回：

```json
{
  "valid": true,
  "type": "measure",
  "native_request": "...",
  "warnings": [
    "ORDER BY omitted because field is not indexed"
  ]
}
```

这样 AI 可以先验证，不直接扫数据。

### 8.2 Schema context 还不够细

当前 MCP context 主要包含：

- groups；
- resource names；
- resource -> group mapping；
- index rules。

但一个稳定的生成器还需要：

- 每个资源的 tag families；
- fields；
- 字段类型；
- 是否可用于 MATCH；
- analyzer 信息；
- 是否可 aggregation；
- 是否可 group by；
- 是否可 order by。

否则模型还是可能在 `WHERE`、`SELECT`、`GROUP BY`、`AGGREGATE BY` 里瞎写字段。

### 8.3 没有错误驱动自修复

生产级 NL2BYDBQL 不应该一次生成失败就结束。

更好的流程是：

```text
LLM 生成 query
  ↓
BanyanDB validate/explain
  ↓
如果失败，把错误信息 + schema context + 原 query 返回给 LLM
  ↓
LLM 只修正 JSON
  ↓
最多重试 2～3 次
```

常见可修复错误包括：

- resource 不存在；
- group 不存在；
- tag / field 不存在；
- ORDER BY 字段不可排序；
- aggregation 字段类型错误；
- TIME 缺失；
- TOPN 语法错误。

### 8.4 Prompt 需要模块化

现在 prompt 是一个比较长的字符串。后续可以拆成几层：

```text
system rules
BYDBQL grammar summary
schema context
user request
few-shot examples
output JSON schema
```

这样更容易维护，也更容易按资源类型裁剪上下文。

## 9. 我建议的生产级架构

如果要把它做成真正可用的 AI 查询助手，我建议不要让 LLM 一步到位直接写最终 BYDBQL，而是分层。

整体架构：

```text
用户自然语言
  ↓
Intent Parser / Query Planner
  ↓
Schema Retriever
  ↓
LLM Generate BYDBQL JSON
  ↓
Static Validator
  ↓
BanyanDB Parse/Transform Dry Run
  ↓
Auto Repair Loop
  ↓
Execute Query
  ↓
Result Summarizer
```

### 9.1 Intent Parser

先把用户意图结构化：

```json
{
  "resource_type": "measure",
  "resource_name": "service_cpm_minute",
  "group": "metricsMinute",
  "time_range": "-3d",
  "need_limit": false,
  "need_order_by": false,
  "filters": [],
  "aggregation": null
}
```

这一步可以由轻量 LLM 或规则 + LLM 混合完成。

### 9.2 Schema Retriever

根据 intent 精确拉 schema：

```text
groups
resource list
目标 resource 的 tags
目标 resource 的 fields
index rules
field type
analyzer
是否支持 MATCH
是否支持 ORDER BY
```

不要把全库 schema 都塞进 prompt，要按候选资源裁剪。

### 9.3 BYDBQL Generator

输入：

```text
用户原始问题
结构化 intent
目标 schema context
BYDBQL grammar subset
few-shot examples
```

输出固定 JSON：

```json
{
  "BydbQL": "SELECT * FROM MEASURE service_cpm_minute IN metricsMinute TIME > '-3d'",
  "resource_type": "measure",
  "resource_name": "service_cpm_minute",
  "group": "metricsMinute",
  "confidence": 0.86,
  "warnings": []
}
```

### 9.4 Static Validator

在调用 BanyanDB 前做本地校验：

- 只允许 `SELECT` / `SHOW TOP`；
- 禁止多语句；
- 必须有 `FROM` 和 `IN`；
- Stream / Measure / Trace / TopN 默认必须有 TIME；
- ORDER BY 字段必须可排序；
- GROUP BY 字段必须存在；
- aggregation 字段必须是 measure field；
- MATCH 只能用于支持全文检索的字段；
- LIMIT 设置上限，防止大查询。

### 9.5 Server-side validate / explain

建议在 BanyanDB server 侧新增：

```text
/v1/bydbql/validate
/v1/bydbql/explain
```

内部复用现有：

```go
bydbql.ParseQuery
Transformer.Transform
```

但不真正执行 storage query。

### 9.6 Auto Repair Loop

如果 validate / explain 失败，把错误喂回模型：

```text
你的上一条 BYDBQL：...
BanyanDB 返回错误：unknown tag service_name
可用 tags：service_id, service_instance_id, endpoint_id
请只返回修正后的 JSON。
```

最多重试 2～3 次。

这样比单次 prompt 靠谱很多。

## 10. 可以怎么落地

我会把方案拆成两个层级。

### 方案 A：基于现有 MCP 快速增强

优点是改动小。

要做的事情：

1. 扩展 `loadQueryContext`，拉取字段级 schema；
2. 改造 `generateBydbQL` prompt，注入目标资源 fields/tags/index；
3. 在 MCP 里增加 `validate_bydbql` tool；
4. 在 MCP client 侧实现 generate → validate → repair → execute；
5. 给常见 query 类型补 few-shot。

适合先做 PoC。

### 方案 B：BanyanDB 原生 AI Query Gateway

适合产品化。

新增一个服务层：

```text
banyand/liaison/aiquery
```

提供：

```text
POST /v1/ai/bydbql/generate
POST /v1/ai/bydbql/execute
POST /v1/bydbql/validate
POST /v1/bydbql/explain
```

其中 LLM provider 可以做成可插拔：

```text
OpenAI-compatible
Anthropic
本地模型
只返回 prompt，由外部调用
```

但我更倾向先走方案 A，因为 BanyanDB 已经有 MCP 方向，顺着它增强成本最低。

## 11. 关键源码地图

最后列一下这次调研里最关键的文件。

| 文件 | 作用 |
| --- | --- |
| `docs/interacting/bydbql.md` | BYDBQL 语法和语义文档 |
| `pkg/bydbql/grammar.go` | Participle AST / grammar 定义 |
| `pkg/bydbql/parser.go` | lexer/parser 初始化，`ParseQuery` |
| `pkg/bydbql/transformer.go` | AST 到原生 protobuf query request 的转换 |
| `banyand/liaison/grpc/bydbql.go` | BydbQL gRPC 服务入口和执行分派 |
| `mcp/src/query/llm-prompt.ts` | 自然语言生成 BYDBQL 的 prompt 模板 |
| `mcp/src/query/context.ts` | 加载 live schema context |
| `mcp/src/query/validation.ts` | MCP 输入和只读 BYDBQL 校验 |
| `mcp/src/server/mcp.ts` | MCP tools / prompt 注册 |
| `ui/src/components/CodeMirror/bydbql-hint.js` | UI BYDBQL 自动补全，可借鉴 schema hint 逻辑 |
| `ui/src/components/CodeMirror/bydbql-mode.js` | CodeMirror BYDBQL 语法高亮 |

## 结论

BanyanDB 现在已经具备做 AI 自动生成 BYDBQL 的基础：

- 有正式 BYDBQL parser；
- 有 AST 到原生 query request 的 Transformer；
- 有 MCP server；
- 有 prompt generator；
- 有 live group/resource/index context；
- 有基础安全校验。

但要做成生产级能力，还需要补上：

- 字段级 schema context；
- validate / explain dry-run；
- 自动修复循环；
- 更严格的静态 validator；
- 更系统的 few-shot 和 prompt 模块化。

我的判断是：

> 最好的路线不是推倒重做，而是沿着现有 MCP 方案继续增强，把它从“生成 prompt 的工具”升级成“带 schema 检索、校验、修复和执行闭环的 BYDBQL Agent”。
