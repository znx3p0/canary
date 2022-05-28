#![allow(unused)]
//! complete zero cost wrappers over network communications

use crate::io::{Read, ReadExt, Write, WriteExt};
use crate::{err, Result};

#[inline]
pub(crate) fn try_vec<T: Default + Clone>(size: usize) -> Result<Vec<T>> {
    let mut buf = Vec::new();
    buf.try_reserve(size as usize).map_err(|e| {
        err!(
            out_of_memory,
            format!("failed to reserve {} bytes, error: {:?}", size, e)
        )
    })?;
    buf.resize(size as usize, T::default());
    Ok(buf)
}

#[inline]
pub(crate) async fn send_u8<T: Write + Unpin>(st: &mut T, obj: u8) -> Result<()> {
    st.write_all(&u8::to_be_bytes(obj)).await?;
    Ok(())
}

#[inline]
pub(crate) async fn send_u16<T: Write + Unpin>(st: &mut T, obj: u16) -> Result<()> {
    st.write_all(&u16::to_be_bytes(obj)).await?;
    Ok(())
}

#[inline]
pub(crate) async fn send_u32<T: Write + Unpin>(st: &mut T, obj: u32) -> Result<()> {
    st.write_all(&u32::to_be_bytes(obj)).await?;
    Ok(())
}

#[inline]
pub(crate) async fn send_u64<T: Write + Unpin>(st: &mut T, obj: u64) -> Result<()> {
    st.write_all(&u64::to_be_bytes(obj)).await?;
    Ok(())
}

#[inline]
pub(crate) async fn read_u8<T: Read + Unpin>(st: &mut T) -> Result<u8> {
    let mut buf = [0u8; 1];
    st.read_exact(&mut buf).await?;
    Ok(u8::from_be_bytes(buf))
}

#[inline]
pub(crate) async fn read_u16<T: Read + Unpin>(st: &mut T) -> Result<u16> {
    let mut buf = [0u8; 2];
    st.read_exact(&mut buf).await?;
    Ok(u16::from_be_bytes(buf))
}

#[inline]
pub(crate) async fn read_u32<T: Read + Unpin>(st: &mut T) -> Result<u32> {
    let mut buf = [0u8; 4];
    st.read_exact(&mut buf).await?;
    Ok(u32::from_be_bytes(buf))
}

#[inline]
pub(crate) async fn read_u64<T: Read + Unpin>(st: &mut T) -> Result<u64> {
    let mut buf = [0u8; 8];
    st.read_exact(&mut buf).await?;
    Ok(u64::from_be_bytes(buf))
}
