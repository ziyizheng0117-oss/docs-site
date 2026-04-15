---
slug: k8s-architecture-overview
title: Kubernetes 架构速览：先搞懂控制面，再谈调优
authors: [xiaoqu]
tags: [notes, project]
---

很多人学 Kubernetes，一上来就背一堆资源对象：Pod、Deployment、Service、Ingress……背到最后脑子像一锅粥。

更省力的办法是先理解它的核心结构：**控制面负责“想要什么”**，**节点负责“实际跑什么”**。

## 控制面做什么

控制面最重要的事情，其实就是持续对比：

- 当前集群状态是什么
- 期望状态是什么
- 两者差多少
- 要不要补、删、替换

常见组件可以粗糙地这么理解：

### API Server

它是整个集群的入口。

无论你执行 `kubectl apply -f deployment.yaml`，还是控制器在同步状态，最终都要经过 API Server。

你可以把它理解成 Kubernetes 的“总前台”。

### etcd

这是集群的状态数据库。

很多关键配置和资源定义，最终都保存在 etcd 里。etcd 挂了，控制面就会非常难受，所以生产环境里它通常是重点保护对象。

### Scheduler

它负责决定：**这个 Pod 应该被放到哪台 Node 上。**

它不会直接把 Pod 跑起来，但它会根据资源、亲和性、污点容忍等条件，挑一个最合适的节点。

### Controller Manager

它负责把“期望状态”维持住。

比如你想要 3 个副本，它发现只剩 2 个，就会想办法补回去。Deployment、ReplicaSet 这种“自愈能力”，背后就是控制器在不停干活。

## Node 上做什么

### kubelet

每台 Node 上都有 kubelet。

它负责和控制面对接，并确保本机上该跑的 Pod 真的跑起来。

### Container Runtime

它负责真正启动容器。

现在常见的是 containerd。

### kube-proxy

它负责节点上的网络转发规则，让 Service 能把流量导到后端 Pod。

## 最重要的理解方式

Kubernetes 不是“你发一个命令，它执行一次就结束”的系统。

它更像一个**持续协调的系统**：

- 你声明目标
- 控制面不断逼近目标
- 节点不断汇报结果
- 控制器持续修正偏差

这个思路一旦通了，后面再学 Deployment、HPA、Ingress、StatefulSet 会顺很多。

{/* truncate */}

## 一句话总结

如果你只记一句：

> Kubernetes 的本质，是一个围绕“期望状态”运转的分布式调谐系统。

先把这句话吃透，后面的概念会轻松很多。
