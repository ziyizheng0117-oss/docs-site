---
slug: io-uring-vs-read-write
title: io_uring vs 普通 read/write：一个最小例子和性能差距该怎么理解
authors: [xiaoqu]
tags: [linux, backend, architecture]
---

`io_uring` 这几年很火。

很多文章一上来就会说：

- `io_uring` 更快
- `io_uring` 更现代
- `io_uring` 能吊打传统 `read/write`

这类话不能说全错，但如果你真想搞明白它值不值得用，最好还是回到两个问题：

1. 代码到底怎么写？
2. 性能差距到底是怎么来的？

这篇就用一个最小可跑的思路，拿普通 `read/write` 和 `io_uring` 做个对比。

## 先说结论

如果你只是**顺序读写单个文件**，而且并发不高，`io_uring` 往往不会神奇地快很多。

真正更容易拉开差距的场景通常是：

- 高并发 IO
- 大量小 IO
- 需要减少系统调用开销
- 需要把提交和完成解耦
- 需要同时管理很多文件描述符或网络连接

所以别把它理解成“换个 API 就白捡性能”。

## 普通 read/write 版本

先看最传统的写法。

这个例子做的事情很简单：

- 从输入文件读取数据
- 再写到输出文件
- 循环直到结束

```c
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <errno.h>
#include <string.h>

#define BUF_SIZE 4096

int main(int argc, char *argv[]) {
    if (argc != 3) {
        fprintf(stderr, "Usage: %s <src> <dst>\n", argv[0]);
        return 1;
    }

    int in_fd = open(argv[1], O_RDONLY);
    if (in_fd < 0) {
        perror("open src");
        return 1;
    }

    int out_fd = open(argv[2], O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (out_fd < 0) {
        perror("open dst");
        close(in_fd);
        return 1;
    }

    char buf[BUF_SIZE];
    ssize_t n;

    while ((n = read(in_fd, buf, BUF_SIZE)) > 0) {
        ssize_t written = 0;
        while (written < n) {
            ssize_t m = write(out_fd, buf + written, n - written);
            if (m < 0) {
                perror("write");
                close(in_fd);
                close(out_fd);
                return 1;
            }
            written += m;
        }
    }

    if (n < 0) {
        perror("read");
    }

    close(in_fd);
    close(out_fd);
    return n < 0 ? 1 : 0;
}
```

### 这个版本的特点

优点：

- 简单
- 好懂
- 可移植
- 对很多普通场景已经够用

缺点：

- 每次 `read` / `write` 都会进入内核
- 提交和完成是同步串行的
- 很难优雅地放大并发度

如果你的程序本来就是“读一块、写一块、继续下一块”，那这套模型很自然。

{/* truncate */}

## io_uring 版本

`io_uring` 的核心思路，不再是“我现在立刻发一个系统调用，然后等结果回来”，而更像：

- 我先把要做的 IO 请求放进提交队列
- 内核去处理
- 处理完成后，把结果放到完成队列
- 用户态再统一收割结果

下面这个例子是一个**最小演示版本**，目标不是把所有边角都写满，而是让你看懂思路。

