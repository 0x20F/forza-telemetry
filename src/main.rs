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
        let mut writer = csv::Writer::from_writer(io::stdout());

        let now = Instant::now();
        let mut stdout = stdout();

        loop {
            let (size, _) = socket.recv_from(&mut buf).await?;

            // For about 16 minutes of gameplay, we'll have 60k of these.
            // That assumes running at 60fps, with 1 packet per frame.
            let packet = Packet::new(&buf[..size]);

            if !packet.is_race_on {
                continue;
            }

            writer.serialize(packet)?;
            writer.flush()?;
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
