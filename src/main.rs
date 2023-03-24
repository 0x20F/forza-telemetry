use std::io::Read;
use std::ops::{BitXor, Shl};
use std::time::Instant;
use std::{error::Error, io};
use tokio::net::UdpSocket;

struct Server {
    socket: UdpSocket,
    buf: Vec<u8>,
}

impl Server {
    async fn run(self) -> Result<(), io::Error> {
        let Server { socket, mut buf } = self;
        let now = Instant::now();

        loop {
            socket.recv(&mut buf).await?;

            println!(
                "Received data: {:?} - {} nanoseconds after server start",
                buf,
                now.elapsed().as_nanos()
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = String::from("0.0.0.0:23555");

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on: {}", socket.local_addr()?);

    let server = Server {
        socket,
        buf: vec![0; 1024],
    };

    server.run().await?;

    Ok(())
}
