//This code was modified from code posted by Reddit user u/nsossonko
//at https://www.reddit.com/r/rust/comments/e82v07/my_introduction_to_tokio_streaming/

use bibifi_runtime::status::Entry;
use bibifi_runtime::status::Status::EXITING;
use bibifi_runtime::BiBiFi;
use futures::future::Either;
use futures::select;
use futures::{StreamExt as FStreamExt, TryStreamExt};
use regex::Regex;
use signal_hook::{iterator::Signals, SIGTERM};
use std::env;
use std::num::ParseIntError;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::signal::unix::{signal, SignalKind};
use tokio::stream::StreamExt as TStreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let port = args.next();
    if port.is_none() {
        std::process::exit(255);
    }
    let port = port.unwrap().parse::<u16>();
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
        for sig in signals.forever() {
            std::process::exit(0);
        }
    });

    std::thread::spawn(move || async move {
        BiBiFi::run(admin_hash, receiver).await;
    });

    while let Ok((mut stream, peer)) = socket.accept().await {
        println!("Incoming connection from: {}", peer.to_string());
        let runtime = runtime.clone();
        tokio::spawn(async move {
            let (reader, writer) = stream.split();

            let mut buf_reader = BufReader::new(reader);
            let mut buf_writer = BufWriter::new(writer);
            let mut buf = vec![];
            let mut ast_count = 0u8;

            let res = loop {
                match buf_reader.read_until(b'*', &mut buf).await {
                    Ok(n) => {
                        if n == 0 {
                            println!("EOF received");
                            break None;
                        }

                        // Create a String out of the u8 buffer of characters
                        if ((ast_count == 0) && (n > 1)) || ((ast_count > 0) && (n == 1)) {
                            ast_count = ast_count + 1;
                        } else {
                            ast_count = 0;
                        }

                        if ast_count == 3 {
                            break Some(());
                        }
                    }
                    Err(e) => {
                        println!("Error receiving message: {}", e);
                        break None;
                    }
                }
            };
            let buf_string = String::from_utf8_lossy(&buf);
            // Printout the message received
            println!("Received message: {}", buf_string);

            let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Entry>();
            let submission = runtime.submit(buf_string.to_string(), sender);

            while let Some(entry) = tokio::stream::StreamExt::next(&mut receiver).await {
                match buf_writer
                    .write_all(serde_json::to_string(&entry).unwrap().as_bytes())
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

            submission.await.unwrap();
        });
    }

    Ok(())
}
