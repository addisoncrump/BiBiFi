//This code was modified from code posted by Reddit user u/nsossonko 
//at https://www.reddit.com/r/rust/comments/e82v07/my_introduction_to_tokio_streaming/

use std::env;
use tokio::net::TcpListener;
// This struck me as an interesting Rust-ism. We must add a use statement for
// `AsyncBufReadExt` even though we don't explicitly "use" it in our code.
// This is necessary so that the BufReader will have methods from that trait.
// Same goes for AsyncWriteExt.
use tokio::io::{BufReader, AsyncBufReadExt, AsyncWriteExt};

// The easiest way to start the tokio runtime is with this decorator
#[tokio::main]
async fn main() -> Result<(), ()> {
    // Allow passing an address to listen on as the first argument of this
    // program, but otherwise we'll just set up our TCP listener on
    // 127.0.0.1:8080.
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());

    // Setup the tcp stream listener. We use unwrap here on the Future result
    // because it makes sense for the program to stop at this point if we can't
    // even bind our listener.
    let mut socket = TcpListener::bind(&addr).await.unwrap();
    println!("Listening on: {}", addr);

    // Here we want to continuously accept new connections. This will keep the
    // accept loop going endlessly (unless there's a problem with the socket).
    while let Ok((mut stream, peer)) = socket.accept().await {
        // Printout connection
        println!("Incoming connection from: {}", peer.to_string());
        // Handle the connection but don't block the thread until it is
        // completely done. Instead, spawn a new Future and handle this
        // connection there. The simplest signature will usually be `async move`
        // as it won't require worrying about mutability and the borrow checker.
        tokio::spawn(async move {
            // We split the TcpStream here so that the reader/writer can be moved in
            let (reader, mut writer) = stream.split();
            // Here we create a BufReader. There is no simple API on TcpStream
            // to read from the stream line-by-line, like there is for the file
            // based IO, instead we have to do this.
            let mut buf_reader = BufReader::new(reader);
            let mut buf = vec![];
            let mut ast_count = 0; //counts the number of astrics
            // Continuously read one line at a time from this stream
            loop {
                match buf_reader.read_until(b'*', &mut buf).await {
                    Ok(n) => {
                        // We received data on the stream. Usually this will be
                        // a complete message until LF, however it is possible
                        // that the remote stream closed the connection and we
                        // received the EOF, check for that
                        if n == 0 {
                            // 0 bytes received, EOF
                            println!("EOF received");
                            break;
                        }
                        ast_count = ast_count + 1;
                        println!("count: {}",ast_count);

                        // Create a String out of the u8 buffer of characters
                        let buf_string = String::from_utf8_lossy(&buf);

                        if buf_string == "*"{
                            println!("Yes!!!");
                        }

 
                        if ast_count == 3{
                            // Printout the message received
                            println!("Received message: {}",buf_string);

                            //TODO: Pass to parser here

                            // Reply with the message received.
                            let message = format!("We received your message of: {}\n", buf_string);
                            // Send off the response.
                            match writer.write_all(&message.as_bytes()).await {
                                Ok(_n) => println!("Response sent"),
                                Err(e) => println!(
                                    "Error sending response: {}", e
                                )
                            }
                            // Clear the buffer so that this line doesn't get mixed
                            // with the next lines
                            buf.clear();
                            ast_count = 0; 
                        }
                        
                    },
                    Err(e) => println!("Error receiving message: {}", e)
                }
            }
        });
    }

    Ok(())
}