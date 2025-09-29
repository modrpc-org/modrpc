use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

use crate::{
    ByteStreamInitState,
    ByteStreamReceiverConfig,
    ByteStreamReceiverHooks,
    ByteStreamReceiverStubs,
};

struct State {
    // Blobs by their start index
    blobs: RefCell<HashMap<u64, modrpc::Packet>>,

    current_blob_start: Cell<u64>,
    consume_cursor: Cell<u64>,

    // A bit clunky, but rather than precisely track and wake waiting tasks by the byte range they
    // are waiting for, just wake every waiting task whenever new bytes come in. We don't expect
    // there to be a lot of concurrent waiters, so doing this seems cheaper than managing another
    // datastructure.
    waiters: localq::WaiterQueue,
}

#[derive(Clone)]
pub struct ByteStreamReceiver {
    state: Rc<State>,
}

pub struct ByteStreamReceiverBuilder {
    stubs: ByteStreamReceiverStubs,
    state: Rc<State>,
}

impl ByteStreamReceiverBuilder {
    pub fn new(
        _name: &'static str,
        _hooks: ByteStreamReceiverHooks,
        stubs: ByteStreamReceiverStubs,
        _config: &ByteStreamReceiverConfig,
        _init: ByteStreamInitState,
    ) -> Self {
        let state = Rc::new(State {
            blobs: RefCell::new(HashMap::new()),
            current_blob_start: Cell::new(0),
            consume_cursor: Cell::new(0),
            waiters: localq::WaiterQueue::new(),
        });

        Self { stubs, state }
    }

    pub fn create_handle(
        &self,
        _setup: &modrpc::RoleSetup,
    ) -> ByteStreamReceiver {
        ByteStreamReceiver {
            state: self.state.clone(),
        }
    }

    pub fn build(
        self,
        setup: &modrpc::RoleSetup,
    ) {
        let state = self.state.clone();
        self.stubs.blob
            .inline_untyped(setup, move |_source, packet| {
                use mproto::BaseLen;

                if packet.len() < 8 {
                    // Invalid packet
                    return;
                }

                // Skip the packet header
                packet.advance(modrpc::TransmitPacket::BASE_LEN);

                // Read start index
                let start_index_bytes: [u8; 8] = packet[..8].try_into().unwrap();
                let start_index = u64::from_le_bytes(start_index_bytes);
                // Remove start index header
                packet.advance(8);

                if start_index < state.current_blob_start.get() {
                    state.current_blob_start.set(start_index);
                }

                let mut blobs = state.blobs.borrow_mut();
                blobs.entry(start_index).or_insert(packet.clone());

                state.waiters.notify(usize::MAX);
            })
            .subscribe();
    }
}

impl ByteStreamReceiver {
    pub fn cursor(&self) -> u64 {
        self.state.consume_cursor.get()
    }

    pub fn peek(&self) -> Option<modrpc::Packet> {
        let start = self.state.current_blob_start.get();
        let cursor = self.state.consume_cursor.get();
        let blobs = self.state.blobs.borrow();

        if start > cursor {
            // Blobs arrived out of order and we don't have the next blob to peek yet.
            return None;
        }

        let blob = blobs.get(&start)?.clone();
        blob.advance((cursor - start) as usize);
        Some(blob)
    }

    pub fn consume(&self, count: u64) -> Option<modrpc::Packet> {
        use std::collections::hash_map::Entry;

        let start = self.state.current_blob_start.get();
        let cursor = self.state.consume_cursor.get();
        let offset_in_blob = cursor - start;
        let mut blobs = self.state.blobs.borrow_mut();

        if start > cursor {
            // Blobs arrived out of order and we don't have the next blob to peek yet.
            return None;
        }

        let Entry::Occupied(blob_entry) = blobs.entry(start) else {
            return None;
        };

        let blob =
            if count >= blob_entry.get().len() as u64 - offset_in_blob {
                // Finish consuming the current blob
                let blob = blob_entry.remove();
                self.state.current_blob_start.set(start + blob.len() as u64);
                blob
            } else {
                blob_entry.get().clone()
            };
        blob.advance(offset_in_blob as usize);
        blob.set_len(std::cmp::min(blob.len(), count as usize));

        self.state.consume_cursor.set(cursor + blob.len() as u64);

        Some(blob)
    }

    pub fn try_peek_ahead(&self, read_start: u64, read_len: u64) -> Option<modrpc::Packet> {
        let start = self.state.current_blob_start.get();
        let consume_cursor = self.state.consume_cursor.get();

        // Another clunk - we allow bytes to be peeked out-of-order, but wait for all bytes up
        // through the end of the read to be present. We could lift this restriction by storing
        // received blobs in a BinaryHeap instead of a HashMap.

        if consume_cursor > read_start {
            // The bytes to read have already been consumed.
            return None;
        }
        if start > read_start {
            // Blobs arrived out of order and we don't have the next blob to peek yet.
            return None;
        }

        let mut cursor = start;
        let blobs = self.state.blobs.borrow();
        loop {
            let Some(blob) = blobs.get(&cursor) else {
                return None;
            };

            if cursor + blob.len() as u64 > read_start
                // Special handling for empty reads
                || cursor + blob.len() as u64 == read_start && read_len == 0
            {
                // Found the blob to read
                let blob = blob.clone();
                blob.advance((read_start - cursor) as usize);
                blob.set_len(std::cmp::min(blob.len(), read_len as usize));
                return Some(blob);
            }

            cursor += blob.len() as u64;
        }
    }

    pub async fn peek_ahead(&self, read_start: u64, read_len: u64) -> modrpc::Packet {
        self.state.waiters.wait_for(|| self.try_peek_ahead(read_start, read_len)).await
    }
}
