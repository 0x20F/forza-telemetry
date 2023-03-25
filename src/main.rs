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
        let mut writer = csv::Writer::from_path("fm7-full-telemetry-rio-4-laps.csv")?;

        let now = Instant::now();
        let mut stdout = stdout();
        let mut count = 0;

        loop {
            let (size, _) = socket.recv_from(&mut buf).await?;

            // For about 16 minutes of gameplay, we'll have 60k of these.
            // That assumes running at 60fps, with 1 packet per frame.
            let packet = Packet::new(&buf[..size], now.elapsed().as_nanos());

            if !packet.is_race_on {
                // Save just in case
                writer.flush()?;
                continue;
            }

            writer.serialize(packet)?;

            print!("\r{}", now.elapsed().as_nanos());
            stdout.flush()?;

            count += 1;
            if count % 1000 == 0 {
                // Actually push the data to the file
                writer.flush()?;
            }
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
