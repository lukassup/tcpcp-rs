pub use async_std::io;
use async_std::io::ReadExt;
pub use async_std::prelude::*;

use async_std::net::SocketAddr;
use async_std::net::TcpStream;
use async_std::net::ToSocketAddrs;
use async_std::sync::Arc;
use async_std::task;
use core::time;
use deadpool::managed;
use rand::seq::IndexedRandom;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

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
pub type TcpConnectionObject = managed::Object<TcpConnectionManager>;

#[derive(Debug)]
pub struct TcpConnectionManager {
    pub addrs: Vec<SocketAddr>,
    addr_index_next: Arc<AtomicUsize>,
    lb_algorithm: LoadBalancing,
    max_requests: usize,
}

impl TcpConnectionManager {
    pub async fn new(
        addrs: impl ToSocketAddrs,
        address_family: AddressFamily,
        lb_algorithm: LoadBalancing,
        max_requests: usize,
    ) -> io::Result<Self> {
        Ok(Self {
            addr_index_next: Arc::new(AtomicUsize::new(0)),
            addrs: addrs
                .to_socket_addrs()
                .await?
                .filter(|a| match address_family {
                    AddressFamily::Any => true,
                    AddressFamily::IPv4 => a.is_ipv4(),
                    AddressFamily::IPv6 => a.is_ipv6(),
                })
                .collect(),
            lb_algorithm,
            max_requests,
        })
    }

    /// Selects and returns a SocketAddr using configured lb_algorithm
    pub async fn get_addr(&self) -> SocketAddr {
        match self.lb_algorithm {
            LoadBalancing::First => *self.addrs.first().unwrap(),
            LoadBalancing::Random => {
                let mut rng = rand::rng();
                *self.addrs.choose(&mut rng).unwrap()
            }
            LoadBalancing::Next => {
                // wrapping atomic increment for async/thread safety
                let curr_index = self
                    .addr_index_next
                    .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |i| {
                        Some((i + 1) % self.addrs.len())
                    })
                    .unwrap();
                *self.addrs.get(curr_index).unwrap()
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
        stream: &mut Self::Type,
        metrics: &managed::Metrics,
    ) -> managed::RecycleResult<Self::Error> {
        if metrics.recycle_count + 1 >= self.max_requests {
            return Err(managed::RecycleError::message("max requests reached"));
        }
        // try reading zero bytes
        let Err(e) = stream.read(&mut []).await else {
            return Ok(());
        };
        match e.kind() {
            // retryable - connection reused
            io::ErrorKind::WouldBlock => Ok(()),
            // non-retryable - new connection created
            _ => {
                task::sleep(time::Duration::from_millis(3000)).await;
                Err(managed::RecycleError::from(dbg!(e)))
            }
        }
    }
}
