//! Type juggling. Do not enter unless you want a headache, or if you want to understand how this works.
//! I won't bother documenting this.

use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    serialization::formats::{Bincode, ReadFormat, SendFormat},
    Channel,
};

/// used for internals.
/// pipe!(tx i32, rx u32) -> TypeIter<Tx<i32>, TypeIter<Rx<u32>>>
#[macro_export]
macro_rules! pipe {
    (tx $t: ty) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Tx<$t>>
    };
    (rx $t: ty) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Rx<$t>>
    };
    (tx $t: ty, $($lit: ident $s: ty),*) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Tx<$t>, $crate::pipe!($($lit $s),*)>
    };
    (rx $t: ty, $($lit: ident $s: ty),*) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Rx<$t>, $crate::pipe!($($lit $s),*)>
    };
}

/// shorten tx calls
/// ```no_run
/// tx!(pipe, 2); // this is equivalent to this
/// let pipe = pipe.tx(2).await?;
/// ```
#[macro_export]
macro_rules! tx {
    ($i: ident, $e: expr) => {
        let $i = $i.tx($e).await?;
    };
}

/// shorten rx calls
/// ```no_run
/// rx!(res, pipe); // this is equivalent to this
/// let (res, pipe) = pipe.rx().await?;
/// ```
#[macro_export]
macro_rules! rx {
    ($i: ident, $e: ident) => {
        let ($i, $e) = $e.rx().await?;
        #[allow(unused_variables)] // disable unused warning
        let $e = $e;
    };
}

///
/// Declares pipelines.
/// Pipelines are used to guarantee that communication is correct at compile-time.
/// ```no_run
/// pipeline! {
///     pub MyPipeline {
///         tx String,
///         rx String,
///     }
/// }
/// ```
#[macro_export]
macro_rules! pipeline {
    () => {};
    (
        $v: vis $i: ident {
            $($lit: ident $s: ty),*
            $(,)?
        }
    ) => {
        $v struct $i;
        impl $crate::type_iter::Pipeline for $i {
            type Pipe = $crate::pipe!($($lit $s),*);
        }
    };
}

pub trait TypeIterT {
    type Next;
    type Type;
}

impl TypeIterT for () {
    type Next = ();
    type Type = ();
}

#[derive(Default)]
pub struct TypeIter<T, L = ()>(PhantomData<T>, PhantomData<L>);
impl<T, L: TypeIterT> TypeIterT for TypeIter<T, L> {
    type Next = L;
    type Type = T;
}

pub trait Transmit {
    type Type;
}
pub trait Receive {
    type Type;
}

impl<T> Transmit for Tx<T> {
    type Type = T;
}
impl<T> Receive for Rx<T> {
    type Type = T;
}

pub struct Tx<T>(T);
pub struct Rx<T>(T);

pub trait Pipeline {
    type Pipe: TypeIterT;
}

impl Pipeline for () {
    type Pipe = ();
}

pub trait Str {}
impl Str for Tx<String> {}
impl Str for Tx<&str> {}

const _: () = {
    assert!(
        std::mem::size_of::<Channel>() == std::mem::size_of::<MainChannel<crate::pipe!(tx())>>()
    );
    assert!(
        std::mem::size_of::<Channel>() == std::mem::size_of::<PeerChannel<crate::pipe!(tx())>>()
    );
};

#[repr(transparent)]
/// Used for writing services, peer services should use PeerChannel.
pub struct MainChannel<T: TypeIterT, ReadFmt: ReadFormat = Bincode, SendFmt: SendFormat = Bincode>(
    pub(crate) PhantomData<T>,
    pub(crate) Channel<ReadFmt, SendFmt>,
);

impl<T: TypeIterT, ReadFmt: ReadFormat, SendFmt: SendFormat> MainChannel<T, ReadFmt, SendFmt> {
    pub fn new<P: Pipeline>(
        chan: Channel<ReadFmt, SendFmt>,
    ) -> MainChannel<P::Pipe, ReadFmt, SendFmt> {
        MainChannel(PhantomData, chan)
    }
    pub async fn tx(
        mut self,
        obj: <T::Type as Transmit>::Type,
    ) -> crate::Result<MainChannel<T::Next, ReadFmt, SendFmt>>
    where
        T::Type: Transmit,
        <T as TypeIterT>::Next: TypeIterT,
        <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send + 'static,
    {
        self.1.tx(obj).await?;
        Ok(MainChannel(PhantomData, self.1))
    }
    pub async fn rx(
        mut self,
    ) -> crate::Result<(
        <T::Type as Receive>::Type,
        MainChannel<T::Next, ReadFmt, SendFmt>,
    )>
    where
        T::Type: Receive,
        <T as TypeIterT>::Next: TypeIterT,
        <T::Type as Receive>::Type: DeserializeOwned + 'static,
    {
        let res = self.1.rx::<<T::Type as Receive>::Type>().await?;
        let chan = MainChannel(PhantomData, self.1);
        Ok((res, chan))
    }
    pub fn coerce(self) -> Channel<ReadFmt, SendFmt> {
        self.1
    }
    // pub async fn tx_str(mut self, obj: &str) -> crate::Result<MainChannel<T::Next, ReadFmt, SendFmt>>
    // where
    //     T::Type: Transmit + Str,
    //     <T as TypeIterT>::Next: TypeIterT,
    //     <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send + 'static,
    // {
    //     self.1.tx_str(obj).await?;
    //     Ok(MainChannel(PhantomData, self.1))
    // }
}

