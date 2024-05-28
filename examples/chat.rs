use std::{fmt::Display, net::SocketAddr, str::FromStr, sync::Arc};

use anyhow::Result;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast::{self, Receiver, Sender},
};
use tokio_util::codec::{Framed, LinesCodec};

use futures::{stream::SplitStream, SinkExt, StreamExt};
use tracing::{error, info, level_filters::LevelFilter, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

const MAX_MESSAGES: usize = 128;

// 监听端口
// 接受客户请求
// 将stream转为framed
// 接受客户输入姓名
// 等待输入
// 接受网络流，进行广播
// 接受广播，写入网络流（过滤发送者）
#[tokio::main]
async fn main() -> Result<()> {
    let (tx, mut _rx) = broadcast::channel::<Arc<Msg>>(MAX_MESSAGES);
    let layer = tracing_subscriber::fmt::Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let server_socket: SocketAddr = SocketAddr::from_str("127.0.0.1:9090")?;

    let listen = TcpListener::bind(server_socket).await?;
    info!("Chat server listen on {server_socket}");

    loop {
        let (stream, client_socket) = listen.accept().await?;
        info!("Chat server accept client from {client_socket}");
        let tx = tx.clone();
        let rx = tx.subscribe();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, client_socket, server_socket, tx, rx).await {
                error!("{e}")
            }
            info!("Client {client_socket} left");
        });
    }
}

async fn handle_client(
    stream: TcpStream,
    client_socket: SocketAddr,
    _server_socket: SocketAddr,
    tx: Sender<Arc<Msg>>,
    mut rx: Receiver<Arc<Msg>>,
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

    let join_msg = Arc::new(Msg::new(client_socket, MsgBody::joined(user_name.clone())));

    if let Err(e) = tx.send(join_msg) {
        error!("Send user: {user_name} joined message failed with error: {e}");
        return Ok(());
    }

    tokio::spawn(async move {
        handle_msg_from_client(user_name, client_socket, tx, stream_receiver).await;
    });

    // handle_send_msg_to_client
    loop {
        let msg = rx.recv().await;
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                warn!("Failed to read msg from rx: {}", e);
                break;
            }
        };

        if msg.sender_socket != client_socket {
            if let Err(e) = stream_sender.send(msg.to_string()).await {
                warn!("Failed to send message to stream_sender: {}", e);
                break;
            }
        } else if let MsgBody::UserLeft(_) = msg.msg_body {
            break;
        }
    }

    Ok(())
}

async fn handle_msg_from_client(
    user_name: String,
    client_socket: SocketAddr,
    tx: Sender<Arc<Msg>>,
    mut stream_receiver: SplitStream<Framed<TcpStream, LinesCodec>>,
) {
    while let Some(line) = stream_receiver.next().await {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                warn!("Failed to read line from stream_receiver: {}", e);
                break;
            }
        };

        let chat_msg = Arc::new(Msg::new(
            client_socket,
            MsgBody::chat(user_name.clone(), line),
        ));
        if let Err(e) = tx.send(chat_msg) {
            warn!("Failed to send msg to user:{} tx: {}", user_name, e);
            return;
        }
    }

    let left_msg = Arc::new(Msg::new(client_socket, MsgBody::left(user_name.clone())));
    if let Err(e) = tx.send(left_msg) {
        error!("Send user: {user_name} left message failed with error: {e}")
    }
}

#[derive(Debug)]
struct Msg {
    sender_socket: SocketAddr,
    msg_body: MsgBody,
}

impl Msg {
    fn new(sender_socket: SocketAddr, msg_body: MsgBody) -> Self {
        Self {
            sender_socket,
            msg_body,
        }
    }
}
impl Display for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg_body)
    }
}

#[derive(Debug)]
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