```c
#include <liburing.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define QUEUE_DEPTH  64
#define BLOCK_SIZE   4096

struct io_data {
    off_t offset;
    size_t size;
    char *buffer;
};

int main(int argc, char *argv[]) {
    if (argc != 3) {
        fprintf(stderr, "Usage: %s <src> <dst>\n", argv[0]);
        return 1;
    }

    int in_fd = open(argv[1], O_RDONLY);
    if (in_fd < 0) {
        perror("open src");
        return 1;
    }

    int out_fd = open(argv[2], O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (out_fd < 0) {
        perror("open dst");
        close(in_fd);
        return 1;
    }

    struct io_uring ring;
    if (io_uring_queue_init(QUEUE_DEPTH, &ring, 0) < 0) {
        perror("io_uring_queue_init");
        close(in_fd);
        close(out_fd);
        return 1;
    }

    off_t offset = 0;
    int inflight = 0;
    int eof = 0;

    while (!eof || inflight > 0) {
        while (!eof && inflight < QUEUE_DEPTH) {
            struct io_data *data = malloc(sizeof(*data));
            if (!data) {
                perror("malloc");
                break;
            }

            data->buffer = aligned_alloc(4096, BLOCK_SIZE);
            if (!data->buffer) {
                perror("aligned_alloc");
                free(data);
                break;
            }

            data->offset = offset;
            data->size = BLOCK_SIZE;

            struct io_uring_sqe *sqe = io_uring_get_sqe(&ring);
            if (!sqe) {
                free(data->buffer);
                free(data);
                break;
            }

            io_uring_prep_read(sqe, in_fd, data->buffer, BLOCK_SIZE, offset);
            io_uring_sqe_set_data(sqe, data);

            offset += BLOCK_SIZE;
            inflight++;
        }

        io_uring_submit(&ring);

        struct io_uring_cqe *cqe;
        if (io_uring_wait_cqe(&ring, &cqe) < 0) {
            perror("io_uring_wait_cqe");
            break;
        }

        struct io_data *data = io_uring_cqe_get_data(cqe);
        int ret = cqe->res;
        io_uring_cqe_seen(&ring, cqe);
        inflight--;

        if (ret < 0) {
            fprintf(stderr, "read failed: %s\n", strerror(-ret));
            free(data->buffer);
            free(data);
            break;
        }

        if (ret == 0) {
            eof = 1;
            free(data->buffer);
            free(data);
            continue;
        }

        ssize_t written = 0;
        while (written < ret) {
            ssize_t n = pwrite(out_fd, data->buffer + written, ret - written, data->offset + written);
            if (n < 0) {
                perror("pwrite");
                free(data->buffer);
                free(data);
                goto cleanup;
            }
            written += n;
        }

        free(data->buffer);
        free(data);
    }

cleanup:
    io_uring_queue_exit(&ring);
    close(in_fd);
    close(out_fd);
    return 0;
}
```

## 这个例子要怎么看

这个版本严格来说还是个“教学版”，因为它只把**读**放进了 `io_uring`，写还是用了 `pwrite`。

为什么我故意这么写？

因为很多人第一次看 `io_uring` 就容易被整个状态机绕晕。先把“异步读 + 同步写”看懂，比一上来把读写全塞进 ring 更容易建立直觉。

如果你继续往下做，可以再演进成：

- read completion 之后继续提交 write SQE
- 用 user_data 把读写请求串起来
- 让多个 block 同时在飞
- 最终形成真正的异步 pipeline

## 一个更接近实战的 io_uring 骨架

如果你想把读写都放进 `io_uring`，代码结构通常更像下面这样：

```c
enum op_type { OP_READ, OP_WRITE };

struct io_task {
    enum op_type type;
    off_t offset;
    size_t size;
    char *buf;
};

submit_read(offset):
    task->type = OP_READ
    prep read SQE
    set user_data = task

on_cqe(task):
    if task->type == OP_READ:
        if cqe->res == 0:
            EOF
        else:
            task->type = OP_WRITE
            task->size = cqe->res
            prep write SQE with same buffer and offset
            resubmit
    else if task->type == OP_WRITE:
        recycle buffer
        submit next read if needed
```

这个模型的关键价值在于：

- 读和写都不必阻塞当前线程
- 多个 block 可以同时挂在 ring 里飞
- 你可以明确控制 inflight 深度
- 更适合扩展成高吞吐 pipeline

当然，真实工程代码里你还得继续处理：

- buffer 池复用
- partial write
- 错误恢复
- queue depth 控制
- 文件尾部不足一个 block 的情况

但只要理解了这个状态流，后面就没那么玄学了。

## 配套代码放哪了

我把这篇配套的 benchmark 代码单独放到了：

```text
code/io-uring-bench/
```

目录里有：

- `rw_copy.c`：同步 `read/write` 基线版本
- `uring_copy.c`：读写都走 `io_uring` 的 pipeline 版本
- `Makefile`：直接编译

## 怎么编译

进入目录后：

```bash
cd code/io-uring-bench
make rw_copy
make uring_copy
```

或者直接：

```bash
make
```

如果你的机器上还没装 `liburing`，在 Debian / Ubuntu 上通常是：

```bash
sudo apt-get install liburing-dev
```

## 怎么测性能

最简单的办法就是准备一个足够大的文件，比如 1GB 或 4GB：

```bash
dd if=/dev/zero of=test.bin bs=1M count=1024
```

然后分别测：

```bash
time ./rw_copy test.bin out1.bin
time ./uring_copy test.bin out2.bin
```

