use anyhow::Result;
use std::{fmt::Debug, sync::Arc};
use tokio::{
    io,
    net::{TcpListener, TcpStream},
};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let config = Arc::new(Config::resolve());
    let listener = TcpListener::bind(&config.listen_addr).await?;

    info!("Service listen on {}", &config.listen_addr);

    if let Err(e) = TcpStream::connect(&config.upstream_addr).await {
        return Err(anyhow::anyhow!(
            "Connect to upstream {} fail with error {}",
            &config.upstream_addr,
            e
        ));
    }

    loop {
        let (mut client, addr) = listener.accept().await?;
        info!("Accept client {}", &addr);
        let upstream_addr = config.upstream_addr.clone();
        tokio::spawn(async move {
            let mut upstream = TcpStream::connect(upstream_addr).await?;

            let upstream_local_addr = upstream.local_addr()?;
            let (mut client_read, mut client_write) = client.split();
            let (mut upstream_read, mut upstream_write) = upstream.split();

            loop {
                info!("Proxy Loop on Upstream {upstream_local_addr} Client {addr}");
                let client_to_upstream = io::copy(&mut client_read, &mut upstream_write);
                let upstream_to_client = io::copy(&mut upstream_read, &mut client_write);
                match tokio::try_join!(client_to_upstream, upstream_to_client) {
                    Ok((n, m)) => {
                        info!("Proxy {n} bytes from client to upstream, {m} bytes from upstream to client");
                        if n == 0 || m == 0 {
                            break;
                        }
                    }
                    Err(e) => warn!("error proxying: {:?}", e),
                }
            }
            info!("Proxy Quit");
            Ok::<(), anyhow::Error>(())
        });
    }
}

#[allow(dead_code)]
async fn async_copy<'a, R, W>(reader: &'a mut R, writer: &'a mut W) -> tokio::io::Result<usize>
where
    R: tokio::io::AsyncReadExt + Unpin + Debug,
    W: tokio::io::AsyncWriteExt + Unpin + Debug,
{
    let mut buffer = [0u8; 8 * 1024]; // 8KB buffer
    let mut total_bytes_copied = 0;

    // loop {
    let n = match reader.read(&mut buffer).await {
        Ok(0) => return Ok(total_bytes_copied), // EOF
        Ok(n) => n,
        Err(e) => match e.kind() {
            io::ErrorKind::ConnectionAborted => {
                info!("reader: {:?}", reader);
                info!("writer: {:?}", writer);
                return Err(e);
            }
            _ => return Err(e),
        },
    };

    writer.write_all(&buffer[..n]).await?;
    total_bytes_copied += n;
    // }

    Ok(total_bytes_copied)
}

#[derive(Debug)]
struct Config {
    listen_addr: String,
    upstream_addr: String,
}

impl Config {
    fn resolve() -> Config {
        Config {
            listen_addr: "0.0.0.0:8080".to_owned(),
            upstream_addr: "127.0.0.1:8081".to_owned(),
        }
    }
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_tokio_try_join() {
        async fn do_stuff_async() -> anyhow::Result<()> {
            Ok(())
        }

        async fn more_async_work() -> anyhow::Result<()> {
            Ok(())
        }

        let res = tokio::try_join!(do_stuff_async(), more_async_work());

        match res {
            Ok((_, _)) => {
                assert!(true, "join")
            }
            Err(err) => {
                println!("processing failed; error = {}", err);
            }
        }
    }

    #[tokio::test]
    async fn test_tokio_try_join_with_error() {
        async fn do_stuff_async() -> anyhow::Result<()> {
            Err(anyhow::anyhow!("do_stuff_async error"))
        }

        async fn more_async_work() -> anyhow::Result<()> {
            Err(anyhow::anyhow!("more_async_work error"))
        }

        let res = tokio::try_join!(do_stuff_async(), more_async_work());

        match res {
            Ok((_, _)) => {}
            Err(err) => {
                println!("processing failed; error = {}", err);
                assert!(true)
            }
        }
    }
}
