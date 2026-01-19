use core::cell::{Cell, RefCell};
use core::cmp::Reverse;
use std::collections::BinaryHeap;
use std::rc::Rc;

pub struct ReceiveStream {
    local_queue_rx: localq::mpsc::Receiver<modrpc::Packet>,
    stream_state: Rc<StreamState>,
}

impl ReceiveStream {
    pub fn new(next_seq: Option<u64>) -> Self {
        // TODO use a waker cell instead of a channel
        let (local_queue_tx, local_queue_rx) = localq::mpsc::channel(1);
        let stream_state = Rc::new(StreamState::new(local_queue_tx, next_seq));

        Self {
            local_queue_rx,
            stream_state,
        }
    }
}

impl ReceiveStream {
    pub fn stream_state(&self) -> &Rc<StreamState> {
        &self.stream_state
    }

    pub fn try_next_packet(&mut self) -> Option<modrpc::Packet> {
        if let Ok(packet) = self.local_queue_rx.try_recv() {
            return Some(packet);
        }

        self.stream_state.try_pop()
    }

    pub async fn next_packet(&mut self) -> modrpc::Packet {
        if let Ok(packet) = self.local_queue_rx.try_recv() {
            return packet;
        }

        if let Some(packet) = self.stream_state.try_pop() {
            return packet;
        }

        self.local_queue_rx.recv().await.unwrap()
    }
}

// Wrapper for stream packets that is Eq + PartialEq + Ord + PartialOrd

struct OrderedItem {
    seq: u64,
    shutdown: bool,
    packet: modrpc::Packet,
}

impl PartialEq for OrderedItem {
    fn eq(&self, other: &Self) -> bool { self.seq.eq(&other.seq) }
}

impl Eq for OrderedItem { }

impl PartialOrd for OrderedItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.seq.partial_cmp(&other.seq)
    }
}

impl Ord for OrderedItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.seq.cmp(&other.seq) }
}

// Shared state of a single stream receiver

pub struct StreamState {
    heap: RefCell<BinaryHeap<Reverse<OrderedItem>>>,
    first_seq: Cell<u64>,
    last_seq: Cell<Option<u64>>,
    received_count: Cell<u64>,
    next_seq: Cell<Option<u64>>,
    local_queue_tx: localq::mpsc::Sender<modrpc::Packet>,
}

impl StreamState {
    fn new(local_queue_tx: localq::mpsc::Sender<modrpc::Packet>, next_seq: Option<u64>) -> Self {
        Self {
            heap: RefCell::new(BinaryHeap::new()),
            first_seq: Cell::new(0),
            last_seq: Cell::new(None),
            received_count: Cell::new(0),
            next_seq: Cell::new(next_seq),
            local_queue_tx,
        }
    }

    fn try_pop(&self) -> Option<modrpc::Packet> {
        let mut heap = self.heap.borrow_mut();
        let Reverse(stream_item) = heap.peek()?;

        let next_seq = self.next_seq.get().unwrap_or_else(|| {
            self.first_seq.set(stream_item.seq);
            stream_item.seq
        });

        if stream_item.seq != next_seq {
            return None;
        }
        self.next_seq.set(Some(next_seq + 1));

        Some(heap.pop().unwrap().0.packet)
    }

    /// Returns true if the stream is finished and should be cleaned up.
    pub fn handle_item(&self, seq: u64, shutdown: bool, packet: modrpc::Packet) -> bool {
        let mut heap = self.heap.borrow_mut();

        // If we don't know the next seq, treat the first item we get as the start of the stream.
        let next_seq = self.next_seq.get().unwrap_or_else(|| {
            self.first_seq.set(seq);
            seq
        });
        // If we subsequently receive earlier items, we drop them.
        if seq < next_seq {
            return false;
        }

        // Reverse order so that heap produces item with smallest seq.
        heap.push(Reverse(OrderedItem { seq, shutdown, packet }));
        self.received_count.set(self.received_count.get() + 1);
        if shutdown {
            self.last_seq.set(Some(seq));
        }

        while let Some(Reverse(stream_item)) = heap.peek() {
            if stream_item.seq != next_seq { break; }

            // Unwrap guaranteed to succeed.
            let Reverse(stream_item) = heap.pop().unwrap();

            if let Err(localq::mpsc::TrySendError::Full(packet)) =
                self.local_queue_tx.try_send(stream_item.packet)
            {
                heap.push(Reverse(OrderedItem {
                    seq: stream_item.seq,
                    shutdown: stream_item.shutdown,
                    packet,
                }));
                break;
            }

            self.next_seq.set(Some(next_seq + 1));
        }

        if let Some(last_seq) = self.last_seq.get() {
            (last_seq - self.first_seq.get() + 1) == self.received_count.get()
        } else {
            false
        }
    }
}

