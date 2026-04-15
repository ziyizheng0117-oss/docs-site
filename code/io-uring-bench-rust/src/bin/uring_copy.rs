use anyhow::{bail, Context, Result};
use io_uring::{opcode, squeue, IoUring};
use std::env;
use std::fs::{metadata, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::time::Instant;

const QUEUE_DEPTH: u32 = 64;
const BLOCK_SIZE: usize = 256 * 1024;

#[derive(Clone, Copy, Debug)]
enum OpType {
    Read,
    Write,
}

#[derive(Debug)]
struct IoTask {
    op: OpType,
    offset: u64,
    len: usize,
    buf: Vec<u8>,
}

fn push_entry(ring: &mut IoUring, entry: squeue::Entry) -> Result<()> {
    unsafe {
        ring.submission()
            .push(&entry)
            .map_err(|_| anyhow::anyhow!("submission queue is full"))?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <src> <dst>", args[0]);
        std::process::exit(1);
    }

    let src = &args[1];
    let dst = &args[2];

    let input = File::open(src).with_context(|| format!("open src: {src}"))?;
    let output = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(dst)
        .with_context(|| format!("open dst: {dst}"))?;

    let file_size = metadata(src)?.len();
    let in_fd = input.as_raw_fd();
    let out_fd = output.as_raw_fd();

    let mut ring = IoUring::new(QUEUE_DEPTH).context("create io_uring")?;
    let mut next_offset: u64 = 0;
    let mut inflight: usize = 0;
    let mut bytes_done: u64 = 0;

    let start = Instant::now();

    for _ in 0..QUEUE_DEPTH {
        if next_offset >= file_size {
            break;
        }
        let task = Box::new(IoTask {
            op: OpType::Read,
            offset: next_offset,
            len: BLOCK_SIZE,
            buf: vec![0u8; BLOCK_SIZE],
        });
        let task_ptr = Box::into_raw(task);
        let read_e = opcode::Read::new(
            io_uring::types::Fd(in_fd),
            unsafe { (*task_ptr).buf.as_mut_ptr() },
            BLOCK_SIZE as _,
        )
        .offset(next_offset as _)
        .build()
        .user_data(task_ptr as u64);
        push_entry(&mut ring, read_e)?;
        next_offset += BLOCK_SIZE as u64;
        inflight += 1;
    }

    ring.submit().context("initial submit")?;

    while inflight > 0 {
        ring.submit_and_wait(1).context("submit_and_wait")?;

        let mut completions = Vec::new();
        {
            let mut cq = ring.completion();
            for cqe in &mut cq {
                completions.push((cqe.user_data(), cqe.result()));
            }
        }

        for (user_data, res) in completions {
            let task_ptr = user_data as *mut IoTask;
            if task_ptr.is_null() {
                bail!("null user_data from cqe");
            }

            let task = unsafe { Box::from_raw(task_ptr) };

            if res < 0 {
                bail!("io failed: {}", std::io::Error::from_raw_os_error(-res));
            }

            match task.op {
                OpType::Read => {
                    if res == 0 {
                        inflight -= 1;
                        continue;
                    }

                    let nread = res as usize;
                    let mut task = task;
                    task.op = OpType::Write;
                    task.len = nread;

                    let task_ptr = Box::into_raw(task);
                    let write_e = opcode::Write::new(
                        io_uring::types::Fd(out_fd),
                        unsafe { (*task_ptr).buf.as_ptr() },
                        nread as _,
                    )
                    .offset(unsafe { (*task_ptr).offset } as _)
                    .build()
                    .user_data(task_ptr as u64);
                    push_entry(&mut ring, write_e)?;
                }
                OpType::Write => {
                    bytes_done += res as u64;
                    inflight -= 1;

                    if next_offset < file_size {
                        let task = Box::new(IoTask {
                            op: OpType::Read,
                            offset: next_offset,
                            len: BLOCK_SIZE,
                            buf: vec![0u8; BLOCK_SIZE],
                        });
                        let task_ptr = Box::into_raw(task);
                        let read_e = opcode::Read::new(
                            io_uring::types::Fd(in_fd),
                            unsafe { (*task_ptr).buf.as_mut_ptr() },
                            BLOCK_SIZE as _,
                        )
                        .offset(next_offset as _)
                        .build()
                        .user_data(task_ptr as u64);
                        push_entry(&mut ring, read_e)?;
                        next_offset += BLOCK_SIZE as u64;
                        inflight += 1;
                    }
                }
            }
        }
    }

    output.sync_all().context("sync output")?;

    let sec = start.elapsed().as_secs_f64();
    let mib = bytes_done as f64 / 1024.0 / 1024.0;
    eprintln!(
        "rust uring_copy: copied {:.2} MiB in {:.3} s ({:.2} MiB/s)",
        mib,
        sec,
        mib / sec
    );

    Ok(())
}
