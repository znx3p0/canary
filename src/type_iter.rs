//! Here be dragons

use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};

use crate::Channel;

/// used for internals.
/// `pipe!(send i32, receive u32)` -> `TypeIter<Tx<i32>, TypeIter<Rx<u32>>>`
#[macro_export]
macro_rules! pipe {
    (send $t: ty) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Tx<$t>>
    };
    (receive $t: ty) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Rx<$t>>
    };
    (send $t: ty) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Tx<$t>>
    };
    (receive $t: ty) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Rx<$t>>
    };

    (send $t: ty, $($lit: ident $s: ty),*) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Tx<$t>, $crate::pipe!($($lit $s),*)>
    };
    (receive $t: ty, $($lit: ident $s: ty),*) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Rx<$t>, $crate::pipe!($($lit $s),*)>
    };
    (send $t: ty, $($lit: ident $s: ty),*) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Tx<$t>, $crate::pipe!($($lit $s),*)>
    };
    (receive $t: ty, $($lit: ident $s: ty),*) => {
        $crate::type_iter::TypeIter<$crate::type_iter::Rx<$t>, $crate::pipe!($($lit $s),*)>
    };
}

/// shorten send calls
/// ```no_run
/// send!(pipe, 2); // this is equivalent to this
/// let pipe = pipe.send(2).await?;
/// ```
#[macro_export]
macro_rules! send {
    ($i: ident, $e: expr) => {
        #[allow(unused_variables)] // disable unused warning
        let $i = $i.send($e).await?;
    };
}

/// shorten receive calls
/// ```no_run
/// receive!(res, pipe); // this is equivalent to this
/// let (res, pipe) = pipe.receive().await?;
/// ```
#[macro_export]
macro_rules! receive {
    ($i: ident, $e: ident) => {
        let ($i, $e) = $e.receive().await?;
        #[allow(unused_variables)] // disable unused warning
        let $e = $e;
    };
}

