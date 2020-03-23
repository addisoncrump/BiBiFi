//This code was modified from code posted by Reddit user u/nsossonko
//at https://www.reddit.com/r/rust/comments/e82v07/my_introduction_to_tokio_streaming/

use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());

    let mut socket = TcpListener::bind(&addr).await.unwrap();
    println!("Listening on: {}", addr);

    while let Ok((mut stream, peer)) = socket.accept().await {
        println!("Incoming connection from: {}", peer.to_string());
        tokio::spawn(async move {
            let (reader, mut writer) = stream.split();

            let mut buf_reader = BufReader::new(reader);
            let mut buf = vec![];
            let mut ast_count = 0u8;
            let mut clear_buf = false;

            loop {
                match buf_reader.read_until(b'*', &mut buf).await {
                    Ok(n) => {
                        if n == 0 {
                            println!("EOF received");
                            break;
                        }

                        // Create a String out of the u8 buffer of characters
                        let buf_string = String::from_utf8_lossy(&buf);
                        if (ast_count == 0) && (n > 1) {
                            ast_count = ast_count + 1;
                        } else if (ast_count > 0) && (n == 1) {
                            ast_count = ast_count + 1;
                        } else {
                            ast_count = 0;
                            clear_buf = true;
                        }
                        println!("count: {}", ast_count);

                        if ast_count == 3 {
                            // Printout the message received
                            println!("Received message: {}", buf_string);

                            //TODO: Pass to parser here

                            // Reply with the message received.
                            let message = format!("We received your message of: {}\n", buf_string);
                            // Send off the response.
                            match writer.write_all(&message.as_bytes()).await {
                                Ok(_n) => println!("Response sent"),
                                Err(e) => println!("Error sending response: {}", e),
                            }
                            // Clear the buffer so that this line doesn't get mixed
                            // with the next lines
                            clear_buf = true;
                            ast_count = 0;
                        }

                        if clear_buf == true {
                            buf.clear();
                            clear_buf = false;
                        }
                    }
                    Err(e) => println!("Error receiving message: {}", e),
                }
            }
        });
    }

    Ok(())
}
