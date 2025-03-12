pub use async_std::io;
pub use async_std::prelude::*;

use async_std::net::SocketAddr;
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::sync::Arc;
use async_std::sync::Mutex;
use deadpool::managed;
use rand::seq::IndexedRandom;

/// Address family preference
#[derive(Debug)]
pub enum AddressFamily {
    /// Either IPv4 or IPv6
    Any,
    /// IPv4 only
    IPv4,
    /// IPv6 only
    IPv6,
}

/// Connection load balancing algorithm
#[derive(Debug)]
pub enum LoadBalancing {
    /// Choose the first address from resolved DNS records
    First,
    /// Randomly choose an address from resolved DNS records
    Random,
    /// Choose addresses in order from resolved DNS records
    Next,
}

pub type TcpConnectionPool = managed::Pool<TcpConnectionManager>;

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
        dbg!(addr);
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
