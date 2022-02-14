use crate::err;
use crate::io::{Read, ReadExt};
use crate::io::{Write, WriteExt};
use async_t::async_trait;
use impl_trait_for_tuples::impl_for_tuples;

#[async_trait]
pub trait AsyncPull: Sized {
    async fn pull<R: Read + Unpin + Send>(io: &'future mut R) -> crate::Result<Self>
    where
        R: 'static;
}

#[async_trait]
pub trait AsyncSend: Sized {
    async fn send<W: Write + Unpin + Send + 'static>(
        &'future self,
        io: &'future mut W,
    ) -> crate::Result<()>;
}

macro_rules! impl_async_pull_int {
    ($($t: ty),*) => {
        $(
            #[async_trait]
            impl AsyncPull for $t {
                async fn pull<R: Read + Unpin + Send>(io: &'future mut R) -> crate::Result<Self> where R: 'static {
                    let mut bytes = [0u8; std::mem::size_of::<Self>()];
                    io.read_exact(&mut bytes).await?;
                    Ok(Self::from_be_bytes(bytes))
                }
            }
            #[async_trait]
            impl AsyncSend for $t {
                async fn send<W: Write + Unpin + Send + 'static>(&'future self, io: &'future mut W) -> crate::Result<()> {
                    let bytes = Self::to_be_bytes(*self);
                    io.write_all(&bytes).await?;
                    Ok(())
                }
            }
        )*
    };

}

impl_async_pull_int! {
    i8, i16, i32, i64, i128,
    u8, u16, u32, u64, u128
}

#[async_trait]
impl AsyncPull for bool {
    async fn pull<R: Read + Unpin + Send + 'static>(io: &'future mut R) -> crate::Result<Self> {
        let mut bytes: [u8; 1] = [0u8; 1];
        io.read_exact(&mut bytes).await?;
        Ok(bytes[0] == 1)
    }
}

#[async_trait]
impl AsyncSend for bool {
    async fn send<W: Write + Unpin + Send + 'static>(
        &'future self,
        io: &'future mut W,
    ) -> crate::Result<()> {
        let bytes = [*self as u8];
        io.write_all(&bytes).await?;
        Ok(())
    }
}

#[async_trait]
impl<T: Send + AsyncPull + 'static> AsyncPull for Vec<T> {
    async fn pull<R: Read + Unpin + Send + 'static>(io: &'future mut R) -> crate::Result<Self> {
        let len = u64::pull(io).await?;
        let mut v = vec![];
        for _ in 0..len {
            let val = T::pull(io).await?;
            v.push(val)
        }
        Ok(v)
    }
}

#[async_trait]
impl<T: Send + Sync + AsyncSend + 'static> AsyncSend for &[T] {
    async fn send<W: Write + Unpin + Send + 'static>(
        &'future self,
        io: &'future mut W,
    ) -> crate::Result<()> {
        let len = self.len() as u64;
        len.send(io).await?;
        for val in self.iter() {
            val.send(io).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl<T: Send + Sync + AsyncSend + 'static, const N: usize> AsyncSend for [T; N] {
    async fn send<W: Write + Unpin + Send + 'static>(
        &'future self,
        io: &'future mut W,
    ) -> crate::Result<()> {
        for val in self {
            val.send(io).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl<T: Send + AsyncPull + 'static + Default + Copy, const N: usize> AsyncPull for [T; N] {
    async fn pull<R: Read + Unpin + Send + 'static>(io: &'future mut R) -> crate::Result<Self> {
        let mut v = [T::default(); N];
        for ptr in v.iter_mut() {
            let val = T::pull(io).await?;
            *ptr = val;
        }
        Ok(v)
    }
}

#[async_trait]
impl AsyncPull for String {
    async fn pull<R: Read + Unpin + Send>(io: &'future mut R) -> crate::Result<Self>
    where
        R: 'static,
    {
        let vec = Vec::pull(io).await?;
        String::from_utf8(vec).map_err(|e| err!(e))
    }
}

#[async_trait]
impl AsyncSend for String {
    async fn send<W: Write + Unpin + Send + 'static>(
        &'future self,
        io: &'future mut W,
    ) -> crate::Result<()> {
        self.as_str().send(io).await?;
        Ok(())
    }
}

#[async_trait]
impl AsyncSend for &str {
    async fn send<W: Write + Unpin + Send + 'static>(
        &'future self,
        io: &'future mut W,
    ) -> crate::Result<()> {
        self.as_bytes().send(io).await?;
        Ok(())
    }
}

#[allow(unused_macros)]
macro_rules! for_tuples {
    ($($t: tt)*) => {}; // make rust-analyzer calm down
}

#[impl_for_tuples(2, 16)]
#[async_trait]
impl AsyncSend for TupleIdentifier {
    for_tuples!( where #( TupleIdentifier: Send + Sync + 'static )* );
    async fn send<W: Write + Unpin + Send + 'static>(
        &'future self,
        io: &'future mut W,
    ) -> crate::Result<()> {
        for_tuples!(
            #( TupleIdentifier.send(io).await?; )*
        );
        Ok(())
    }
}

#[impl_for_tuples(2, 16)]
#[async_trait]
impl AsyncPull for TupleIdentifier {
    for_tuples!( where #( TupleIdentifier: Send + 'static + AsyncPull )* );

    async fn pull<R: Read + Unpin + Send>(io: &'future mut R) -> crate::Result<Self>
    where
        R: 'static,
    {
        let tpl = for_tuples!(
           ( #( TupleIdentifier::pull(io).await? ),* )
        );
        Ok(tpl)
    }
}


// test


struct A {
    d: String,
    f: u32,
    q: ([u8; 5], u32, u32)
}

#[async_trait]
impl AsyncSend for A {
    async fn send<W: Write + Unpin + Send + 'static>(&'future self, io: &'future mut W) -> crate::Result<()> {
        self.d.send(io).await?;
        self.f.send(io).await?;
        self.q.send(io).await?;
        Ok(())
    }
}

#[async_trait]
impl AsyncPull for A {
    async fn pull<R: Read + Unpin + Send>(io: &'future mut R) -> crate::Result<Self>
    where
        R: 'static
    {
        Ok(A {
            d: <String as AsyncPull>::pull(io).await?,
            f: <u32 as AsyncPull>::pull(io).await?,
            q: <([u8; 5], u32, u32) as AsyncPull>::pull(io).await?
        })
    }
}
