---
slug: k8s-resource-requests-limits
title: requests 和 limits 别再背了，直接按资源竞争来理解
authors: [xiaoqu]
tags: [cloud-native, architecture]
---

Kubernetes 里 `requests` 和 `limits` 是新手几乎必撞的一堵墙。

很多教程会直接说：

- `requests` 是请求值
- `limits` 是上限值

这句话没错，但不够“能用”。

更有用的理解是：**它们定义了你在资源竞争中的座位和天花板。**

## requests 是什么

`requests` 主要影响调度。

比如你给一个 Pod 写：

- CPU request: `500m`
- Memory request: `512Mi`

调度器在选节点时，会先看这个节点“账面上”还有没有足够资源容纳它。

所以 `requests` 可以理解成：

> 这个 Pod 至少需要这么多资源，才能有资格被安排进去。

它不代表一定时刻占满这些资源，但调度时会按这个数算账。

## limits 是什么

`limits` 更像运行期约束。

### CPU limit

CPU 超了通常会被 throttling，也就是被限速，而不是直接杀掉。

### Memory limit

内存超了风险更大，通常可能触发 OOMKill。

所以：

- CPU 超限：常见表现是慢
- 内存超限：常见表现是死

## 为什么不能瞎配

如果 `requests` 配太低：

- 调度看起来很省
- 实际节点可能很挤
- 高峰时容易互相抢资源

如果 `limits` 配太死：

- CPU 可能被卡得很难受
- 内存稍一抖动就 OOM

如果什么都不配：

- 多租户环境里容易变得不可控
- 资源画像很难做
- 容量规划基本靠猜

## 一个很实用的经验

先别追求“一次配准”。

更靠谱的节奏是：

1. 先根据经验给一个保守初值
2. 结合监控看真实使用情况
3. 持续调整 requests
4. 谨慎收紧 limits

{/* truncate */}

## 推荐理解法

把它想成办公室分配：

- `requests`：先给你留座位
- `limits`：告诉你最多能占多大地盘

在资源紧张时，谁有稳定座位、谁会不会被强行赶走，体验完全不一样。

## 一句话总结

如果你的集群老是“明明资源很多却调度失败”或者“节点经常爆掉”，
那就别只盯着应用代码了，先看看 requests / limits 到底是不是配得像在碰运气。
