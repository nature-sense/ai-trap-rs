use async_broadcast::{ Receiver as BroadcastReceiver, Sender as BroadcastSender };
use async_channel::{Receiver as ChannelReceiver, Sender as ChannelSender};

#[derive(Debug, Clone)]
pub struct BroadcastStream<T> {
    size : usize,
    stream_sender : BroadcastSender<T>,
    stream_receiver : BroadcastReceiver<T>
}

impl<T> BroadcastStream<T> {
    pub(crate) fn new(size : usize) -> Self {
        let (stream_sender, stream_receiver) = async_broadcast::broadcast::<T>(size);
        Self {size, stream_sender, stream_receiver }
    }
    
    pub fn broadcast_sender(&self) -> BroadcastSender<T> {
        self.stream_sender.clone()
    }

    pub fn broadcast_receiver(&self) -> BroadcastReceiver<T> {
        self.stream_receiver.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ChannelStream<T> {
    size : usize,
    stream_sender : ChannelSender<T>,
    stream_receiver : ChannelReceiver<T>
}

impl<T> ChannelStream<T> {
    pub(crate) fn new(size : usize) -> Self {
        let (stream_sender, stream_receiver) = async_channel::bounded::<T>(size);
        Self { size, stream_sender, stream_receiver }
    }

    pub fn channel_sender(&self) -> ChannelSender<T> {
        self.stream_sender.clone()
    }

    pub fn channel_receiver(&self) -> ChannelReceiver<T> {
        self.stream_receiver.clone()
    }

}
