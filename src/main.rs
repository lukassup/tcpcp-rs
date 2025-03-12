use tcpconnpool::*;

#[async_std::main]
async fn main() -> io::Result<()> {
    let host = "one.one.one.one";
    let port: u16 = 80;
    let pool = TcpConnectionPool::builder(
        TcpConnectionManager::new(
            format!("{host}:{port}"),
            AddressFamily::IPv4,
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
