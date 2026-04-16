---
slug: kubernetes-knowledge-map-and-learning-path
title: Kubernetes 知识地图：从集群架构到安全治理的一份精确主线
authors: [xiaoqu]
tags: [cloud-native, architecture, troubleshooting, internals]
---

Kubernetes 的资料很多，但真正难的不是“找不到”，而是**信息过载、概念混杂、层次不清**。

这篇文章不追求百科全书式罗列，而是给出一份更适合工程实践的主线：

- Kubernetes 到底解决什么问题
- 集群内部核心组件怎么协作
- Pod、Deployment、StatefulSet、Service、Ingress 各自扮演什么角色
- 存储、资源管理、安全控制分别落在什么边界
- 一名工程师应该按什么顺序建立 K8s 心智模型

如果你想真正把 K8s 学成“能做判断的系统”，而不是只会背 YAML，这篇可以当总纲。

{/* truncate */}

## 一、先别背对象，先理解 Kubernetes 的本质

Kubernetes 本质上不是“容器运行工具集合”，而是一个**声明式分布式控制系统**。

更精确地说，它围绕三件事工作：

1. **声明期望状态（Desired State）**：你提交 YAML，告诉系统“我想要什么”
2. **观测实际状态（Actual State）**：集群实时感知当前资源处于什么状态
3. **持续控制收敛（Control Loop）**：控制器不断把实际状态推回期望状态

所以 Kubernetes 的底层思维不是“执行一次命令”，而是：

> 你声明目标，系统持续逼近目标。

这也是为什么 K8s 的很多对象看起来“啰嗦”，但可自动恢复、可滚动升级、可持续运维。

## 二、Kubernetes 的宏观架构：控制面与数据面

先建立最重要的全局图。

### 1. Control Plane（控制面）

控制面负责**接收声明、存储状态、做决策、驱动收敛**。

核心组件通常包括：

- **kube-apiserver**：集群统一入口，所有对象读写都先经过它
- **etcd**：保存集群状态的键值存储，属于事实来源（source of truth）
- **kube-scheduler**：决定新 Pod 应该落到哪个 Node
- **kube-controller-manager**：运行大量控制器，把对象状态拉回期望值
- **cloud-controller-manager**（可选）：把云厂商能力接入集群

### 2. Worker Node（数据面 / 工作节点）

工作节点负责**真正运行 Pod**。

典型组件包括：

- **kubelet**：节点代理，负责把 Pod 规格落实成实际运行状态
- **container runtime**（如 containerd）：真正创建和管理容器
- **kube-proxy**：维护 Service 对应的流量转发规则（实现因环境而异）

### 3. 一条关键链路

可以把一次应用发布简化理解为：

`kubectl/apply -> API Server -> etcd -> Controller/Scheduler -> kubelet -> container runtime`

这条链路里最值得记住的一点是：

- **API Server 是统一入口**
- **etcd 是状态存储**
- **Controller/Scheduler 是决策者**
- **kubelet/runtime 是执行者**

## 三、工作负载模型：Pod 是原子单位，不是 Deployment

很多人一上来就用 Deployment，于是误以为 Deployment 是最小运行单元。

不是。

### 1. Pod 是最小调度与运行单元

Pod 是 Kubernetes 调度、网络、存储挂载的基本单位。

一个 Pod 可以包含一个或多个容器，这些容器：

- 共享网络命名空间
- 可以共享存储卷
- 作为一个整体被调度到同一个 Node

所以更准确的理解是：

> 容器是进程级运行实体，Pod 才是 K8s 世界里的部署原子。

### 2. 为什么不直接调度单个容器

因为很多容器需要天然协作，例如：

- 主业务容器 + sidecar 日志/代理容器
- 共享 localhost 通信
- 共享卷与生命周期

Pod 提供了比“单容器”更稳定的编排边界。

## 四、Deployment 与 StatefulSet：不是“有状态/无状态”那么粗糙

这是 K8s 初学者最容易被讲坏的一组概念。