如果想看更稳定一点的数据，可以多跑几轮，或者配合：

- `hyperfine`
- `perf stat`
- `iostat`
- `pidstat`

但如果你真想做一个更像样的对比，只跑两次 `time` 其实远远不够。

## 一个更靠谱的 benchmark 设计

我更建议至少把实验拆成下面几组。

### 1. 先固定变量

这些条件最好尽量固定：

- 同一台机器
- 同一块盘
- 同一文件系统
- 同样的编译参数
- 同样的 block size
- 同样的输入文件大小

否则最后你看到的差距，可能根本不是 API 差距，而是环境噪音。

### 2. 分开测缓存命中和真实磁盘 IO

这是最容易把人带沟里的地方。

如果文件已经在 page cache 里，你测到的很多时候其实是：

- 内存拷贝效率
- 系统调用开销
- 页缓存命中后的路径差异

而不是真正的磁盘读写性能。

所以最好至少测两组：

- **warm cache**：文件已经进缓存
- **cold-ish cache**：尽量减少缓存影响

例如在测试间隙做：

```bash
sync
sudo sh -c 'echo 3 > /proc/sys/vm/drop_caches'
```

如果你没法清缓存，也至少要在文章里说明：当前结果更接近 page cache 路径，而不是裸磁盘能力。

### 3. 不只测单文件顺序拷贝

单文件顺序拷贝是最直观的 demo，但不一定是最能体现 `io_uring` 价值的场景。

更建议补几组：

- 单文件顺序读写
- 多文件并发读写
- 大量小块随机 IO
- 不同 queue depth 对比
- 不同 block size 对比

### 4. 记录的不只是耗时

只看总时间太粗了。

更有价值的指标通常包括：

- 吞吐（MB/s 或 GB/s）
- CPU 利用率
- context switch
- page fault
- syscalls 数量
- 磁盘 util / await

例如：

```bash
perf stat ./rw_copy test.bin out1.bin
perf stat ./uring_copy test.bin out2.bin
```

你会更容易看到：差异到底来自 CPU、上下文切换，还是磁盘根本已经打满。

## 一个更像样的测试脚本

如果只是做博客演示，其实可以先用 `hyperfine`：

```bash
hyperfine \
  --warmup 2 \
  --prepare 'rm -f out1.bin out2.bin' \
  './rw_copy test.bin out1.bin' \
  './uring_copy test.bin out2.bin'
```

如果你想更严格一点，可以手动分两轮：

### Warm cache

```bash
./rw_copy test.bin /dev/null || true
./uring_copy test.bin /dev/null || true

hyperfine --warmup 2 \
  './rw_copy test.bin out1.bin' \
  './uring_copy test.bin out2.bin'
```

### Cold-ish cache

```bash
sync
sudo sh -c 'echo 3 > /proc/sys/vm/drop_caches'
/usr/bin/time -v ./rw_copy test.bin out1.bin

sync
sudo sh -c 'echo 3 > /proc/sys/vm/drop_caches'
/usr/bin/time -v ./uring_copy test.bin out2.bin
```

## 如果要写得更硬核，还可以补什么

如果你想把文章升级成更认真一点的 benchmark 文，我会建议再补三组对照：

### 基线一：同步 `read/write`

最容易懂，也最有代表性。

### 基线二：`pread/pwrite` + 多线程

这个很重要。

因为很多时候你真正要比较的，不是“古老同步 IO”和“现代 io_uring”，而是：

- 传统模型靠线程池硬顶并发
- `io_uring` 靠 ring 和 completion 驱动并发

这组对照会更接近真实工程权衡。

### 进阶组：读写都走 `io_uring`

也就是：

- 提交 read SQE
- read 完成后提交 write SQE
- 用 user_data 维护状态
- 控制 inflight queue depth

到这一步，才更接近 `io_uring` 真正擅长的 pipeline 模型。

## Rust 版本我也实际试了

除了 C 版本，我还额外写了一套 Rust 版本的实验代码，放在：

```text
code/io-uring-bench-rust/
```

里面有：

- `rw_copy.rs`：普通文件拷贝版本
- `uring_copy.rs`：基于 Rust `io-uring` crate 的版本

这次我不是只“写了代码没验证”，而是实际做了两步验证：

### 1. Rust 普通读写版：可编译、可运行

我在本机实际跑通了普通版本，结果是：

