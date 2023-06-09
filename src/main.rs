mod base;
mod server;

use std::error::Error;
use clap::Parser;
use tokio::net::UdpSocket;

use tokio::signal;
use tokio::sync::mpsc::channel;

use server::Server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The name and path of the csv file to dump data into
    #[arg(short, long)]
    csv: Option<String>
}




#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let (stop_tx, stop_rx) = channel(1);
    let (done_tx, mut done_rx) = channel(2);


    let addr = String::from("0.0.0.0:23555");
    let socket = UdpSocket::bind(&addr).await?;
    
    let clone_tx = done_tx.clone();
    let mut server = Server::new(socket, args).await?;

    tokio::spawn(async move {
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
