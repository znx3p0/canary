use crate::io::{Read, ReadExt, Write, WriteExt};
use crate::Result;
use serde::{de::DeserializeOwned, Serialize};

use super::formats::{ReadFormat, SendFormat};
use super::zc;

pub async fn tx<T, O, F: SendFormat>(st: &mut T, obj: O) -> Result<usize>
where
    T: Write + Unpin,
    O: Serialize,
{
    let serialized = F::serialize(&obj)?;
    zc::send_u32(st, serialized.len() as u32).await?;
    // return length of object sent
    st.write_all(&serialized).await?;
    st.flush().await?;
    Ok(serialized.len())
}

pub async fn rx<T, O, F: ReadFormat>(st: &mut T) -> Result<O>
where
    T: Read + Unpin,
    O: DeserializeOwned,
{
    let size = zc::read_u32(st).await?;
    // this is done for fallibility, we don't want people sending in usize::MAX
    // as the len unexpectedly crashing the program
    let mut buf = zc::try_vec(size as usize)?;
    buf.resize(size as usize, 0);
    // read message into buffer
    st.read_exact(&mut buf).await?;
    F::deserialize(&buf)
}
