use crate::io::{Read, ReadExt, Write, WriteExt};
use crate::{Result, err};
use async_tungstenite::tungstenite::Message;
use futures::SinkExt;
use futures_lite::StreamExt;
use serde::{de::DeserializeOwned, Serialize};

use super::formats::{ReadFormat, SendFormat};
use super::zc;

/// send an item through the stream
pub async fn tx<T, O, F: SendFormat>(st: &mut T, obj: O) -> Result<usize>
where
    T: Write + Unpin,
    O: Serialize,
{
    let serialized = F::serialize(&obj)?;
    zc::send_u64(st, serialized.len() as _).await?;
    // return length of object sent
    st.write_all(&serialized).await?;
    st.flush().await?;
    Ok(serialized.len())
}

/// receive an item from the stream
pub async fn rx<T, O, F: ReadFormat>(st: &mut T) -> Result<O>
where
    T: Read + Unpin,
    O: DeserializeOwned,
{
    let size = zc::read_u64(st).await?;
    // this is done for fallibility, we don't want people sending in usize::MAX
    // as the len unexpectedly crashing the program
    let mut buf = zc::try_vec(size as usize)?;
    // read message into buffer
    st.read_exact(&mut buf).await?;
    F::deserialize(&buf)
}


/// send a message from a websocket stream
pub async fn wss_tx<T, O, F: SendFormat>(st: &mut T, obj: O) -> Result<usize>
where
    T: futures::prelude::Sink<Message> + Unpin,
    O: Serialize,
{
    let serialized = F::serialize(&obj)?;
    let len = serialized.len();
    let msg = Message::Binary(serialized);
    st.feed(msg).await
        .map_err(|_| err!(broken_pipe, "websocket connection broke, unable to send message"))?;
    st.flush().await
        .map_err(|_| err!(broken_pipe, "websocket connection broke, unable to send message"))?;
    Ok(len)
}

/// receive a message from a websocket stream
pub async fn wss_rx<T, O, F: ReadFormat>(st: &mut T) -> Result<O>
where
    T: futures::prelude::Stream<Item = std::result::Result<Message, async_tungstenite::tungstenite::Error>> + Unpin,
    O: DeserializeOwned,
{
    let msg = st.next().await
        .ok_or(err!(broken_pipe, "websocket connection broke"))?
        .map_err(|e| err!(broken_pipe, e))?;
    match msg {
        Message::Binary(vec) => F::deserialize(&vec),
        Message::Text(_) => err!((invalid_data, "expected binary message, found text message")),
        Message::Ping(_) => err!((invalid_data, "expected binary message, found ping message")),
        Message::Pong(_) => err!((invalid_data, "expected binary message, found pong message")),
        Message::Close(_) => err!((invalid_data, "expected binary message, found close message")),
    }
}
