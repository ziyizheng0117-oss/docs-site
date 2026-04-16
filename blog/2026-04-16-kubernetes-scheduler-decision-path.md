---
slug: kubernetes-scheduler-decision-path
title: Kubernetes 调度到底在决策什么：把 Pod 为什么调不上去一次讲透
authors: [xiaoqu]
tags: [cloud-native, architecture, troubleshooting, internals]
---

很多人学 Kubernetes 调度，最后只记住一句：

- scheduler 会给 Pod 找 Node

这句话没错，但几乎没用。

真正有用的问题是：

- 为什么这个 Pod 没被调度上去？
- 为什么它被调到了这台机器，而不是另一台？
- `nodeSelector`、亲和性、污点、资源请求，到底谁先起作用？
- 调度失败时应该先看什么？

这篇就专门把这件事讲透。

{/* truncate */}

## 一、先把调度理解成一个“筛选 + 打分”过程

Kubernetes 默认调度器 `kube-scheduler` 的核心工作，不是“随便挑一台机器”，而是两步：

1. **Filtering（过滤）**：哪些 Node 根本不满足条件，先淘汰
2. **Scoring（打分）**：剩下可选节点里，谁更合适

所以更精确地说：

> 调度不是先选“最好”的节点，而是先找“可行节点”，再从中选“更优节点”。

如果第一步之后没有任何可行节点，Pod 就会保持 Pending / Unschedulable。

## 二、调度器到底在看什么

调度器做决策时，通常会综合看这些维度：

- 资源请求是否满足（CPU / Memory / 扩展资源）
- 节点标签是否匹配
- `nodeSelector` / `nodeAffinity` 是否满足
- Pod 亲和 / 反亲和规则是否满足
- `topologySpreadConstraints` 是否满足
- 污点（taints）是否被容忍（tolerations）
- 卷绑定 / 存储拓扑是否允许
- 是否会造成更差的资源分布或可用性布局

也就是说，调度器不是只看“还有多少空闲资源”，它同时还在执行一套**约束检查 + 布局优化**。

## 三、最容易理解错的一个点：调度器只决定“放哪”，不负责“跑起来”

调度器的职责边界是：

- 为还没绑定 Node 的 Pod 找到一个合适节点
- 把绑定结果写回 API Server

之后真正去拉镜像、创建容器、挂卷、启动进程的，是目标节点上的 **kubelet** 和容器运行时。

所以：

- **调度成功 ≠ 启动成功**
- **Pending 也不一定全是调度问题**

如果一个 Pod 已经绑定了 Node，但后面卡在镜像拉取、卷挂载、探针失败，那已经不是 scheduler 的核心职责了。

## 四、资源请求：为什么 requests 才是调度器真正关心的数字

调度器看资源时，核心看的是 **requests**，不是实时使用量，也不是 limits。

### 1. requests 决定能不能“上车”

如果 Pod 声明了：

- CPU request: `500m`
- Memory request: `1Gi`

那调度器会看每个候选节点是否还有足够的**可分配余量**承接它。

所以：

> `requests` 是调度时的“账面占位”，不是瞬时实际消耗。

### 2. 为什么节点明明看起来不忙，却调不上去

因为调度器关心的是：

- 这个节点的 allocatable 还剩多少
- 已经被其他 Pod 的 requests 占掉多少

而不是 `top` 看起来 CPU 还很闲。

这也是很多人第一次遇到 `0/3 nodes are available: insufficient cpu` 时会困惑的原因。

### 3. limits 对调度不是主开关

`limits` 主要影响运行期，不是调度期。

- CPU limit 太低，运行期可能被 throttling
- Memory limit 太低，运行期可能 OOMKill

但是否能被调度上去，主要还是看 requests。

## 五、nodeSelector 与 nodeAffinity：硬约束和软偏好要分清

### 1. nodeSelector：最简单的硬匹配

`nodeSelector` 就是：

- Pod 只能去那些带有指定 label 的节点

它很直接，但表达能力有限。

### 2. nodeAffinity：更强的节点选择表达式

`nodeAffinity` 本质上也是基于节点标签匹配，但能力更强。

最重要的区别是它分成两类：

- `requiredDuringSchedulingIgnoredDuringExecution`
- `preferredDuringSchedulingIgnoredDuringExecution`

前者是硬约束，后者是软偏好。

#### 硬约束

如果不满足，就不调度。

#### 软偏好

如果满足当然更好；不满足也不是绝对不能调度，只是得分会低。

所以判断一个 Pod 为什么 Pending，先看你写的是 required 还是 preferred，这差别非常大。

## 六、Pod 亲和 / 反亲和：不是“喜欢哪台机器”，而是“喜欢和谁在一起”

节点亲和看的是 **Node 标签**。

Pod affinity / anti-affinity 看的是 **其他 Pod 的标签**。

这就带来两类很常见的场景：

### 1. Pod Affinity

让某个 Pod 倾向于靠近另一类 Pod。

典型场景：

- 两个服务调用很频繁，希望同 zone 部署，减少跨区网络开销

### 2. Pod Anti-Affinity

让某些副本尽量别挤在一起。

典型场景：

- 一个 Deployment 的多个副本尽量分散到不同 Node / Zone，提高可用性

### 3. 为什么它容易把调度搞复杂

因为它不再只检查“节点自己符不符合”，还要看：

