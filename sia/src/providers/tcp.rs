use crate::routes::Status;
use crate::routes::GLOBAL_ROUTE;
use crate::runtime;
use crate::runtime::JoinHandle;
use crate::Result;
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use igcp::err;
use igcp::BareChannel;
use igcp::Channel;

pub struct Tcp(TcpListener);

impl Tcp {
    pub async fn bind(addrs: &str) -> Result<JoinHandle<Result<()>>> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                runtime::spawn(async move {
                    let chan: Channel = Channel::new_tcp_encrypted(stream).await?;
                    let chan: BareChannel = chan.bare();
                    GLOBAL_ROUTE.introduce_static_unspawn(chan).await?;
                    Ok::<_, std::io::Error>(())
                });
            }
        }))
    }
    pub async fn raw_connect_with_retries(
        addrs: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let stream = loop {
            match TcpStream::connect(addrs).await {
                Ok(s) => break s,
                Err(e) => {
                    log::error!(
                        "connecting to address `{}` failed, attempt {} starting",
                        addrs,
                        attempt
                    );
                    async_std::task::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                    attempt += 1;
                    if attempt == retries {
                        err!((e))?
                    }
                    continue;
                }
            }
        };
        let chan = Channel::new_tcp_encrypted(stream).await?;
        Ok(chan)
    }
    pub async fn connect(addrs: &str, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    pub async fn connect_retry(
        addrs: &str,
        id: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut c = Self::raw_connect_with_retries(addrs, retries, time_to_retry).await?;
        c.tx(id).await?;
        match c.rx().await? {
            Status::Found => Ok(c),
            Status::NotFound => err!((
                not_found,
                format!("id `{}` not found at address `{}`", id, addrs)
            )),
        }
    }
}

pub struct InsecureTcp(TcpListener);

impl InsecureTcp {
    pub async fn bind(addrs: &str) -> Result<JoinHandle<Result<()>>> {
        let listener = TcpListener::bind(addrs).await?;
        Ok(runtime::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await?;
                let chan = BareChannel::InsecureTcp(stream);
                GLOBAL_ROUTE.introduce_static(chan);
            }
        }))
    }
    pub async fn raw_connect_with_retries(
        addrs: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut attempt = 0;
        let stream = loop {
            match TcpStream::connect(addrs).await {
                Ok(s) => break s,
                Err(e) => {
                    log::error!(
                        "connecting to address `{}` failed, attempt {} starting",
                        addrs,
                        attempt
                    );
                    async_std::task::sleep(std::time::Duration::from_millis(time_to_retry)).await;
                    attempt += 1;
                    if attempt == retries {
                        err!((e))?
                    }
                    continue;
                }
            }
        };
        Ok(Channel::InsecureTcp(stream))
    }
    pub async fn connect(addrs: &str, id: &str) -> Result<Channel> {
        Self::connect_retry(addrs, id, 3, 10).await
    }
    pub async fn connect_retry(
        addrs: &str,
        id: &str,
        retries: u32,
        time_to_retry: u64,
    ) -> Result<Channel> {
        let mut c = Self::raw_connect_with_retries(addrs, retries, time_to_retry).await?;
        c.tx(id).await?;
        match c.rx().await? {
            Status::Found => Ok(c),
            Status::NotFound => err!((
                not_found,
                format!("id `{}` not found at address `{}`", id, addrs)
            )),
        }
    }
}