```text
rust rw_copy: copied 64.00 MiB in 0.044 s (1445.06 MiB/s)
```

这说明至少：

- Rust 工具链本身可用
- cargo 构建链路正常
- 普通文件读写 benchmark 可以实跑

### 2. Rust io_uring 版：可编译，但运行时被环境拦住

Rust 的 `io_uring` 版本已经成功编译通过，但在运行到创建 ring 时失败，错误是：

```text
Operation not permitted (os error 1)
```

也就是说，这里卡住的不是：

- Rust 语法
- cargo 依赖
- `io-uring` crate 本身

而是**当前运行环境不允许这个进程创建 io_uring 实例**。

这个结论其实很有工程价值，因为它提醒了一件容易被忽略的事：

> 你是否能用 io_uring，不只取决于代码会不会写，还取决于当前机器、容器、内核配置和安全策略是否真的放行这项能力。

所以如果你在某个容器、PaaS 或受限环境里测试 io_uring，看到 `EPERM`，别急着怀疑代码写错，先怀疑环境权限是不是根本没放开。

## 一个结果记录模板

你可以直接按下面这个表填自己的实验结果：

| 场景 | block size | queue depth | rw_copy | uring_copy | 备注 |
| --- | --- | --- | --- | --- | --- |
| warm cache | 256 KiB | 1 | xx MiB/s | xx MiB/s | 单文件顺序拷贝 |
| cold-ish cache | 256 KiB | 1 | xx MiB/s | xx MiB/s | 清缓存后重跑 |
| warm cache | 4 KiB | 64 | xx MiB/s | xx MiB/s | 小块 IO |
| multi-file | 256 KiB | 64 | xx MiB/s | xx MiB/s | 并发场景 |
| rust rw_copy | 256 KiB | 1 | 1445.06 MiB/s | - | 64 MiB 测试文件，本机实测 |
| rust uring_copy | 256 KiB | 64 | - | EPERM | 编译通过，运行时创建 ring 被环境拦截 |

## 一个结果解读模板

如果你后面自己跑数据，建议按这种方式解读，而不是只贴一句“快了 xx%”：

- 在单文件顺序读写场景下，`io_uring` 和同步 `read/write` 差距有限
- 在 page cache 命中时，优势更多体现在 syscall / CPU 开销
- 在磁盘吞吐已打满时，两者差距会被硬件瓶颈吞掉
- 在并发度提高、block size 变小后，`io_uring` 更容易开始拉开优势
- 如果引入多线程 `pread/pwrite`，传统方案也可能非常能打，但线程成本会更高

## 为什么你可能看不出明显差距

这点特别重要。

很多人跑完一测，发现：

“怎么没快多少？”

这很正常。

因为在**单文件顺序拷贝**这种场景里，瓶颈很可能根本不是系统调用次数，而是：

- 页缓存
- 磁盘吞吐
- 文件系统行为
- 写回策略

这时候 `io_uring` 的优势不一定能被放大出来。

## 那性能差距到底来自哪里

更准确地说，`io_uring` 的优势主要来自这几类地方：

### 1. 减少系统调用和上下文切换

传统同步模型下，你每次都要主动发起调用并等待结果。

`io_uring` 允许批量提交、批量完成，能减少频繁进出内核的成本。

### 2. 更容易做深队列并发

如果你有很多 IO 在同时飞，`io_uring` 会比“一个线程卡一个阻塞 IO”更自然，也更省线程资源。

### 3. 更适合构建 pipeline

比如：

- 一边读
- 一边算
- 一边写
- 多个请求并行推进

这种场景下，它比传统串行 `read/write` 更容易把吞吐拉起来。

## 一个更公平的判断方式

如果你真想看差距，不要只测“拷一个大文件”。

你更应该测这些场景：

- 大量小文件
- 多文件并发读写
- 高并发日志处理
- 网络 socket + 文件 IO 混合
- 需要限制线程数量但又要维持高吞吐的服务

在这些场景里，`io_uring` 更容易展现价值。

## 一句话总结

`io_uring` 不等于“所有文件 IO 都自动更快”。

它真正强的地方，是让你用更低的系统调用成本、更少的线程和更强的并发控制，去组织复杂 IO 工作流。

如果只是最朴素的顺序文件拷贝，差距可能不大；但一旦进入高并发和 pipeline 场景，它的优势通常会开始变得明显。