use std::{
    cell::Cell,
    rc::Rc,
};

use modrpc::RoleSetup;

use crate::{
    ByteStreamInitState,
    ByteStreamSenderConfig,
    ByteStreamSenderHooks,
    ByteStreamSenderStubs,
};

struct State {
    hooks: ByteStreamSenderHooks,
    send_cursor: Cell<u64>,
}

#[derive(Clone)]
pub struct ByteStreamSender {
    state: Rc<State>,
}

pub struct ByteStreamSenderBuilder {
    state: Rc<State>,
}

impl ByteStreamSenderBuilder {
    pub fn new(
        _name: &'static str,
        hooks: ByteStreamSenderHooks,
        _stubs: ByteStreamSenderStubs,
        _config: &ByteStreamSenderConfig,
        _init: ByteStreamInitState,
    ) -> Self {
        let state = Rc::new(State {
            hooks: hooks.clone(),
            send_cursor: Cell::new(0),
        });
        Self { state }
    }

    pub fn create_handle(
        &self,
        _setup: &RoleSetup,
    ) -> ByteStreamSender {
        ByteStreamSender {
            state: self.state.clone(),
        }
    }

    pub fn build(
        self,
        _setup: &RoleSetup,
    ) {
    }
}

impl ByteStreamSender {
    pub async fn send(&self, bytes: &[u8]) -> u64 {
        let start_index = self.state.send_cursor.get();
        self.state.send_cursor.set(start_index + bytes.len() as u64);

        self.state.hooks.blob.send_raw(8 + bytes.len(), |write_buf| {
            write_buf[..8].copy_from_slice(&start_index.to_le_bytes());
            write_buf[8..].copy_from_slice(bytes);
        })
        .await;

        start_index
    }

    /// SAFETY: You must have exclusive ownership of the buffer and there must be enough headroom
    /// for modrpc::TransmitPacket::BASE_LEN + 8 bytes
    pub async unsafe fn send_buffer(&self, buffer: modrpc::BufferPtr) -> u64 {
        let headroom = <modrpc::TransmitPacket as mproto::BaseLen>::BASE_LEN;
        let payload_len =
            modrpc::WriterFlushSender::get_complete_buffer_len(buffer) as usize - headroom - 8;

        let start_index = self.state.send_cursor.get();
        self.state.send_cursor.set(start_index + payload_len as u64);

        // Write the start index
        let headroom = <modrpc::TransmitPacket as mproto::BaseLen>::BASE_LEN;
        let start_index_buf = unsafe { buffer.slice_mut(headroom..headroom + 8) };
        start_index_buf.copy_from_slice(&start_index.to_le_bytes());

        unsafe { self.state.hooks.blob.send_buffer(buffer).await; }

        start_index
    }

    pub async fn wait_consumed(&self, _cursor: u64) {
        // TODO
    }
}