### 1. Deployment 适合什么

Deployment 管理的是一组**可替换、可滚动升级**的 Pod。

它背后通过 ReplicaSet 保证副本数，并支持：

- 滚动发布
- 回滚
- 扩缩容
- 暂停 / 恢复发布

适合：

- Web 服务
- API 服务
- 大多数无本地身份要求的应用

它的核心前提是：

> 副本之间是“近似等价、可以替换”的。

### 2. StatefulSet 适合什么

StatefulSet 适合那些需要**稳定身份、稳定网络名、稳定存储绑定**的应用。

它提供的关键能力是：

- 固定序号（如 `web-0`、`web-1`）
- 稳定 DNS 身份
- 每个 Pod 独立绑定自己的 PVC
- 有序创建 / 删除 / 更新

适合：

- 数据库集群
- 消息队列
- 需要稳定节点身份的分布式系统

### 3. 更精确的区分方式

不要只问“它是不是有状态”。

要问：

- Pod 副本是不是可以互换？
- 每个副本是否需要稳定身份？
- 每个副本是否需要独立持久卷？
- 更新时是否必须有序？

如果答案大多是“需要”，StatefulSet 往往更合适。

## 五、服务发现与流量入口：Service 解决抽象，Ingress 解决入口

### 1. Service 解决什么

Pod 是会漂移的：

- Pod 会重建
- IP 会变化
- 副本会增加或减少

如果调用方直接依赖 Pod IP，系统会非常脆弱。

Service 的价值是：

- 给一组 Pod 提供稳定访问入口
- 通过 label selector 动态绑定后端
- 把“谁在提供服务”从调用方视角抽象掉

常见类型：

- **ClusterIP**：集群内访问
- **NodePort**：通过节点端口暴露
- **LoadBalancer**：借助云负载均衡对外暴露
- **Headless Service**：不做虚拟 IP，常给 StatefulSet 提供稳定 DNS 身份

### 2. Ingress 解决什么

Ingress 主要解决 **HTTP/HTTPS 七层入口治理**：

- 域名路由
- 路径转发
- TLS 终止
- 统一入口

它本身只是 API 对象，真正执行这些逻辑的是 Ingress Controller。

所以更准确地说：

> Ingress 是声明规则，Ingress Controller 才是数据面实现。

## 六、网络模型：Pod 间直连是假设，网络策略是边界

Kubernetes 默认网络模型的核心假设是：

- Pod 应该能直接互相通信
- 每个 Pod 都有自己的 IP
- 节点与 Pod、Pod 与 Pod 的通信要尽量统一抽象

但默认“能通”不等于默认“安全”。

### 1. CNI 决定底层连通实现

K8s 不内置具体网络实现，而是通过 **CNI 插件**接入。

这意味着：

- Pod IP 如何分配
- 跨节点网络如何打通
- 是否支持 NetworkPolicy

都与具体网络插件强相关。

### 2. NetworkPolicy 是四层访问控制，不是万能防火墙

NetworkPolicy 主要控制 L3/L4 流量（IP / TCP / UDP / SCTP）。

几个很关键的理解点：

- 它依赖网络插件支持，写了策略不代表一定生效
- 默认没有策略时，命名空间里通常是全放通
- ingress / egress 隔离是分别生效的
- 最终是否允许通信，要同时看源端 egress 和目标端 ingress
- default deny egress 往往也会把 DNS 一起拦掉

所以生产环境里，NetworkPolicy 不应被当作“锦上添花”，而应是租户隔离与最小暴露的一部分。

## 七、存储模型：PVC 不是磁盘本身，而是声明

K8s 的存储概念也经常被讲得很混乱。

### 1. 三个核心对象

- **PV（PersistentVolume）**：底层存储资源
- **PVC（PersistentVolumeClaim）**：工作负载对存储的请求
- **StorageClass**：动态供给存储的模板与策略

更精准的关系是：

