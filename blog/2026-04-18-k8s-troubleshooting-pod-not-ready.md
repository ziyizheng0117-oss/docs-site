---
slug: k8s-troubleshooting-pod-not-ready
title: Pod 一直 NotReady？一套够用的排查顺序
authors: [xiaoqu]
tags: [notes, project]
---

Kubernetes 出问题时，最常见也最烦人的状态之一就是：

- Pod 在跑
- 但业务不可用
- 一看状态，不是 `CrashLoopBackOff`，而是迟迟不 `Ready`

这种时候最怕的就是东看一眼、西看一眼，最后把自己看乱。

我更推荐一套固定顺序，排查起来不容易漏。

## 第一步：先看事件

先别急着猜，先看事件：

```bash
kubectl describe pod <pod-name> -n <namespace>
```

重点看底部的 Events。

很多问题其实会直接写在这里，比如：

- 镜像拉取失败
- 挂载卷失败
- 探针失败
- 调度失败

## 第二步：看容器日志

如果容器已经启动，马上看日志：

```bash
kubectl logs <pod-name> -n <namespace>
kubectl logs <pod-name> -n <namespace> --previous
```

`--previous` 很重要，尤其是容器反复重启时，不看上一轮日志经常啥也抓不到。

## 第三步：看探针

很多 Pod 不是没跑起来，而是 **readinessProbe 过不了**。

检查这几个点：

- 端口是不是配错了
- 路径是不是配错了
- 服务启动是不是本来就慢
- 探针超时时间是不是太短
- 依赖项没就绪时，应用会不会直接返回失败

如果服务本来启动很慢，可以考虑：

- 调整 `initialDelaySeconds`
- 调整 `timeoutSeconds`
- 调整 `failureThreshold`
- 必要时引入 `startupProbe`

## 第四步：看依赖

有些应用自己没挂，但它依赖的东西没起来：

- 数据库没通
- Redis 没通
- DNS 解析失败
- 配置中心不可达
- Secret / ConfigMap 内容不对

这个时候 Pod 看着“活着”，其实业务根本没准备好。

## 第五步：进容器里验证

如果前面都没定位清楚，就别猜了，直接进去看：

```bash
kubectl exec -it <pod-name> -n <namespace> -- sh
```

进去后重点验证：

- 进程在不在
- 端口有没有监听
- 配置文件是否正确
- 环境变量是否注入
- DNS / 网络是否通

{/* truncate */}

## 一个很实用的思路

遇到 Pod NotReady，别直接问“为什么不行”，而是按层拆：

1. 调度了没有
2. 容器起来了没有
3. 应用启动了没有
4. 探针通过了没有
5. 依赖可用了没有

一层一层剥，比到处乱翻 YAML 快得多。

## 最后一句

K8s 排障最怕的不是问题复杂，而是顺序混乱。

有一套固定动作，很多问题其实十分钟内就能收敛。
