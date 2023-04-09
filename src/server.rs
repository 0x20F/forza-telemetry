use std::fs::File;
use std::io::{stdout, Write};
use std::time::Instant;
use paris::log;
use crate::Args;

use super::base::Packet;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{Receiver, Sender};
use std::io;


pub struct Server {
    socket: UdpSocket,
    csv_writer: Option<csv::Writer<File>>
}

impl Server {
    pub fn new(socket: UdpSocket, args: Args) -> Result<Server, io::Error> {
        let mut writer = None;

        if args.csv.is_some() {
            let file = args.csv.unwrap();

            log!("[<cyan>+</>] Outputting CSV to: <bright_green>{}</>", file);
            writer = Some(csv::Writer::from_path(file)?);
        }

        Ok(
            Server {
                csv_writer: writer,
                socket
            }
        )
    }

    fn buffer_csv(&mut self, packet: &Packet) -> Result<(), io::Error> {
        if self.csv_writer.is_none() {
            return Ok(());
        }

        let writer = self.csv_writer.as_mut().unwrap();
        writer.serialize(packet)?;

        return Ok(())
    }

    fn flush_csv(&mut self) -> Result<(), io::Error> {
        if self.csv_writer.is_some() {
            self.csv_writer.as_mut().unwrap().flush()?;
        }

        return Ok(())
    }

    pub async fn run(&mut self, mut shutdown_receiver: Receiver<()>, _message_sender: Sender<()>) -> Result<(), io::Error> {
        let now = Instant::now();
        let mut stdout = stdout();
        let mut count = 0;
        let mut buf = vec![0; 1024];

        log!("[<cyan>+</>] Listening on: <bright_green>{}</>", self.socket.local_addr()?);

        loop {
            let ( size, _ ) = tokio::select! {
                res = self.socket.recv_from(&mut buf) => res?,
                _ = shutdown_receiver.recv() => {
                    log!("[<yellow>~</>] Received shutdown request...");

                    // On receive, we want to save what we got, and get out
                    // of the function
                    log!("   [<bright_red>*</>] Saving any unsaved data to chosen storage options");
                    self.flush_csv()?;

                    log!("   [<bright_red>*</>] Exiting.");
                    return Ok(())
                }
            };

            // For about 16 minutes of gameplay, we'll have 60k of these.
            // That assumes running at 60fps, with 1 packet per frame.
            let packet = Packet::new(&buf[..size], now.elapsed().as_nanos());

            if !packet.is_race_on {
                // Save just in case
                self.flush_csv()?;
                continue;
            }

            self.buffer_csv(&packet)?;

            print!("\r{}", now.elapsed().as_nanos());
            stdout.flush()?;

            count += 1;
            if count % 1000 == 0 {
                // Actually push the data to the file
                self.flush_csv()?;
            }
        }
    }
}