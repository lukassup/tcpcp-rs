// cargo add async_std --features attributes
// cargo add deadpool --features rt_async-std_1
// cargo add rand

use async_std::io;
use async_std::net::SocketAddr;
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::prelude::*;
use async_std::sync::Mutex;
use deadpool::managed;
use rand::seq::IndexedRandom;
use std::sync::Arc;

#[derive(Debug)]
pub enum AddressFamily {
    Any,
    IPv4,
    IPv6,
}

#[derive(Debug)]
pub enum LoadBalancing {
    /// Choose the first address from resolved DNS records
    First,
    /// Randomly choose an address from resolved DNS records
    Random,
    /// Choose addresses in order from resolved DNS records
    Next,
}

#[derive(Debug)]
pub struct TcpConnectionManager {
    pub addrs: Vec<SocketAddr>,
    addr_index_next: Arc<Mutex<usize>>,
    lb_algorithm: LoadBalancing,
}

impl TcpConnectionManager {
    pub async fn new(
        addrs: impl ToSocketAddrs,
        address_family: AddressFamily,
        lb_algorithm: LoadBalancing,
    ) -> io::Result<Self> {
        Ok(Self {
            addrs: addrs
                .to_socket_addrs()
                .await?
                .filter(|a| match address_family {
                    AddressFamily::Any => true,
                    AddressFamily::IPv4 => a.is_ipv4(),
                    AddressFamily::IPv6 => a.is_ipv6(),
                })
                .collect(),
            addr_index_next: Arc::new(Mutex::new(0)),
            lb_algorithm,
        })
    }

    pub async fn get_addr(&self) -> SocketAddr {
        match self.lb_algorithm {
            LoadBalancing::First => *self.addrs.first().unwrap(),
            LoadBalancing::Random => {
                let mut rng = rand::rng();
                *self.addrs.choose(&mut rng).unwrap()
            }
            LoadBalancing::Next => {
                // Return self.addrs[self.addr_index_next]
                // Do wrapping increment with Mutex::lock() to prevent multiple
                // threads from updating it
                let mut curr_index = self.addr_index_next.lock().await;
                let addr = self.addrs.get(*curr_index).unwrap();
                *curr_index = (*curr_index + 1) % self.addrs.len();
                *addr
            }
        }
    }
}

impl managed::Manager for TcpConnectionManager {
    type Type = TcpStream;
    type Error = io::Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        let addr = self.get_addr().await;
        Self::Type::connect(addr).await
    }

    async fn recycle(
        &self,
        _: &mut Self::Type,
        _: &managed::Metrics,
    ) -> managed::RecycleResult<Self::Error> {
        Ok(())
    }
}

type TcpConnectionPool = managed::Pool<TcpConnectionManager>;

#[async_std::main]
async fn main() -> io::Result<()> {
    let host = "one.one.one.one";
    let port: u16 = 80;
    let pool = TcpConnectionPool::builder(
        TcpConnectionManager::new(
            format!("{host}:{port}"),
            AddressFamily::Any,
            LoadBalancing::Random,
        )
        .await?,
    )
    .build()
    .unwrap();
    dbg!(&pool.manager().addrs);

    // acquire connection from pool
    let mut conn1 = pool.get().await.unwrap();
    let local_addr = conn1.local_addr()?;
    let remote_addr = conn1.peer_addr()?;
    conn1
        .write_all(format!("GET / HTTP/1.0\r\nHost: {host}\r\n\r\n").as_bytes())
        .await?;
    println!("{local_addr} -> {remote_addr}");
    let mut response_buf: String = String::new();
    let _rx_bytes = conn1.read_to_string(&mut response_buf).await?;
    println!("{local_addr} <- {remote_addr}");

    // on drop, pool does not close the connection
    dbg!(drop(conn1));

    // on acquire, existing connection is used
    let mut conn2 = pool.get().await.unwrap();
    let local_addr = conn2.local_addr()?;
    let remote_addr = conn2.peer_addr()?;
    conn2
        .write_all(format!("GET / HTTP/1.0\r\nHost: {host}\r\n\r\n").as_bytes())
        .await?;
    println!("{local_addr} -> {remote_addr}");
    let mut response_buf: String = String::new();
    let _rx_bytes = conn2.read_to_string(&mut response_buf).await?;
    println!("{local_addr} <- {remote_addr}");

    // a new connection will be opened
    let mut conn3 = pool.get().await.unwrap();
    let local_addr = conn3.local_addr()?;
    let remote_addr = conn3.peer_addr()?;
    conn3
        .write_all(format!("GET / HTTP/1.0\r\nHost: {host}\r\n\r\n").as_bytes())
        .await?;
    println!("{local_addr} -> {remote_addr}");
    let mut response_buf: String = String::new();
    let _rx_bytes = conn3.read_to_string(&mut response_buf).await?;
    println!("{local_addr} <- {remote_addr}");

    Ok(())
}
