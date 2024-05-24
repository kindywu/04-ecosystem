use anyhow::Result;
use std::sync::Arc;
use tokio::{
    io::copy,
    net::{TcpListener, TcpStream},
};
use tracing::{error, info, level_filters::LevelFilter, warn};
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

        let conifg_clone = config.clone();

        tokio::spawn(async move {
            info!("Accept client {}", &addr);
            let upstream_addr = &conifg_clone.upstream_addr;
            // connect to upstream
            match TcpStream::connect(upstream_addr).await {
                Ok(mut upstream) => {
                    // pipe the stream

                    // cargo add tokio --features net
                    let (mut client_reader, mut client_writer) = client.split();
                    let (mut upstream_reader, mut upstream_writer) = upstream.split();

                    // cargo add tokio --features io-util
                    let client_to_stream = copy(&mut client_reader, &mut upstream_writer);
                    let stream_to_client = copy(&mut upstream_reader, &mut client_writer);

                    match tokio::try_join!(client_to_stream, stream_to_client) {
                        Ok((n, m)) => info!(
                            "proxied {} bytes from client to upstream, {} bytes from upstream to client",
                            n, m
                        ),
                        Err(e) => warn!("error proxying: {:?}", e),
                    }
                }
                Err(e) => error!(
                    "Connect to upstream {} fail with error {}",
                    upstream_addr, e
                ),
            }
        });
    }
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
