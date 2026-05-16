use std::{
    cell::Cell,
    rc::Rc,
};

use modrpc::RoleSetup;

use crate::{
    proto::MultiStreamId,
    MultiByteStreamInitState,
    MultiByteStreamSenderConfig,
    MultiByteStreamSenderHooks,
    MultiByteStreamSenderStubs,
};

/// The wire format for each `blob` event is (all integers little-endian):
///
///  MultiStreamId (u64 owner + u32 id)
///  start_index   (u64)
///  payload       (raw bytes)
///
/// The high bit of `start_index` is reserved as an end-of-stream flag. When set, the low 63 bits
/// give the total byte length of the stream and the payload must be empty.
pub const MULTI_BYTE_STREAM_HEADER_LEN: usize = 12 + 8;
pub(crate) const EOF_FLAG: u64 = 1 << 63;
pub(crate) const INDEX_MASK: u64 = !EOF_FLAG;

struct State {
    hooks: MultiByteStreamSenderHooks,
}

#[derive(Clone)]
pub struct MultiByteStreamSender {
    state: Rc<State>,
}

pub struct MultiByteStreamSenderBuilder {
    state: Rc<State>,
}

impl MultiByteStreamSenderBuilder {
    pub fn new(
        _name: &'static str,
        hooks: MultiByteStreamSenderHooks,
        _stubs: MultiByteStreamSenderStubs,
        _config: &MultiByteStreamSenderConfig,
        _init: MultiByteStreamInitState,
    ) -> Self {
        let state = Rc::new(State { hooks });
        Self { state }
    }

    pub fn create_handle(
        &self,
        _setup: &RoleSetup,
    ) -> MultiByteStreamSender {
        MultiByteStreamSender {
            state: self.state.clone(),
        }
    }

    pub fn build(
        self,
        _setup: &RoleSetup,
    ) {
    }
}

pub struct SendMultiByteStream {
    stream_id: MultiStreamId,
    send_cursor: Cell<u64>,
    blob_tx: modrpc::EventTx<()>,
}

impl MultiByteStreamSender {
    pub fn new_stream(&self, stream_id: MultiStreamId) -> SendMultiByteStream {
        SendMultiByteStream {
            stream_id,
            send_cursor: Cell::new(0),
            blob_tx: self.state.hooks.blob.clone(),
        }
    }
}

impl SendMultiByteStream {
    pub fn stream_id(&self) -> MultiStreamId {
        self.stream_id
    }

    pub fn cursor(&self) -> u64 {
        self.send_cursor.get()
    }

    pub async fn send(&self, bytes: &[u8]) -> u64 {
        let start_index = self.send_cursor.get();
        self.send_cursor.set(start_index + bytes.len() as u64);

        let payload_len = MULTI_BYTE_STREAM_HEADER_LEN + bytes.len();
        self.blob_tx.send_raw(payload_len, |write_buf| {
            write_buf[..8].copy_from_slice(&self.stream_id.owner.to_le_bytes());
            write_buf[8..12].copy_from_slice(&self.stream_id.id.to_le_bytes());
            write_buf[12..20].copy_from_slice(&start_index.to_le_bytes());
            write_buf[20..].copy_from_slice(bytes);
        })
        .await;

        start_index
    }

    /// SAFETY: You must have exclusive ownership of the buffer and there must be enough headroom
    /// for `modrpc::TransmitPacket::BASE_LEN + MULTI_BYTE_STREAM_HEADER_LEN` bytes.
    pub async unsafe fn send_buffer(&self, buffer: modrpc::BufferPtr) -> u64 {
        let packet_header = <modrpc::TransmitPacket as mproto::BaseLen>::BASE_LEN;
        let payload_len = modrpc::WriterFlushSender::get_complete_buffer_len(buffer) as usize
            - packet_header
            - MULTI_BYTE_STREAM_HEADER_LEN;

        let start_index = self.send_cursor.get();
        self.send_cursor.set(start_index + payload_len as u64);

        // Write the stream id + start index
        let header_buf = unsafe {
            buffer.slice_mut(packet_header..packet_header + MULTI_BYTE_STREAM_HEADER_LEN)
        };
        header_buf[..8].copy_from_slice(&self.stream_id.owner.to_le_bytes());
        header_buf[8..12].copy_from_slice(&self.stream_id.id.to_le_bytes());
        header_buf[12..20].copy_from_slice(&start_index.to_le_bytes());

        unsafe { self.blob_tx.send_buffer(buffer).await; }

        start_index
    }

    /// Signal the end of the stream to the receiver.
    pub async fn end(self) {
        let final_len = self.send_cursor.get();
        assert!(final_len & EOF_FLAG == 0, "MultiByteStream length overflowed 63 bits");
        let encoded = final_len | EOF_FLAG;
        self.blob_tx.send_raw(MULTI_BYTE_STREAM_HEADER_LEN, |write_buf| {
            write_buf[..8].copy_from_slice(&self.stream_id.owner.to_le_bytes());
            write_buf[8..12].copy_from_slice(&self.stream_id.id.to_le_bytes());
            write_buf[12..20].copy_from_slice(&encoded.to_le_bytes());
        })
        .await;
    }

    pub async fn wait_consumed(&self, _cursor: u64) {
        // TODO
    }
}
