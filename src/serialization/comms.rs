use crate::io::{Read, ReadExt, Write, WriteExt};
use crate::{err, Result};

use futures::SinkExt;
use futures::StreamExt;
use serde::{de::DeserializeOwned, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::io::wss::tungstenite::Message;

#[cfg(target_arch = "wasm32")]
use reqwasm::websocket::Message;

use super::formats::{ReadFormat, SendFormat};
use super::zc;

/// send an item through the stream
pub async fn tx<T, O, F: SendFormat>(st: &mut T, obj: O, f: &F) -> Result<usize>
where
    T: Write + Unpin,
    O: Serialize,
{
    let serialized = f.serialize(&obj)?;
    zc::send_u64(st, serialized.len() as _).await?;
    // return length of object sent
    st.write_all(&serialized).await?;
    st.flush().await?;
    Ok(serialized.len())
}

/// receive an item from the stream
pub async fn rx<T, O, F: ReadFormat>(st: &mut T, f: &F) -> Result<O>
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
    f.deserialize(&buf)
}

#[cfg(not(target_arch = "wasm32"))]
/// send a message from a websocket stream
pub async fn wss_tx<T, O, F: SendFormat>(st: &mut T, obj: O, f: &F) -> Result<usize>
where
    T: futures::prelude::Sink<Message> + Unpin,
    O: Serialize,
    <T as futures::prelude::Sink<Message>>::Error: ToString,
{
    let serialized = f.serialize(&obj)?;
    let len = serialized.len();
    let msg = Message::Binary(serialized);
    st.feed(msg).await.map_err(|e| err!(e.to_string()))?;
    st.flush().await.map_err(|e| err!(e.to_string()))?;
    Ok(len)
}

#[cfg(target_arch = "wasm32")]
/// send a message from a websocket stream
pub async fn wss_tx<T, O, F: SendFormat>(st: &mut T, obj: O, f: &F) -> Result<usize>
where
    T: futures::prelude::Sink<Message> + Unpin,
    O: Serialize,
    <T as futures::prelude::Sink<Message>>::Error: ToString,
{
    let serialized = f.serialize(&obj)?;
    let len = serialized.len();
    let msg = Message::Bytes(serialized);
    st.feed(msg).await.map_err(|e| err!(e.to_string()))?;
    st.flush().await.map_err(|e| err!(e.to_string()))?;
    Ok(len)
}

#[cfg(not(target_arch = "wasm32"))]
/// receive a message from a websocket stream
pub async fn wss_rx<T, O, F: ReadFormat>(st: &mut T, f: &F) -> Result<O>
where
    T: futures::prelude::Stream<
            Item = std::result::Result<Message, crate::io::wss::tungstenite::error::Error>,
        > + Unpin,
    O: DeserializeOwned,
{
    let msg = st
        .next()
        .await
        .ok_or(err!(broken_pipe, "websocket connection broke"))?
        .map_err(|e| err!(broken_pipe, e))?;

    match msg {
        Message::Binary(vec) => f.deserialize(&vec),
        Message::Text(_) => err!((invalid_data, "expected binary message, found text message")),
        Message::Ping(_) => err!((invalid_data, "expected binary message, found ping message")),
        Message::Pong(_) => err!((invalid_data, "expected binary message, found pong message")),
        Message::Close(_) => err!((invalid_data, "expected binary message, found close message")),
        Message::Frame(_) => err!((invalid_data, "expected binary message, found frame")),
    }
}

#[cfg(target_arch = "wasm32")]
/// receive a message from a websocket stream
pub async fn wss_rx<T, O, F: ReadFormat>(st: &mut T, f: &F) -> Result<O>
where
    T: futures::prelude::Stream<
            Item = std::result::Result<Message, reqwasm::websocket::WebSocketError>,
        > + Unpin,
    O: DeserializeOwned,
{
    let msg = st
        .next()
        .await
        .ok_or(err!(broken_pipe, "websocket connection broke"))?
        .map_err(|e| err!(broken_pipe, e.to_string()))?;

    match msg {
        Message::Bytes(vec) => f.deserialize(&vec),
        Message::Text(_) => err!((invalid_data, "expected binary data, found text")),
    }
}
