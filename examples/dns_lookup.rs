use dns_lookup::lookup_host;

// use tokio::net::lookup_host;

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let hostname = "www.bing.com";
//     for addr in lookup_host(hostname).await? {
//         println!("socket address is {}", addr);
//     }
//     Ok(())
// }

fn main() {
    let hostname = "www.bing.com";
    let ips: Vec<std::net::IpAddr> = lookup_host(hostname).unwrap();
    println!("{:?}", ips);
}
