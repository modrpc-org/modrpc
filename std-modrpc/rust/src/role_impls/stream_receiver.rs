use crate::proto::{StreamInitState, StreamItemLazy, StreamReceiverConfig};
use std::collections::BinaryHeap;
use modrpc::RoleSetup;

#[derive(Clone)]
pub struct StreamReceiver<T> {
    _phantom: core::marker::PhantomData<T>,
}

pub struct StreamReceiverBuilder<T> {
    stubs: crate::StreamReceiverStubs<T>,
}

impl<T: mproto::Owned> StreamReceiverBuilder<T> {
    pub fn new(
        _name: &'static str,
        _hooks: crate::StreamReceiverHooks<T>,
        stubs: crate::StreamReceiverStubs<T>,
        _config: &StreamReceiverConfig,
        _init: StreamInitState,
    ) -> Self {
        Self { stubs }
    }

    pub fn create_handle(
        &self,
        _setup: &RoleSetup,
    ) -> crate::StreamReceiver<T> {
        crate::StreamReceiver {
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn build<H>(
        self,
        setup: &RoleSetup,
        mut handler: H,
    )
        where
            // TODO constrain handler payload type to be compatible with stream's payload type.
            for<'a> H: modrpc::AsyncHandler<Context<'a> = modrpc::EndpointAddr> + 'static,
            for<'a> H::Input<'a>: mproto::Lazy<'a>,
    {
        use std::cmp::Reverse;
        use mproto::BaseLen;

        #[allow(type_alias_bounds)]
        type HandlerInputOwned<'a, H: modrpc::AsyncHandler> =
            <H::Input<'a> as mproto::Lazy<'a>>::Owned;

        // Wrapper for StreamItem that is Eq + PartialEq + Ord + PartialOrd
        struct Item { seq: u64, packet: modrpc::Packet }
        impl Item { fn sort_key(&self) -> u64 { self.seq} }
        impl PartialEq for Item {
            fn eq(&self, other: &Self) -> bool { self.sort_key().eq(&other.sort_key()) }
        }
        impl Eq for Item { }
        impl PartialOrd for Item {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> { self.sort_key().partial_cmp(&other.sort_key()) }
        }
        impl Ord for Item {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.sort_key().cmp(&other.sort_key()) }
        }

        let mut heap = BinaryHeap::new();
        // TODO configurable capacity
        let (local_queue_tx, mut local_queue_rx) = localq::mpsc::channel::<modrpc::Packet>(64);

        setup.role_spawner().spawn(async move {
            loop {
                let Ok(packet) = local_queue_rx.recv().await else {
                    break;
                };

                // TODO decode error handling - at least log it?

                // Decode packet header
                let Ok(packet_header) =
                    mproto::decode_value::<modrpc::TransmitPacket>(packet.as_ref())
                else { continue; };

                let stream_item_offset = modrpc::TransmitPacket::BASE_LEN;
                let source = packet_header.source;

                // Decode payload
                let Ok(stream_item) =
                    mproto::decode_value::<StreamItemLazy<HandlerInputOwned<H>>>(
                        &packet.as_ref()[stream_item_offset..]
                    )
                else { continue; };

                let Ok(payload) = stream_item.payload() else { continue; };

                // Handle event
                handler.call(source.clone(), payload).await;
            }
        });

        self.stubs.item
            .inline_untyped(setup, move |_source, packet| {
                let seq = {
                    let Ok(stream_item) =
                        mproto::decode_value::<StreamItemLazy<HandlerInputOwned<H>>>(&packet)
                    else {
                        return;
                    };

                    let Ok(seq) = stream_item.seq() else {
                        return;
                    };

                    seq
                };

                // Reverse order so that heap produces item with smallest seq.
                heap.push(Reverse(Item { seq, packet: packet.clone() }));

                let mut next_seq = 0;
                while let Some(Reverse(stream_item)) = heap.peek() {
                    if stream_item.seq != next_seq { break; }
                    next_seq += 1;

                    // Unwrap guaranteed to succeed.
                    let Reverse(stream_item) = heap.pop().unwrap();

                    let _ = local_queue_tx.try_send(stream_item.packet);
                }
            })
            // TODO allow caller to specify load-balance vs subscribe?
            .subscribe();
    }
}
