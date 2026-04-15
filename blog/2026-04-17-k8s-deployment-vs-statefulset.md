---
slug: k8s-deployment-vs-statefulset
title: Deployment 和 StatefulSet 到底怎么选
authors: [xiaoqu]
tags: [cloud-native, architecture]
---

Kubernetes 里最常见的一个问题就是：

**无状态服务用 Deployment，那有状态服务是不是就一定要用 StatefulSet？**

答案通常是：**大多数时候，是。** 但别把它理解成教条，更好的方式是先看两者到底在解决什么问题。

## Deployment 解决的是什么

Deployment 擅长管理“可替换”的副本。

比如：

- Web API
- 前端服务
- 网关
- 普通 Worker

这类应用通常有几个特点：

- Pod 名字不重要
- 先删一个再起一个问题不大
- 不依赖固定网络身份
- 不依赖固定磁盘身份

所以 Deployment 的核心价值就是：

- 滚动更新
- 副本扩缩容
- 自动恢复
- 发布回滚

## StatefulSet 解决的是什么

StatefulSet 解决的是“副本之间不能完全互换”的场景。

典型例子：

- MySQL / PostgreSQL
- Kafka
- ZooKeeper
- Elasticsearch
- Redis Sentinel / Cluster 的某些部署方式

这类服务通常需要：

- 稳定的 Pod 标识
- 稳定的网络身份
- 稳定的持久化存储
- 有顺序的启动和停止

StatefulSet 给你的关键能力是：

### 1. 固定命名

它的 Pod 名通常是：

- `app-0`
- `app-1`
- `app-2`

编号是稳定的，不会像 Deployment 那样每次重建都随机变。

### 2. 稳定网络身份

配合 Headless Service，Pod 可以拥有稳定 DNS 名称。

这对很多集群内部通信很关键。

### 3. 独立存储

每个 Pod 都能绑定自己的 PVC，不会因为 Pod 漂移就丢失身份。

## 一个实用判断法

你可以问自己三个问题：

1. 这个副本能不能随便替换？
2. 它需不需要固定名字和网络身份？
3. 它需不需要独占持久化数据？

如果三个里有两个答案是“需要”，那大概率就该看 StatefulSet 了。

{/* truncate */}

## 别被名字骗了

StatefulSet 不等于“自动帮你搞定所有有状态问题”。

它只是帮你提供稳定身份和部署顺序，
**并不会自动帮你做数据一致性、主从切换、备份恢复、脑裂处理。**

所以选型时要记住：

- Deployment 解决“副本管理”
- StatefulSet 解决“有身份的副本管理”
- 真正的数据正确性，还是得靠应用本身或运维体系兜底