///
/// Declares pipelines.
/// Pipelines are used to guarantee that communication is correct at compile-time.
/// ```no_run
/// pipeline! {
///     pub pipeline MyPipeline {
///         send String,
///         receive String,
///     }
/// }
/// ```
#[macro_export]
macro_rules! pipeline {
    () => {};
    (
        $v: vis pipeline $i: ident {
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

/// used for iterating over types
pub trait TypeIterT {
    /// next type iterator
    type Next;
    /// current value of node
    type Type;
}

impl TypeIterT for () {
    type Next = ();
    type Type = ();
}

#[derive(Default)]
/// type iterator which allows compile-time magic
pub struct TypeIter<T, L = ()>(PhantomData<T>, PhantomData<L>);
impl<T, L: TypeIterT> TypeIterT for TypeIter<T, L> {
    type Next = L;
    type Type = T;
}

/// trait that represents send or send in pipelines
pub trait Transmit {
    /// type that can be transmitted
    type Type;
}
/// trait that represents receive or receive in pipelines
pub trait Receive {
    /// type that can be received
    type Type;
}

impl<T> Transmit for Tx<T> {
    type Type = T;
}
impl<T> Receive for Rx<T> {
    type Type = T;
}

/// type iterator that represents a type to be sent
pub struct Tx<T>(T);
/// type iterator that represents a type to be received
pub struct Rx<T>(T);

/// used for constructing pipelines
pub trait Pipeline {
    /// inner pipeline
    type Pipe: TypeIterT;
}

impl Pipeline for () {
    type Pipe = ();
}

/// optimization to allow &str to be sent whenever a String needs to be received
pub trait Str {}
impl Str for Tx<String> {}
impl Str for Tx<&str> {}

/// optimization to allow &str to be sent whenever a Vec needs to be received
pub trait Slice<T> {}
impl<T> Slice<T> for Tx<&[T]> {}
impl<T> Slice<T> for Tx<Vec<T>> {}

/// Used for writing services, peer services should use PeerChannel.
pub struct MainChannel<T: TypeIterT>(pub(crate) PhantomData<T>, pub(crate) Channel);

impl<T: TypeIterT> MainChannel<T> {
    /// construct a new main channel
    pub fn new<P: Pipeline>(chan: Channel) -> MainChannel<P::Pipe> {
        MainChannel(PhantomData, chan)
    }
    /// send an object through the stream and iterate to the next type
    pub async fn send(
        mut self,
        obj: <T::Type as Transmit>::Type,
    ) -> crate::Result<MainChannel<T::Next>>
    where
        T::Type: Transmit,
        <T as TypeIterT>::Next: TypeIterT,
        <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send,
    {
        self.1.send(obj).await?;
        Ok(MainChannel(PhantomData, self.1))
    }
    /// receive an object from the stream and iterate to the next type
    pub async fn receive(
        mut self,
    ) -> crate::Result<(<T::Type as Receive>::Type, MainChannel<T::Next>)>
    where
        T::Type: Receive,
        <T as TypeIterT>::Next: TypeIterT,
        <T::Type as Receive>::Type: DeserializeOwned,
    {
        let res = self.1.receive::<<T::Type as Receive>::Type>().await?;
        let chan = MainChannel(PhantomData, self.1);
        Ok((res, chan))
    }
    /// coerce into a different kind of channel:
    pub fn coerce(self) -> Channel {
        self.1
    }
    /// send a str through the stream, this is an optimization done for pipelines receiving String
    /// to make sure an unnecessary allocation is not made
    pub async fn send_str(mut self, obj: &str) -> crate::Result<MainChannel<T::Next>>
    where
        T::Type: Transmit + Str,
        <T as TypeIterT>::Next: TypeIterT,
        <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send,
    {
        self.1.send(obj).await?;
        Ok(MainChannel(PhantomData, self.1))
    }

    /// send a slice through the stream, this is an optimization done for pipelines receiving Vec<T>
    /// to make sure an unnecessary allocation is not made
    pub async fn send_slice(mut self, obj: &[T::Type]) -> crate::Result<MainChannel<T::Next>>
    where
        T::Type: Transmit + Slice<T::Type> + Serialize,
        <T as TypeIterT>::Next: TypeIterT,
        <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send,
    {
        self.1.send(obj).await?;
        Ok(MainChannel(PhantomData, self.1))
    }
}

/// Used for consuming services. Services should use MainChannel.
pub struct PeerChannel<T: TypeIterT>(pub(crate) PhantomData<T>, pub(crate) Channel);

impl<T: TypeIterT> PeerChannel<T> {
    /// construct a new peer channel
    pub fn new<P: Pipeline>(chan: Channel) -> PeerChannel<P::Pipe>
    where
        <P as Pipeline>::Pipe: TypeIterT,
    {
        PeerChannel(PhantomData, chan)
    }
    /// send an object through the stream and iterate to the next type
    pub async fn send(
        mut self,
        obj: <T::Type as Receive>::Type,
    ) -> crate::Result<PeerChannel<T::Next>>
    where
        T::Type: Receive,
        <T as TypeIterT>::Next: TypeIterT,
        <<T as TypeIterT>::Type as Receive>::Type: Serialize + Send,
    {
        self.1.send(obj).await?;
        Ok(PeerChannel(PhantomData, self.1))
    }

    /// receive an object from the stream and iterate to the next type
    pub async fn receive(
        mut self,
    ) -> crate::Result<(<T::Type as Transmit>::Type, PeerChannel<T::Next>)>
    where
        T::Type: Transmit,
        <T as TypeIterT>::Next: TypeIterT,
        <T::Type as Transmit>::Type: DeserializeOwned + 'static,
    {
        let res = self.1.receive::<<T::Type as Transmit>::Type>().await?;
        let chan = PeerChannel(PhantomData, self.1);
        Ok((res, chan))
    }
    /// coerce into a different kind of channel:
    pub fn channel(self) -> Channel {
        self.1
    }
    /// send a str through the stream, this is an optimization done for pipelines receiving String
    /// to make sure an unnecessary allocation is not made
    pub async fn send_str(mut self, obj: &str) -> crate::Result<PeerChannel<T::Next>>
    where
        T::Type: Transmit + Str,
        <T as TypeIterT>::Next: TypeIterT,
        <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send,
    {
        self.1.send(obj).await?;
        Ok(PeerChannel(PhantomData, self.1))
    }
    /// send a slice through the stream, this is an optimization done for pipelines receiving Vec<T>
    /// to make sure an unnecessary allocation is not made
    pub async fn send_slice(mut self, obj: &[T::Type]) -> crate::Result<PeerChannel<T::Next>>
    where
        T::Type: Transmit + Slice<T::Type> + Serialize,
        <T as TypeIterT>::Next: TypeIterT,
        <<T as TypeIterT>::Type as Transmit>::Type: Serialize + Send,
    {
        self.1.send(obj).await?;
        Ok(PeerChannel(PhantomData, self.1))
    }
}
