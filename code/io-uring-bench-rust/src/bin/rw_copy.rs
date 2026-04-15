use anyhow::{Context, Result};
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::time::Instant;

const BUF_SIZE: usize = 256 * 1024;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <src> <dst>", args[0]);
        std::process::exit(1);
    }

    let src = &args[1];
    let dst = &args[2];

    let input = File::open(src).with_context(|| format!("open src: {src}"))?;
    let output = File::create(dst).with_context(|| format!("create dst: {dst}"))?;

    let mut reader = BufReader::with_capacity(BUF_SIZE, input);
    let mut writer = BufWriter::with_capacity(BUF_SIZE, output);
    let mut buf = vec![0u8; BUF_SIZE];
    let mut total: u64 = 0;

    let start = Instant::now();
    loop {
        let n = reader.read(&mut buf).context("read failed")?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n]).context("write failed")?;
        total += n as u64;
    }
    writer.flush().context("flush failed")?;
    let sec = start.elapsed().as_secs_f64();
    let mib = total as f64 / 1024.0 / 1024.0;
    eprintln!("rust rw_copy: copied {:.2} MiB in {:.3} s ({:.2} MiB/s)", mib, sec, mib / sec);
    Ok(())
}
