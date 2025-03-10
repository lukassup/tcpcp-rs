# TCP Connection Pool

A simple TCP connection pooling implentation for reusing connections between
threads or async contexts in Rust.

- Resource pool implementation done with `deadpool-rs` library.
- TCP connections done using `async_std` library.
- random selection of DNS records using `rand` library.


```sh
cargo run
[src/main.rs:114:5] &pool.manager().addrs = [
    1.0.0.1:80,
    1.1.1.1:80,
]
192.168.88.100:44980 -> 1.0.0.1:80
192.168.88.100:44980 <- 1.0.0.1:80
[src/main.rs:129:5] drop(conn1) = ()
192.168.88.100:44980 -> 1.0.0.1:80
192.168.88.100:44980 <- 1.0.0.1:80
192.168.88.100:51600 -> 1.1.1.1:80
192.168.88.100:51600 <- 1.1.1.1:80
```