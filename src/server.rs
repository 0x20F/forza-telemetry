use std::io::{stdout, Write};
use std::time::Instant;
use paris::log;
use super::base::Packet;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{Receiver, Sender};
use std::io;


pub struct Server<'a> {
    pub socket: UdpSocket,
    pub buf: Vec<u8>,

    pub out_file: &'a str
}

impl<'a> Server<'a> {
    pub async fn run(self, mut shutdown_receiver: Receiver<()>, _message_sender: Sender<()>) -> Result<(), io::Error> {
        let Server { socket, mut buf, out_file } = self;
        let mut writer = csv::Writer::from_path(out_file)?;

        let now = Instant::now();
        let mut stdout = stdout();
        let mut count = 0;

        log!("[<cyan>+</>] Listening on: <bright_green>{}</>", socket.local_addr()?);
        log!("[<cyan>+</>] Outputting to: <bright_green>{}</>", out_file);

        loop {
            let ( size, _ ) = tokio::select! {
                res = socket.recv_from(&mut buf) => res?,
                _ = shutdown_receiver.recv() => {
                    log!("[<yellow>~</>] Received shutdown request...");

                    // On receive, we want to save what we got, and get out
                    // of the function
                    log!("   [<bright_red>*</>] Saving any unsaved data to {}", out_file);
                    writer.flush()?;

                    log!("   [<bright_red>*</>] Exiting.");
                    return Ok(())
                }
            };

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