> Pod 不直接“拿磁盘”，而是通过 PVC 声明需求，再由系统把 PVC 绑定到 PV。

### 2. 为什么 StatefulSet 常和 PVC 绑定出现

因为有状态应用通常要求：

- 每个副本有自己的独立数据目录
- Pod 重建后还能挂回原来的数据卷

StatefulSet 的 `volumeClaimTemplates` 正是为这种模式设计的。

### 3. 必须记住的一个边界

删除 StatefulSet 或缩容，不等于自动删除关联卷。

这是为了优先保护数据安全，而不是自动清理干净。

## 八、资源管理：requests 决定“能不能上车”，limits 决定“最多占多少”

Kubernetes 的资源管理不能只背定义，必须和调度、争抢、故障联系起来理解。

### 1. requests 的本质

`requests` 主要影响调度。

调度器会根据 Pod 的 CPU / Memory requests 判断某个 Node 是否有足够可分配资源容纳它。

所以：

> requests 更像“入场门槛”和“保底座位”。

### 2. limits 的本质

`limits` 是运行期约束。

- CPU 超限，通常是被 throttling（限速）
- Memory 超限，常见结果是 OOMKill

所以很多线上问题其实是：

- CPU limit 配得过死，系统一直慢
- Memory limit 配得过紧，容器频繁被杀
- requests 配得太低，调度看起来省，运行时却互相争抢

### 3. 进一步要学的两个对象

- **LimitRange**：给命名空间内资源配置设默认值或边界
- **ResourceQuota**：限制命名空间总体资源与对象数量

它们决定的是“多租户治理能力”，不只是单个 Pod 参数。

## 九、安全模型：K8s 的安全不是一个开关，而是一组分层控制

Kubernetes 安全最容易被误解的点，是大家总想找一个“总开关”。

实际上它是分层的。

### 1. 身份与访问控制

最核心的是：

- **Authentication**：你是谁
- **Authorization**：你能做什么
- **Admission**：你即将提交的对象是否符合策略

### 2. RBAC 的常识与高风险点

RBAC 的核心原则是 **least privilege（最小权限）**。

一些必须记住的实践：

- 尽量优先使用 namespace 级 Role / RoleBinding
- 避免随手给 `cluster-admin`
- 避免通配符权限 `*`
- 不要把用户随便加进 `system:masters`
- 定期审计高权限 ServiceAccount 与绑定关系

而且 RBAC 里有一些权限不是“普通权限”，而是**潜在提权入口**，例如：

- 读取 / 列出 Secret
- 创建 workload（可间接拿到 Secret、ServiceAccount 权限）
- `bind` / `escalate` / `impersonate`
- 控制 admission webhooks
- 创建 `serviceaccounts/token`
- 拿到 `nodes/proxy`

这类权限必须按“高危面”理解，而不能只看字面意思。

### 3. Pod 级安全

除了 RBAC，还要关注：

- Pod Security 标准 / Admission
- 容器是否特权运行
- 是否允许 hostPath
- 是否默认挂载 ServiceAccount token
- 镜像来源与供应链可信度

一句话说，**K8s 安全的关键不是会建 Secret，而是知道哪些能力会把边界直接打穿。**

## 十、配置管理：ConfigMap 和 Secret 只是起点，不是终点

配置管理要解决的不是“把值塞进去”，而是：

- 配置是否与代码解耦
- 敏感值是否分层治理
- 配置变更是否有发布路径

### 1. ConfigMap 适合什么

适合：

- 非敏感参数
- 应用开关
- 配置文件片段
- 环境差异配置

### 2. Secret 适合什么

适合：

- 密码
- Token
- API Key
- TLS 证书材料

但要清楚：

> Secret 不是“自动绝对安全”，默认也不等于强加密治理完成。

如果密钥重要度更高，通常还要引入外部密钥系统（Vault、云 KMS 等）。

## 十一、真正的专家路线：从“会写 YAML”走向“能解释为什么”

我更推荐按下面的顺序学，而不是东一块西一块地看。

