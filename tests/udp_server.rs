//! Phase 3 integration test: spin up a local `serve()` instance, send a
//! synthetic Dash FM packet to it, and confirm the SessionWriter wrote a
//! row with the expected fields. Mirrors what the `record` binary does.

mod common;

use std::time::Duration;

use chrono::Utc;
use common::PacketSpec;
use forza_telemetry::csv_writer::{SessionMeta, SessionWriter};
use forza_telemetry::{decode, stream, Source};
use tempfile::tempdir;
use tokio::net::UdpSocket;
use tokio::sync::oneshot;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn udp_packet_round_trips_through_serve_and_writer() {
    let dir = tempdir().unwrap();
    let out_dir = dir.path().to_path_buf();

    // Bind a sender first so we can pick its peer using a known listener port.
    let listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let listen_addr = listener.local_addr().unwrap();
    drop(listener); // release the port; serve() will rebind

    // Channel used as our "shutdown" signal. We resolve it from the test
    // body once we've confirmed the packet was processed.
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Build the packet we will send.
    let mut spec = PacketSpec::distinct();
    spec.steer = -64;
    let packet_bytes = spec.to_bytes(Source::DashFm);
    let expected_ordinal = spec.car_ordinal;

    let server = tokio::spawn({
        let out_dir = out_dir.clone();
        async move {
            let mut writer: Option<SessionWriter> = None;
            stream::serve(
                listen_addr,
                |bytes, recv_ns| {
                    let pkt = decode(bytes, recv_ns).expect("decode");
                    if writer.is_none() {
                        let meta = SessionMeta {
                            source: pkt.source,
                            car_ordinal: pkt.car_ordinal,
                            track_id: pkt.track_id,
                            session_started: Utc::now(),
                        };
                        writer = Some(SessionWriter::open(&out_dir, meta).unwrap());
                    }
                    writer.as_mut().unwrap().write_packet(&pkt).unwrap();
                },
                async move {
                    let _ = shutdown_rx.await;
                },
            )
            .await
            .unwrap();
            let w = writer.expect("at least one packet handled");
            let path = w.path().to_path_buf();
            w.close().unwrap();
            path
        }
    });

    // Give the server time to bind, then emit the packet.
    tokio::time::sleep(Duration::from_millis(100)).await;
    let sender = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    sender.send_to(&packet_bytes, listen_addr).await.unwrap();

    // Wait long enough for the receive loop to handle the packet, then
    // shut down so the writer flushes (csv::Writer buffers until close).
    tokio::time::sleep(Duration::from_millis(300)).await;
    let _ = shutdown_tx.send(());

    let path = server.await.unwrap();

    let contents = std::fs::read_to_string(&path).unwrap();
    let mut lines = contents.lines();
    assert!(lines.next().unwrap().starts_with("# schema_version=2 source=dash_fm"));
    let header = lines.next().unwrap();
    let row = lines.next().expect("at least one row");
    let fields: Vec<&str> = header.split(',').collect();
    let values: Vec<&str> = row.split(',').collect();
    let by_name: std::collections::HashMap<_, _> =
        fields.iter().copied().zip(values.iter().copied()).collect();

    assert_eq!(by_name["source"], "dash_fm");
    assert_eq!(by_name["car_ordinal"], expected_ordinal.to_string());
    assert_eq!(by_name["steer"], "-64");
}
