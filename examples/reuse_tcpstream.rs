use anyhow::Result;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};
const ADDR: &str = "127.0.0.1:8081";

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    tokio::spawn(async move { echo_server().await });

    info!("Waiting for echo server start");
    sleep(Duration::from_secs(3)).await;
    info!("Begin to connect to the echo server start");

    // 目标服务器地址
    let upstream_addr = ADDR;

    // 建立连接
    let mut upstream = TcpStream::connect(upstream_addr).await?;

    // 发送数据
    let data = b"Hello, server!";
    upstream.write_all(data).await?;
    info!("Send data {:?}", data);

    read(&mut upstream).await?;

    // 假设我们想要再次发送数据
    let more_data = b"More data!";
    upstream.write_all(more_data).await?;

    read(&mut upstream).await?;
    // 继续使用连接...

    Ok(())
}

async fn read(upstream: &mut TcpStream) -> Result<()> {
    let mut buf = vec![0; 1024];
    // 从socket读取数据
    let n = upstream.read(&mut buf).await?;

    if n == 0 {
        // 如果读取到的数据长度为0，表示对方已经关闭连接
        println!("Client disconnected.");
        return Ok(());
    }

    // 打印读取到的数据
    println!("Received from client: {:?}", &buf[..n]);
    Ok(())
}

async fn echo_server() -> anyhow::Result<()> {
    // 创建一个TcpListener来监听传入的连接

    let listener = TcpListener::bind(ADDR).await?;
    info!("Echo: Server start {}", ADDR);
    loop {
        // 接受一个连接
        let (mut socket, c_addr) = listener.accept().await?;
        info!("Echo: Accept client {}", c_addr);
        // 为每个连接创建一个异步任务
        tokio::spawn(async move {
            // 使用一个循环来持续读取和发送数据
            let mut buf = vec![0; 1024];

            loop {
                // 从socket读取数据
                let n = match socket.read(&mut buf).await {
                    Ok(0) => return, // 如果没有数据，则退出循环
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Echo: Failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                info!("Echo: Receive data len {}", n);
                // 将读取的数据发送回客户端
                if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprintln!("Echo: Failed to write to socket; err = {:?}", e);
                    return;
                }
                info!("Echo: Write data back");
            }
        });
    }
}