impl<T: TypeIterT, ReadFmt: ReadFormat, SendFmt: SendFormat> From<MainChannel<T, ReadFmt, SendFmt>>
    for Channel<ReadFmt, SendFmt>
{
    fn from(s: MainChannel<T, ReadFmt, SendFmt>) -> Self {
        s.coerce()
    }
}

impl<T: TypeIterT> From<PeerChannel<T>> for Channel {
    fn from(s: PeerChannel<T>) -> Self {
        s.coerce()
    }
}

#[repr(transparent)]
/// Used for consuming services. Services should use MainChannel.
pub struct PeerChannel<T: TypeIterT, ReadFmt: ReadFormat = Bincode, SendFmt: SendFormat = Bincode>(
    pub(crate) PhantomData<T>,
    pub(crate) Channel<ReadFmt, SendFmt>,
);

impl<T: TypeIterT, ReadFmt: ReadFormat, SendFmt: SendFormat> PeerChannel<T, ReadFmt, SendFmt> {
    pub fn new<P: Pipeline>(
        chan: Channel<ReadFmt, SendFmt>,
    ) -> PeerChannel<P::Pipe, ReadFmt, SendFmt>
    where
        <P as Pipeline>::Pipe: TypeIterT,
    {
        PeerChannel(PhantomData, chan)
    }
    pub async fn tx(
        mut self,
        obj: <T::Type as Receive>::Type,
    ) -> crate::Result<PeerChannel<T::Next, ReadFmt, SendFmt>>
    where
        T::Type: Receive,
        <T as TypeIterT>::Next: TypeIterT,
        <<T as TypeIterT>::Type as Receive>::Type: Serialize + Send + 'static,
    {
        self.1.tx(obj).await?;
        Ok(PeerChannel(PhantomData, self.1))
    }

    pub async fn rx(
        mut self,
    ) -> crate::Result<(
        <T::Type as Transmit>::Type,
        PeerChannel<T::Next, ReadFmt, SendFmt>,
    )>
    where
        T::Type: Transmit,
        <T as TypeIterT>::Next: TypeIterT,
        <T::Type as Transmit>::Type: DeserializeOwned + 'static,
    {
        let res = self.1.rx::<<T::Type as Transmit>::Type>().await?;
        let chan = PeerChannel(PhantomData, self.1);
        Ok((res, chan))
    }
    pub fn channel(self) -> Channel<ReadFmt, SendFmt> {
        self.1
    }
    pub fn coerce<R: ReadFormat, S: SendFormat>(self) -> Channel<R, S> {
        self.1.coerce()
    }
}

/// Experiments at the moment.
/// AutoTyped is supposed to allow for writing self-describing services,
/// so pipelines will be written automatically.
/// The exact implementation is unresolved at the moment.
/// ```no_run
/// #[service]
/// async fn my_service(chan: AutoTyped) -> Result<_> {
///     let chan = chan.send(false).await?;
///     let chan = chan.send("hello world!").await?;
///     chan
/// } // this will be the equivalent of writing this pipeline
///
/// pipeline! {
///     __internal { // __internal since no pipelines will actually be written
///         tx bool,
///         tx String,
///     }
/// }
/// ```
// unimplemented
#[allow(unused)]
pub struct AutoTyped<T: TypeIterT>(Channel, PhantomData<T>);

impl<T: TypeIterT> AutoTyped<T> {
    #[allow(unused)]
    pub async fn tx<D: Serialize + Send + 'static>(
        mut self,
        s: D,
    ) -> crate::Result<AutoTyped<TypeIter<Tx<D>, T>>> {
        self.0.tx(s).await?;
        Ok(AutoTyped(self.0, PhantomData))
    }
    #[allow(unused)]
    pub async fn rx<D: DeserializeOwned + 'static>(
        mut self,
    ) -> crate::Result<(D, AutoTyped<TypeIter<Rx<D>, T>>)> {
        let p = self.0.rx().await?;
        Ok((p, AutoTyped(self.0, PhantomData)))
    }
}
