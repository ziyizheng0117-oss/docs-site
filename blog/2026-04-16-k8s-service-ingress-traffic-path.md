---
slug: k8s-service-ingress-traffic-path
title: 从 Service 到 Ingress：一条请求在 Kubernetes 里怎么走
authors: [xiaoqu]
tags: [cloud-native, architecture]
---

很多人第一次接触 Kubernetes 网络时，会被这些名词绕晕：

- Pod
- Service
- Ingress
- Ingress Controller

其实最好理解的办法不是死记定义，而是顺着一条请求真的走一遍。

## 先从 Pod 开始

应用最终还是跑在 Pod 里。

假设你有一个 Web 服务，监听 `8080` 端口。Pod 本身能提供服务，但 Pod IP 不稳定，重建后可能就变了，所以通常不会让外部直接依赖 Pod IP。

## Service 解决什么问题

Service 的作用，本质上是给一组 Pod 一个**稳定入口**。

比如你定义一个 `ClusterIP` 类型的 Service：

- 它有固定虚拟 IP
- 它有固定 DNS 名字
- 它能把流量转发给匹配标签的后端 Pod

这时候请求路径可以理解成：

> Client in cluster → Service → 某个后端 Pod

所以 Service 解决的是**服务发现和负载分发**。

## Ingress 解决什么问题

如果你只是集群内部访问，Service 通常就够了。

但如果你想让外部用户通过域名访问，比如：

- `api.example.com`
- `blog.example.com`

那就需要一层 HTTP/HTTPS 入口规则，这就是 Ingress 的角色。

Ingress 自己不是流量转发程序，它更像一份“路由规则声明”：

- 哪个域名进来
- 哪个路径匹配
- 转发到哪个 Service

## 真正干活的是 Ingress Controller

这个很关键。

**Ingress 只是规则对象，真正处理流量的是 Ingress Controller。**

常见实现有：

- NGINX Ingress Controller
- Traefik
- HAProxy Ingress
- 云厂商自带控制器

所以一条外部请求大致会这样走：

> Browser → 云负载均衡 / NodePort → Ingress Controller → Service → Pod

## 一个最常见的误区

很多人以为配置了 Ingress，流量就能自动进来。

其实如果你没有安装 Ingress Controller，Ingress 规则只是“写在那儿”，没人执行。

{/* truncate */}

## 怎么判断该用什么

- **集群内调用**：通常先用 Service
- **外部 HTTP/HTTPS 暴露**：通常用 Ingress
- **四层 TCP/UDP 暴露**：可能看 LoadBalancer / NodePort / Gateway / 特定控制器

## 一句话总结

如果把 Kubernetes 网络想简单一点：

- Pod：真正跑应用的地方
- Service：给 Pod 集合一个稳定入口
- Ingress：定义域名和路径级别的访问规则
- Ingress Controller：真正把规则变成流量转发的人

这条链路一通，后面再看网关、证书、灰度发布就不容易懵了。
