use anyhow::{Context, Result};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use srt_tokio::{SrtListener, SrtSocket};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt::init();

	let args: Vec<String> = std::env::args().collect();
	if args.len() != 3 {
		eprintln!("Usage: {} <input_address> <output_address>", args[0]);
		eprintln!("Example: {} 0.0.0.0:10001 0.0.0.0:11001", args[0]);
		std::process::exit(1);
	}

	let input_addr: SocketAddr = args[1].parse().with_context(|| format!("Failed to parse input address: {}", args[1]))?;

	let output_addr: SocketAddr = args[2].parse().with_context(|| format!("Failed to parse output address: {}", args[2]))?;

	info!("Starting srt-relay server");
	info!("Input socket: {}", input_addr);
	info!("Output socket: {}", output_addr);

	let (tx, _) = broadcast::channel::<Bytes>(1024);
	let tx = Arc::new(tx);

	let tx_input = Arc::clone(&tx);
	let input_task = tokio::spawn(async move {
		if let Err(e) = handle_input(input_addr, tx_input).await {
			error!("Input handler error: {}", e);
		}
	});

	let output_task = tokio::spawn(async move {
		if let Err(e) = handle_output(output_addr, tx).await {
			error!("Output handler error: {}", e);
		}
	});

	let _ = tokio::join!(input_task, output_task);

	Ok(())
}

async fn handle_input(addr: SocketAddr, tx: Arc<broadcast::Sender<Bytes>>) -> Result<()> {
	let (_listener, mut incoming) = SrtListener::builder().bind(addr).await.context("Failed to bind input SRT listener")?;

	info!("Input listener ready on {}", addr);

	while let Some(request) = incoming.incoming().next().await {
		let peer_addr = request.remote();
		info!("Input connection request from {}", peer_addr);

		match request.accept(None).await {
			Ok(socket) => {
				info!("Input connection accepted from {}", peer_addr);

				let tx = Arc::clone(&tx);
				tokio::spawn(async move {
					if let Err(e) = process_input_stream(socket, peer_addr, tx).await {
						error!("Error processing input from {}: {}", peer_addr, e);
					}
					info!("Input connection from {} closed", peer_addr);
				});
			}
			Err(e) => {
				error!("Failed to accept connection from {}: {}", peer_addr, e);
			}
		}
	}

	Ok(())
}

async fn process_input_stream(mut socket: SrtSocket, peer_addr: SocketAddr, tx: Arc<broadcast::Sender<Bytes>>) -> Result<()> {
	while let Some(result) = socket.next().await {
		match result {
			Ok((_, packet)) => {
				let packet_size = packet.len();
				debug!("Received {} bytes from {}", packet_size, peer_addr);

				match tx.send(packet) {
					Ok(count) => {
						debug!("Broadcasted packet to {} receivers", count);
					}
					Err(_) => {
						debug!("No active receivers for broadcast");
					}
				}
			}
			Err(e) => {
				warn!("Error receiving packet from {}: {}", peer_addr, e);
				return Err(e.into());
			}
		}
	}

	Ok(())
}

async fn handle_output(addr: SocketAddr, tx: Arc<broadcast::Sender<Bytes>>) -> Result<()> {
	let (_listener, mut incoming) = SrtListener::builder().bind(addr).await.context("Failed to bind output SRT listener")?;

	info!("Output listener ready on {}", addr);

	while let Some(request) = incoming.incoming().next().await {
		let peer_addr = request.remote();
		info!("Output connection request from {}", peer_addr);

		match request.accept(None).await {
			Ok(socket) => {
				info!("Output connection accepted from {}", peer_addr);

				let rx = tx.subscribe();

				tokio::spawn(async move {
					if let Err(e) = process_output_stream(socket, peer_addr, rx).await {
						error!("Error processing output for {}: {}", peer_addr, e);
					}
					info!("Output connection to {} closed", peer_addr);
				});
			}
			Err(e) => {
				error!("Failed to accept connection from {}: {}", peer_addr, e);
			}
		}
	}

	Ok(())
}

async fn process_output_stream(mut socket: SrtSocket, peer_addr: SocketAddr, mut rx: broadcast::Receiver<Bytes>) -> Result<()> {
	loop {
		match rx.recv().await {
			Ok(packet) => {
				let packet_size = packet.len();

				if let Err(e) = socket.send((std::time::Instant::now(), packet)).await {
					warn!("Failed to send packet to {}: {}", peer_addr, e);
					return Err(e.into());
				}

				debug!("Sent {} bytes to {}", packet_size, peer_addr);
			}
			Err(broadcast::error::RecvError::Lagged(count)) => {
				warn!("Output {} lagged by {} messages", peer_addr, count);
			}
			Err(broadcast::error::RecvError::Closed) => {
				info!("Broadcast channel closed for {}", peer_addr);
				return Ok(());
			}
		}
	}
}