- 当前节点上已经跑了哪些 Pod
- 这些 Pod 的标签是什么
- 它们所在拓扑域是什么

所以集群越大，这类规则的调度成本越高，配置也越容易写出意外效果。

## 七、污点与容忍：一个是节点“赶人”，一个是 Pod “我能忍”

这个概念特别容易被讲乱。

### 1. Taint 是加在 Node 上的排斥信号

意思是：

- 这台节点不欢迎普通 Pod

常见 effect：

- `NoSchedule`：不再调度新 Pod 上来
- `PreferNoSchedule`：尽量别调度，但不是绝对禁止
- `NoExecute`：不仅不能新调度，已有 Pod 也可能被驱逐

### 2. Toleration 是加在 Pod 上的“我可以接受”

Pod 有 matching toleration，才有资格被放到被 taint 的节点上。

但注意：

> toleration 只是“允许你不被排斥”，不是“保证你一定被调到那里”。

因为调度器还会继续检查资源、亲和性、拓扑、卷等别的条件。

### 3. 一条很实用的工程经验

如果想把某类节点专门留给某些工作负载，通常不是只写 toleration，最好是：

- **Node 上加 taint**：挡住普通 Pod
- **Pod 上加 toleration**：允许目标工作负载进入
- **再配 node affinity / selector**：确保它优先甚至只去目标节点

否则会出现“它能上专用节点，但也可能跑去普通节点”的尴尬状态。

## 八、Topology Spread：调度不只是能放下，还要尽量分布合理

这是很多人后面才补上的概念。

`topologySpreadConstraints` 解决的是：

- 副本如何尽量均匀分布到不同 Node / Zone / 机架

这和 Pod anti-affinity 有点像，但目标更偏“全局均衡分布”。

典型用途：

- 避免所有副本压在一个可用区
- 避免某台节点上聚集过多同类 Pod
- 让系统在节点故障 / 可用区故障时更抗打

所以调度器不仅在判断“能不能放”，也在优化“放得是否更稳”。

## 九、为什么 Pod 会调度失败：最常见的几类根因

如果一个 Pod 一直 Pending，常见原因通常就集中在下面几类。

### 1. 资源不足

最常见。

比如：

- CPU requests 太高
- Memory requests 太高
- GPU / 扩展资源不够

### 2. 节点选择条件过死

比如：

- `nodeSelector` 指向了根本不存在的标签
- `required` 类型 nodeAffinity 写得太窄

### 3. 污点未容忍

节点有 taint，Pod 没对应 toleration，自然进不去。

### 4. Pod 反亲和 / 拓扑分布约束过严

为了高可用写了分散规则，结果集群规模不够，根本无处可放。

### 5. 存储拓扑或卷绑定限制

某些卷只能在特定 zone/node 拿到，和 Pod 的其他调度约束冲突。

所以排查调度问题，别只看一项，要把这些条件一起看。

## 十、一个够用的排查顺序

如果 Pod 调不上去，我建议按下面顺序看：

### 第一步：看 Pod Events

先看事件里调度器到底报了什么。

例如典型信息：

- insufficient cpu / memory
- node(s) had taint that the pod didn't tolerate
- node(s) didn't match Pod's node affinity
- node(s) didn't satisfy existing pods anti-affinity rules

这一步通常已经能直接缩小范围。

### 第二步：看 requests 是否过高

确认：

- CPU / Memory request 是否明显高估
- 是否声明了稀缺扩展资源

### 第三步：看节点选择条件

检查：

- `nodeSelector`
- `nodeAffinity`
- `schedulerName`
- 是否误绑了特定 Node

### 第四步：看 taints / tolerations

确认节点是否带 taint，Pod 是否有 matching toleration。

### 第五步：看亲和 / 反亲和 / spread 约束

尤其是副本类工作负载，很容易因为规则太“理想化”而导致调度死锁。

### 第六步：看存储与拓扑限制

如果是 StatefulSet、PVC 或特定云盘场景，这一步不能省。

## 十一、一个必须建立的判断框架

以后只要看到调度问题，就问自己三件事：

### 1. 是“没有可行节点”，还是“有多个可行节点但我不理解为什么选了这台”？

- 前者重点看 filtering
- 后者重点看 scoring

### 2. 是资源问题，还是约束问题？

- 资源问题：requests / allocatable / 扩展资源
- 约束问题：selector / affinity / toleration / spread / storage topology

### 3. 是调度问题，还是调度后执行问题？

- 没绑定 Node：重点看 scheduler
- 已绑定 Node 但起不来：重点看 kubelet / runtime / image / volume / probe

这个区分特别关键，不然很容易把后续启动失败误当成 scheduler 问题。

## 十二、一句话记住调度器

如果你只想记一句话，我建议记这个：

> kube-scheduler 不是“找一台空闲机器”，而是在一堆资源、标签、约束、拓扑和策略之间，先筛掉不可能，再从可行解里选一个更优解。

一旦按这个视角理解，Kubernetes 调度就不再神秘了。

## 参考资料

- Kubernetes 官方文档：Kubernetes Scheduler
- Kubernetes 官方文档：Assigning Pods to Nodes
- Kubernetes 官方文档：Taints and Tolerations
- Kubernetes 官方文档：Manage Resources for Containers
- 结合当前博客内已有的 Deployment / StatefulSet / 资源管理文章整理
