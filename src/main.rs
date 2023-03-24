mod base;

use std::io::{stdout, Write};
use std::time::Instant;
use std::{error::Error, io};
use tokio::net::UdpSocket;

use base::Packet;

struct Server {
    socket: UdpSocket,
    buf: Vec<u8>,
}

impl Server {
    async fn run(self) -> Result<(), io::Error> {
        let Server { socket, mut buf } = self;
        let now = Instant::now();
        let mut stdout = stdout();

        loop {
            let (size, _) = socket.recv_from(&mut buf).await?;
            let packet = Packet::new(&buf[..size]);

            if !packet.is_race_on {
                continue;
            }

            print!("\r{:?}", packet.dash.gear);

            stdout.flush().unwrap();
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
