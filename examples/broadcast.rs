use anyhow::Result;
use tokio::{sync::broadcast, task::JoinHandle, try_join};
#[tokio::main]
async fn main() {
    let (tx, mut _rx) = broadcast::channel(16);
    let tx2 = tx.clone();
    let mut rx1 = tx.subscribe();
    let mut rx2 = tx2.subscribe();

    let t1 = tokio::spawn(async move {
        // loop {
        //     let i = rx1.recv().await.unwrap();
        //     println!("{i}")
        // }
        assert_eq!(rx1.recv().await.unwrap(), 10);
        assert_eq!(rx1.recv().await.unwrap(), 20);
    });

    let t2 = tokio::spawn(async move {
        assert_eq!(rx2.recv().await.unwrap(), 10);
        assert_eq!(rx2.recv().await.unwrap(), 20);
    });

    tx.send(10).unwrap();
    tx2.send(20).unwrap();

    match try_join!(flatten(t1), flatten(t2)) {
        Ok(_) => {
            println!("finish");
        }
        Err(err) => {
            println!("Failed with {}.", err);
        }
    }
}

async fn flatten(handle: JoinHandle<()>) -> Result<()> {
    match handle.await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.into()),
    }
}
