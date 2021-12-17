use crate::Result;
use crate::{
    err,
    io::{Read, ReadExt, Write, WriteExt},
};
use serde::{de::DeserializeOwned, Serialize};

use super::formats::{ReadFormat, SendFormat};

pub async fn tx<T, O, F: SendFormat>(st: &mut T, obj: O) -> Result<usize>
where
    T: Write + Unpin,
    O: Serialize,
{
    let serialized = F::serialize(&obj)?;
    let len: [u8; 8] = u64::to_be_bytes(serialized.len() as u64);
    st.write(&len).await?;
    // return length of object sent
    let len = st.write(&serialized).await?;
    st.flush().await?;
    Ok(len)
}

pub async fn rx<T, O, F: ReadFormat>(st: &mut T) -> Result<O>
where
    T: Read + Unpin,
    O: DeserializeOwned,
{
    let mut size_buffer = [0u8; 8];
    st.read_exact(&mut size_buffer).await?;
    let size = u64::from_be_bytes(size_buffer);
    // this is done for fallibility, we don't want people sending in usize::MAX
    // as the len unexpectedly crashing the program
    let mut buf = Vec::new();
    buf.try_reserve(size as usize).or_else(|e| {
        err!((
            out_of_memory,
            format!("failed to reserve {:?} bytes, error: {:?}", size, e)
        ))
    })?;
    buf.resize(size as usize, 0);
    // read message into buffer
    st.read_exact(&mut buf).await?;
    F::deserialize(&buf)
}
