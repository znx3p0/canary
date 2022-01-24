#![allow(unused)]
//! complete zero cost wrappers over network communications

use crate::{err, Result};
use async_std::io::{Read, ReadExt, Write, WriteExt};
use cfg_if::cfg_if;
pub(crate) fn try_vec<T: Default + Clone>(size: usize) -> Result<Vec<T>> {
    let mut buf = Vec::new();
    buf.try_reserve(size as usize).or_else(|e| {
        err!((
            out_of_memory,
            format!("failed to reserve {} bytes, error: {:?}", size, e)
        ))
    })?;
    buf.resize(size as usize, T::default());
    Ok(buf)
}

cfg_if! {
    if #[cfg(not(feature = "handwritten-futures"))] {
        pub(crate) async fn send_u8<T: Write + Unpin>(st: &mut T, obj: u8) -> Result<()> {
            st.write_all(&u8::to_be_bytes(obj)).await?;
            Ok(())
        }
        pub(crate) async fn send_u16<T: Write + Unpin>(st: &mut T, obj: u16) -> Result<()> {
            st.write_all(&u16::to_be_bytes(obj)).await?;
            Ok(())
        }
        pub(crate) async fn send_u32<T: Write + Unpin>(st: &mut T, obj: u32) -> Result<()> {
            st.write_all(&u32::to_be_bytes(obj)).await?;
            Ok(())
        }
        pub(crate) async fn send_u64<T: Write + Unpin>(st: &mut T, obj: u64) -> Result<()> {
            st.write_all(&u64::to_be_bytes(obj)).await?;
            Ok(())
        }
        pub(crate) async fn read_u8<T: Read + Unpin>(st: &mut T) -> Result<u8> {
            let mut buf = [0u8; 1];
            st.read_exact(&mut buf).await?;
            Ok(u8::from_be_bytes(buf))
        }
        pub(crate) async fn read_u16<T: Read + Unpin>(st: &mut T) -> Result<u16> {
            let mut buf = [0u8; 2];
            st.read_exact(&mut buf).await?;
            Ok(u16::from_be_bytes(buf))
        }
        pub(crate) async fn read_u32<T: Read + Unpin>(st: &mut T) -> Result<u32> {
            let mut buf = [0u8; 4];
            st.read_exact(&mut buf).await?;
            Ok(u32::from_be_bytes(buf))
        }
        pub(crate) async fn read_u64<T: Read + Unpin>(st: &mut T) -> Result<u64> {
            let mut buf = [0u8; 8];
            st.read_exact(&mut buf).await?;
            Ok(u64::from_be_bytes(buf))
        }
    } else {
        macro_rules! implement_read {
            (
                $vis: vis fn $fn_name: ident -> $ret: ident < $int_ty: ty > where size = $e: expr, from $from: path;
            ) => {
                $vis fn $fn_name<'a, T: Read + Unpin>(st: &'a mut T) -> $ret<'a, T> {
                    $ret(st)
                }
                $vis struct $ret<'a, T: Read + Unpin>(&'a mut T);
                impl<'a, T: Read + Unpin> std::future::Future for $ret<'a, T> {
                    type Output = Result<$int_ty>;
                    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                        let stream = &mut self.0;
                        let mut buf = [0u8; $e];
                        let fut = stream.read_exact(&mut buf);
                        futures::pin_mut!(fut);
                        match fut.poll(cx) {
                            std::task::Poll::Ready(res) => {
                                match res {
                                    Ok(_) => std::task::Poll::Ready(Ok($from(buf))),
                                    Err(e) => std::task::Poll::Ready(Err(e.into()))
                                }
                            },
                            std::task::Poll::Pending => std::task::Poll::Pending,
                        }
                    }
                }
            };
        }
        macro_rules! implement_send {
            (
                $vis: vis fn $fn_name: ident -> $ret: ident < $int_ty: ty > where size = $e: expr, from $from: path;
            ) => {
                $vis fn $fn_name<'a, T: Write + Unpin>(st: &'a mut T, num: $int_ty) -> $ret<'a, T> {
                    $ret(st, num)
                }
                pub(crate) struct $ret<'a, T: Write + Unpin>(&'a mut T, $int_ty);
                impl<'a, T: Write + Unpin> std::future::Future for $ret<'a, T> {
                    type Output = Result<()>;
                    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
                        let buf = &$from(self.1);
                        let stream = &mut self.0;
                        let fut = stream.write_all(buf);
                        futures::pin_mut!(fut);
                        match fut.poll(cx) {
                            std::task::Poll::Ready(res) => {
                                match res {
                                    Ok(_) => std::task::Poll::Ready(Ok(())),
                                    Err(e) => std::task::Poll::Ready(Err(e.into()))
                                }
                            },
                            std::task::Poll::Pending => std::task::Poll::Pending,
                        }
                    }
                }
            };
        }

        implement_read! {
            pub(crate) fn read_u128 -> ReadU128<u128> where size = 16, from u128::from_be_bytes;
        }
        implement_read! {
            pub(crate) fn read_u64 -> ReadU64<u64> where size = 8, from u64::from_be_bytes;
        }
        implement_read! {
            pub(crate) fn read_u32 -> ReadU32<u32> where size = 4, from u32::from_be_bytes;
        }
        implement_read! {
            pub(crate) fn read_u16 -> ReadU16<u16> where size = 2, from u16::from_be_bytes;
        }
        implement_read! {
            pub(crate) fn read_u8 -> ReadU8<u8> where size = 1, from u8::from_be_bytes;
        }
        implement_send! {
            pub(crate) fn send_u128 -> SendU128<u128> where size = 16, from u128::to_be_bytes;
        }
        implement_send! {
            pub(crate) fn send_u64 -> SendU64<u64> where size = 8, from u64::to_be_bytes;
        }
        implement_send! {
            pub(crate) fn send_u32 -> SendU32<u32> where size = 4, from u32::to_be_bytes;
        }
        implement_send! {
            pub(crate) fn send_u16 -> SendU16<u16> where size = 2, from u16::to_be_bytes;
        }
        implement_send! {
            pub(crate) fn send_u8 -> SendU8<u8> where size = 1, from u8::to_be_bytes;
        }
    }
}
