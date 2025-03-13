// use async_std::task;
// use core::time;

use tcpconnpool::*;

#[async_std::main]
async fn main() -> io::Result<()> {
    // let host = "one.one.one.one";
    // let ka = "keep-alive";
    let host = "raw.githubusercontent.com";
    let ka = "close";
    let ua = "curl/8.7.1";
    let port: u16 = 80;
    let req = format!(
        "GET / HTTP/1.1\r\n\
        Host: {host}\r\n\
        Connection: {ka}\r\n\
        User-Agent: {ua}\r\n\
        Accept: */*\r\n\
        \r\n\
        "
    );
    let pool = TcpConnectionPool::builder(
        TcpConnectionManager::new(
            format!("{host}:{port}"),
            AddressFamily::IPv4,
            LoadBalancing::Next,
            10, // max_requests: 1 - disable keepalive
        )
        .await?,
    )
    .build()
    .unwrap();
    dbg!(&pool.manager().addrs);

    for _ in 0..10 {
        // acquire connection from pool
        let cx = &mut pool.get().await.unwrap();
        let local_addr = cx.local_addr()?;
        let remote_addr = cx.peer_addr()?;
        println!("{local_addr} -> {remote_addr}");

        // task::sleep(time::Duration::from_millis(10)).await; // #1

        // dbg!(&req);
        cx.write_all(req.as_bytes()).await?;
        cx.flush().await?;

        // task::sleep(time::Duration::from_millis(20)).await; // #2

        let mut response_buf = String::new();
        println!("{local_addr} <- {remote_addr}");
        let rx_bytes = cx.read_to_string(&mut response_buf).await?;
        dbg!(rx_bytes);
        // dbg!(response_buf);

        // task::sleep(time::Duration::from_millis(40)).await; // #3

        // force release connection
        // let _ = TcpConnectionObject::take(cx);
    }
    Ok(())
}
