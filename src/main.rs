mod base;

use std::io::{stdout, Write};
use std::time::Instant;
use std::{error::Error, io};
use clap::Parser;
use tokio::net::UdpSocket;

use base::Packet;
use tokio::signal;
use tokio::sync::mpsc::{Sender, Receiver, channel};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "output.csv")]
    filename: String
}


struct Server<'a> {
    socket: UdpSocket,
    buf: Vec<u8>,

    out_file: &'a str
}

impl<'a> Server<'a> {
    async fn run(self, mut shutdown_receiver: Receiver<()>, _message_sender: Sender<()>) -> Result<(), io::Error> {
        let Server { socket, mut buf, out_file } = self;
        let mut writer = csv::Writer::from_path(out_file)?;

        let now = Instant::now();
        let mut stdout = stdout();
        let mut count = 0;

        println!("Listening on: {}", socket.local_addr()?);

        loop {
            let ( size, _ ) = tokio::select! {
                res = socket.recv_from(&mut buf) => res?,
                _ = shutdown_receiver.recv() => {
                    println!("Received shutdown request...");

                    // On receive, we want to save what we got, and get out
                    // of the function
                    println!("   Saving any unsaved data to {}", out_file);
                    writer.flush()?;

                    println!("   Exiting.");
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let (stop_tx, stop_rx) = channel(1);
    let (done_tx, mut done_rx) = channel(2);


    let addr = String::from("0.0.0.0:23555");
    let socket = UdpSocket::bind(&addr).await?;
    
    let clone_tx = done_tx.clone();

    tokio::spawn(async move {
        let server = Server {
            socket,
            buf: vec![0; 1024],
            out_file: &args.filename
        };

        server
            .run(
                stop_rx, 
                clone_tx
            ).await.unwrap();
    });

    match signal::ctrl_c().await {
        Ok(()) => { 
            // Drop our version
            drop(done_tx);

            // Send shutdown signal to the application and wait for the
            // other end of the done channel to drop. This will return an error
            // but we don't care about that. It indicates that the other code
            // is done running and the clone we sent over got dropped.
            stop_tx.send(()).await?;
            let _ = done_rx.recv().await;
        },
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }

    Ok(())
}
