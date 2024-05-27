use std::fmt::Display;

use anyhow::Result;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast::{self, Receiver, Sender},
};
use tokio_util::codec::{Framed, LinesCodec};

use futures::{SinkExt, StreamExt};
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[tokio::main]
async fn main() -> Result<()> {
    // 监听端口
    // 接受客户请求
    // 将stream转为framed
    // 接受客户输入姓名
    // 等待输入
    // 接受流，进行广播
    // 接受广播，写入流
    let (tx, mut _rx) = broadcast::channel::<Msg>(16);
    let layer = tracing_subscriber::fmt::Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();
    let server_addr = "127.0.0.1:9090";

    let listen = TcpListener::bind(server_addr).await?;
    info!("Chat server listen on {server_addr}");

    loop {
        let (stream, client_addr) = listen.accept().await?;
        info!("Chat server accept client from {client_addr}");
        let tx = tx.clone();
        let rx = tx.subscribe();
        tokio::spawn(async move {
            let client_addr = client_addr.to_string();
            if let Err(e) = handle_client(stream, &client_addr, server_addr, tx, rx).await {
                error!("{e}")
            }
            info!("Client {client_addr} left");
        });
    }
}

async fn handle_client(
    stream: TcpStream,
    client_addr: &str,
    server_addr: &str,
    tx: Sender<Msg>,
    mut rx: Receiver<Msg>,
) -> Result<()> {
    let stream = Framed::new(stream, LinesCodec::new());
    let (mut stream_sender, mut stream_receiver) = stream.split();

    // 输入名称
    let user_name = loop {
        stream_sender.send("Input your name:".to_string()).await?;
        let input = match stream_receiver.next().await {
            Some(Ok(name)) => name,
            Some(Err(e)) => return Err(e.into()),
            None => return Ok(()),
        };
        if !input.is_empty() {
            break input;
        }
    };

    let sender_addr = client_addr.to_string();
    let peer_addr = sender_addr.clone();

    tokio::spawn(async move {
        let join_msg = Msg::new(sender_addr.clone(), MsgBody::joined(user_name.clone()));

        if let Err(e) = tx.send(join_msg) {
            error!("Send user: {user_name} joined message failed with error: {e}");
            return;
        }

        while let Some(line) = stream_receiver.next().await {
            let line = match line {
                Ok(line) => line,
                Err(e) => {
                    warn!("Failed to read line from {}: {}", sender_addr, e);
                    break;
                }
            };

            let chat_msg = Msg::new(sender_addr.clone(), MsgBody::chat(user_name.clone(), line));
            if let Err(e) = tx.send(chat_msg) {
                error!("Send user: {user_name} left message failed with error: {e}");
                return;
            }
        }

        let left_msg = Msg::new(sender_addr.clone(), MsgBody::left(user_name.clone()));
        if let Err(e) = tx.send(left_msg) {
            error!("Send user: {user_name} left message failed with error: {e}")
        }
    });

    loop {
        let msg = rx.recv().await;
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                warn!("Failed to read line from {}: {}", server_addr, e);
                break;
            }
        };

        if msg.sender_addr != peer_addr {
            if let Err(e) = stream_sender.send(msg.to_string()).await {
                warn!("Failed to send message to {}: {}", server_addr, e);
                break;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct Msg {
    sender_addr: String,
    msg_body: MsgBody,
}

impl Msg {
    fn new(sender_addr: String, msg_body: MsgBody) -> Self {
        Self {
            sender_addr,
            msg_body,
        }
    }
}
impl Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg_body)
    }
}

#[derive(Debug, Clone)]
enum MsgBody {
    UserJoined(String),
    UserLeft(String),
    Chat { sender: String, content: String },
}

impl MsgBody {
    fn joined(user_name: String) -> Self {
        Self::UserJoined(format!("{user_name} joined"))
    }
    fn left(user_name: String) -> Self {
        Self::UserLeft(format!("{user_name} left"))
    }
    fn chat(sender: String, content: String) -> Self {
        Self::Chat { sender, content }
    }
}

impl Display for MsgBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserJoined(content) => write!(f, "[{}]", content),
            Self::UserLeft(content) => write!(f, "[{} :(]", content),
            Self::Chat { sender, content } => write!(f, "{}: {}", sender, content),
        }
    }
}
