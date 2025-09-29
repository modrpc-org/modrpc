use mproto::BaseLen;
use tokio::io::AsyncReadExt;

use crate::{Packet, PacketBundle};

pub struct TcpIngress {
    stream: tokio::net::tcp::OwnedReadHalf,
    framer: bab::Framer,
    // Max packet size including bundle header.
    max_packet_size: usize,
    cursor: usize,
}

impl TcpIngress {
    pub fn new(
        stream: tokio::net::tcp::OwnedReadHalf,
        buffer_pool: bab::HeapBufferPool,
        // Max packet size including bundle header.
        max_packet_size: usize,
    ) -> Self {
        assert!(max_packet_size <= buffer_pool.buffer_size());

        Self {
            stream,
            framer: bab::Framer::new(buffer_pool),
            max_packet_size,
            cursor: 0,
        }
    }

    pub async fn receive(&mut self) -> std::io::Result<Packet> {
        loop {
            let write = self.framer.write().await;
            if self.cursor < PacketBundle::BASE_LEN {
                let max_read = std::cmp::max(
                    // Leave enough space for max packet size
                    write.len() - self.max_packet_size,
                    // But if we've already started reading a bundle header, finish reading the bundle
                    // header.
                    PacketBundle::BASE_LEN,
                );

                if max_read > self.cursor {
                    let n = self.stream.read(&mut write[self.cursor..max_read]).await?;
                    if n == 0 {
                        // Socket was shutdown
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::ConnectionReset,
                            "tcp transport shutdown",
                        ));
                    }
                    self.cursor += n;
                } else {
                    debug_assert!(self.cursor >= PacketBundle::BASE_LEN);
                }

                if self.cursor < PacketBundle::BASE_LEN {
                    continue;
                }
            }

            let packet_header: PacketBundle =
                mproto::decode_value(&write).expect("decode rx bundle header");
            let packet_len = packet_header.length as usize;

            while packet_len > self.cursor - PacketBundle::BASE_LEN {
                // Read till whichever is higher - the max safe read length or the end of the current
                // packet.
                let max_read = std::cmp::max(
                    // Try to leave enough space for max packet size
                    write.len() - self.max_packet_size,
                    PacketBundle::BASE_LEN + packet_len,
                );
                let n = self.stream.read(&mut write[self.cursor..max_read]).await?;
                if n == 0 {
                    // Socket was shutdown
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionReset,
                        "tcp transport shutdown",
                    ));
                }
                self.cursor += n;
            }

            self.framer.commit(PacketBundle::BASE_LEN + packet_len);
            self.cursor -= PacketBundle::BASE_LEN + packet_len;

            let finished_packet = if self.framer.remaining_on_buffer() < self.max_packet_size {
                debug_assert_eq!(self.cursor, 0);
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
