use futures_util::{Stream, StreamExt};
use mproto::BaseLen;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::{Packet, endpoint_proto::PacketBundle};

pub struct WebSocketIngress<Ws> {
    ws: Ws,
    framer: bab::Framer,
    // Max packet size including bundle header.
    max_packet_size: usize,
}

impl<Ws> WebSocketIngress<Ws>
where
    Ws: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + std::marker::Unpin,
{
    pub fn new(
        ws: Ws,
        buffer_pool: bab::HeapBufferPool,
        // Max packet size including bundle header.
        max_packet_size: usize,
    ) -> Self {
        assert!(max_packet_size <= buffer_pool.buffer_size());

        Self {
            ws,
            framer: bab::Framer::new(buffer_pool),
            max_packet_size,
        }
    }

    pub async fn receive(&mut self) -> std::io::Result<Packet> {
        loop {
            let (_bundle_header, bundle_bytes) = match self.ws.next().await {
                Some(Ok(Message::Binary(payload))) => {
                    let Ok(bundle_header) = mproto::decode_value::<PacketBundle>(&payload[..])
                    else {
                        continue;
                    };

                    assert_eq!(
                        PacketBundle::BASE_LEN + bundle_header.length as usize,
                        payload.len()
                    );
                    assert!(payload.len() < self.max_packet_size);

                    (bundle_header, payload)
                }
                Some(Ok(_)) => {
                    println!("Received non-binary WebSocket message - ignoring.");
                    continue;
                }
                Some(Err(e)) => {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
                }
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionReset,
                        "WebSocket connection closed",
                    ));
                }
            };

            let write = self.framer.write().await;
            write[..bundle_bytes.len()].copy_from_slice(&bundle_bytes);
            self.framer.commit(bundle_bytes.len());

            let finished_packet = if self.framer.remaining_on_buffer() < self.max_packet_size {
                self.framer.next_buffer()
            } else {
                self.framer.finish_frame()
            };

            if let Some(packet) = finished_packet {
                return Ok(packet);
            }
        }
    }
}
