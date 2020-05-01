#[forbid(unused_must_use)]
//This code was modified from code posted by Reddit user u/nsossonko
//at https://www.reddit.com/r/rust/comments/e82v07/my_introduction_to_tokio_streaming/
use bibifi_runtime::status::Status::EXITING;
use bibifi_runtime::BiBiFi;
use futures::{StreamExt as FStreamExt, TryFutureExt};
use regex::Regex;
use signal_hook::{iterator::Signals, SIGTERM};
use std::env;
use tokio::io::AsyncReadExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpListener;
use tokio::stream::StreamExt as TStreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let port = args.next();
    if port.is_none() {
        std::process::exit(255);
    }
    let port = port.unwrap();
    if port.starts_with('0') {
        std::process::exit(255);
    }
    let port = port.parse::<u16>();
    let port = if let Ok(port) = port {
        if port < 1024 {
            std::process::exit(255);
        }
        port
    } else {
        std::process::exit(255);
    };
    let addr = format!("{}:{}", "0.0.0.0", port);

    let pass = args.next();
    let admin_hash = match pass {
        None => bibifi_util::hash("admin".to_string()),
        Some(pass) => {
            if pass.len() > 4096
                || !Regex::new("[A-Za-z0-9_ ,;\\\\.?!-]*")
                    .unwrap()
                    .is_match(&pass)
            {
                std::process::exit(255);
            }
            bibifi_util::hash(pass)
        }
    };

    let (runtime, receiver) = bibifi_runtime::BiBiFi::new();
    let mut socket = TcpListener::bind(&addr).await.unwrap();
    println!("Listening on: {}", addr);

    let signals = Signals::new(&[SIGTERM])?;
    std::thread::spawn(move || {
        // sighandler
        for _ in signals.forever() {
            std::process::exit(0);
        }
    });

    tokio::spawn(async move { BiBiFi::run(admin_hash, receiver).await });

    while let Ok((mut stream, peer)) = socket.accept().await {
        println!("Incoming connection from: {}", peer.to_string());
        let runtime = runtime.clone();
        tokio::spawn(async move {
            let (reader, writer) = stream.split();

            let mut buf_reader = BufReader::new(reader).take(1000000u64);
            let mut buf_writer = BufWriter::new(writer);
            let mut buf = Vec::with_capacity(1000000usize);
            let mut ast_count = 0u8;

            buf_reader.set_limit(1000000u64);

            while ast_count < 3 {
                match buf_reader.read_until(b'*', &mut buf).await {
                    Ok(n) => {
                        if n == 0 {
                            println!("EOF received");
                            return;
                        }

                        // Create a String out of the u8 buffer of characters
                        if ((ast_count == 0) && (n > 1)) || ((ast_count > 0) && (n == 1)) {
                            ast_count = ast_count + 1;
                        } else {
                            ast_count = 0;
                        }
                    }
                    Err(e) => {
                        println!("Error receiving message: {}", e);
                        return;
                    }
                }
            }

            let buf_string = String::from_utf8_lossy(&buf);

            let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

            if let Some(entries) = runtime
                .submit(buf_string.to_string(), sender)
                .and_then(
                    |_| async move { Ok(tokio::stream::StreamExt::next(&mut receiver).await) },
                )
                .await
                .unwrap()
            {
                for entry in entries {
                    match buf_writer
                        .write_all(
                            format!("{}\n", serde_json::to_string(&entry).unwrap()).as_bytes(),
                        )
                        .await
                    {
                        Ok(_) => {
                            if entry.status == EXITING {
                                std::process::exit(0)
                            }
                        }
                        Err(_) => break, // stream closed
                    }
                }
            }
            buf_writer.flush().await.unwrap_or(()); // cheaty hack
            drop(buf_reader);
            drop(buf_writer);
        });
    }

    Ok(())
}
