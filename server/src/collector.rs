use std::net::SocketAddr;

use sqlx::{Pool, Sqlite};
use tokio::{io::AsyncReadExt, net::TcpListener, net::TcpStream};

use shared_data::{decode, CollectorCommand, DATA_COLLECTOR_ADDRESS};

pub async fn data_collector(cnn: Pool<Sqlite>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(DATA_COLLECTOR_ADDRESS).await?;

    loop {
        let (socket, address) = listener.accept().await?;
        tokio::spawn(new_connection(socket, address, cnn.clone()));
    }
}

async fn new_connection(mut socket: TcpStream, address: SocketAddr, cnn: Pool<Sqlite>) {
    println!("New connection from {address:?}");
    let mut buf = vec![0u8; 1024];

    loop {
        let n = socket
            .read(&mut buf)
            .await
            .expect("failed to read data from socket");

        if n == 0 {
            println!("No data received - connection closed");
            return;
        }

        println!("Received {n} bytes");
        let received_data = decode(&buf[0..n]);

        match received_data {
            (
                timestamp,
                CollectorCommand::SubmitData {
                    collector_id,
                    total_memory,
                    used_memory,
                    average_cpu_usage,
                },
            ) => {
                let collector_id = uuid::Uuid::from_u128(collector_id);
                let collector_id = collector_id.to_string();

                let result = sqlx::query("INSERT INTO timeseries (collector_id, received, total_memory, used_memory, average_cpu) VALUES($1, $2, $3, $4, $5)")
                    .bind(collector_id)
                    .bind(timestamp)
                    .bind(total_memory as i64)
                    .bind(used_memory as i64)
                    .bind(average_cpu_usage)
                    .execute(&cnn)
                    .await;

                if result.is_err() {
                    println!("Error inserting data into the database: {result:?}");
                }
            }
        }
    }
}