### 第一阶段：建立系统观

先搞清楚：

1. 控制面 / 数据面各自负责什么
2. API Server、etcd、Scheduler、Controller Manager、kubelet 的职责边界
3. 声明式 API + 控制循环为什么是 Kubernetes 的核心

### 第二阶段：掌握工作负载与流量模型

重点吃透：

1. Pod 的本质
2. Deployment / ReplicaSet / StatefulSet 的差异
3. Service / Ingress / Headless Service 的角色
4. 探针、滚动更新、回滚、扩缩容怎么影响可用性

### 第三阶段：补齐资源、存储与调度

要能说清：

1. requests / limits 的运行后果
2. PVC / PV / StorageClass 的绑定关系
3. 调度器为什么把 Pod 放到某台机器
4. 亲和性、反亲和、污点容忍解决什么问题

### 第四阶段：补齐安全与治理

至少要系统理解：

1. RBAC
2. ServiceAccount
3. Secret 风险
4. Pod Security
5. NetworkPolicy
6. 配额与租户治理

### 第五阶段：进入源码与控制器思维

如果要真正往“专家”走，迟早要回到下面这些：

- API 对象定义
- Informer / List-Watch / Local Cache
- Controller reconcile 模式
- kube-scheduler 的决策流程
- kubelet 如何把 PodSpec 落实成运行状态

也就是说，最终你要从“对象使用者”升级成“控制系统理解者”。

## 十二、一份可以长期维护的 K8s 判断框架

以后遇到一个新问题，可以先按这几个维度拆：

### 1. 这是控制面问题还是数据面问题？

- API Server / etcd / scheduler / controller 侧？
- kubelet / runtime / 网络 / 存储侧？

### 2. 这是声明问题还是收敛问题？

- YAML 写错了？
- 控制器没收敛？
- 调度条件不满足？
- 节点执行失败？

### 3. 这是对象抽象问题还是实现细节问题？

- Service 抽象没理解？
- 还是 kube-proxy / CNI 实现差异导致？

### 4. 这是可用性问题、性能问题，还是安全问题？

- 发布滚动太慢？
- 资源限制导致抖动？
- 权限边界过宽？
- 网络默认全通？

当你能持续用这个框架拆问题，K8s 就会从“概念集合”变成“可推理系统”。

## 十三、接下来我会继续沉淀哪些专题

这篇之后，建议持续扩展成几条明确主线：

1. **架构主线**：API Server、etcd、Controller、Scheduler、kubelet
2. **工作负载主线**：Pod、Deployment、StatefulSet、Job、DaemonSet
3. **流量主线**：Service、Ingress、Gateway、DNS、NetworkPolicy
4. **存储主线**：PV、PVC、StorageClass、CSI、卷故障排查
5. **资源主线**：requests / limits、QoS、HPA、VPA、Quota
6. **安全主线**：RBAC、ServiceAccount、Secret、Pod Security、Admission
7. **排障主线**：Pending、CrashLoopBackOff、NotReady、DNS、网络、存储
8. **源码主线**：Informer、Reconcile、调度框架、kubelet 执行链路

后面这些专题会逐步把这个博客从“会写几篇 K8s 文章”，推进到“形成一套像样的 Kubernetes 知识体系”。

## 参考资料

这篇文章的主线主要基于以下高可信来源整理：

- Kubernetes 官方文档：Concepts / Overview / Architecture
- Kubernetes 官方文档：Pods / Deployments / StatefulSets
- Kubernetes 官方文档：Service / NetworkPolicy / Persistent Volumes
- Kubernetes 官方文档：Manage Resources for Containers
- Kubernetes 官方文档：Security Overview / RBAC Good Practices
- 当前仓库内已有的 K8s 文章与源码阅读导引

如果你只记一句话：

> Kubernetes 不是一堆 YAML，而是一套围绕“声明、观察、收敛”运行的分布式控制系统。

一旦抓住这句，后面很多对象就都不再是死知识。