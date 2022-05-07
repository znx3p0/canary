use crate::serialization::formats::Format;

use super::encrypted::{
    bidirectional::{BidirectionalChannel, RefBidirectionalChannel},
    receive_channel, send_channel,
};

pub type Channel<F = Format> = BidirectionalChannel<F>;
pub type RefChannel<'a, F = Format> = RefBidirectionalChannel<'a, F>;

pub type SendChannel<F = Format> = send_channel::SendChannel<F>;
pub type RefSendChannel<'a, F = Format> = send_channel::RefSendChannel<'a, F>;

pub type ReceiveChannel<F = Format> = receive_channel::ReceiveChannel<F>;
pub type RefReceiveChannel<'a, F = Format> = receive_channel::RefReceiveChannel<'a, F>;
