// src/bin/server.rs
use s2n_quic::Server;
use s2n_quic::{client::Connect, Client};
use std::env;
use std::{error::Error, net::SocketAddr};
use tracing::error;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

// 生成私钥：
// openssl genrsa -out key.pem 2048
// 生成证书签名请求（CSR）：
// openssl req -new -key key.pem -out cert.csr
// 在提示时，输入你的组织信息和域名。
// 生成自签名证书：
// openssl x509 -req -days 365 -in cert.csr -signkey key.pem -out cert.pem

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    tokio::spawn(async {
        if let Err(e) = server().await {
            error!("handle with {}", e)
        }
    });
    client().await?;
    Ok(())
}

async fn server() -> Result<(), Box<dyn Error>> {
    info!("Server start");
    let current_dir = env::current_dir()?;
    let cert = current_dir.join("examples").join("cert.pem");
    let key = current_dir.join("examples").join("key.pem");
    let mut server = Server::builder()
        .with_tls((cert.as_path(), key.as_path()))?
        .with_io("127.0.0.1:4433")?
        .start()?;

    while let Some(mut connection) = server.accept().await {
        // spawn a new task for the connection
        tokio::spawn(async move {
            while let Ok(Some(mut stream)) = connection.accept_bidirectional_stream().await {
                // spawn a new task for the stream
                tokio::spawn(async move {
                    // echo any data back to the stream
                    while let Ok(Some(data)) = stream.receive().await {
                        stream.send(data).await.expect("stream should be open");
                    }
                });
            }
        });
    }

    Ok(())
}

async fn client() -> Result<(), Box<dyn Error>> {
    info!("Client start");
    let current_dir = env::current_dir()?;
    let cert = current_dir.join("examples").join("cert.pem");
    let client = Client::builder()
        .with_tls(cert.as_path())?
        .with_io("0.0.0.0:0")?
        .start()?;

    let addr: SocketAddr = "127.0.0.1:4433".parse()?;
    let connect = Connect::new(addr).with_server_name("localhost");
    let mut connection = client.connect(connect).await?;

    // ensure the connection doesn't time out with inactivity
    connection.keep_alive(true)?;

    // open a new stream and split the receiving and sending sides
    let stream = connection.open_bidirectional_stream().await?;
    let (mut receive_stream, mut send_stream) = stream.split();

    // spawn a task that copies responses from the server to stdout
    tokio::spawn(async move {
        let mut stdout = tokio::io::stdout();
        let _ = tokio::io::copy(&mut receive_stream, &mut stdout).await;
    });

    // copy data from stdin and send it to the server
    let mut stdin = tokio::io::stdin();
    tokio::io::copy(&mut stdin, &mut send_stream).await?;

    Ok(())
}
