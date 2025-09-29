use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Poll, Waker, Context};

use mproto;
use modrpc::{Packet, TransmitPacket};

use crate::proto::{RequestLazy, ResponseLazy};

pub type RequestSubscriptionCallback = Box<dyn FnMut(Packet, PendingRequestSubscription)>;

#[derive(Clone)]
pub struct RequestTracker {
    inner: Rc<RequestTrackerInner>,
}

impl RequestTracker {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RequestTrackerInner {
                pending_requests: RefCell::new(slab::Slab::new()),
                request_subscriptions: RefCell::new(HashMap::new()),
                // TODO configurable pool size
                subscription_pool: RefCell::new(refpool::Pool::new(64).filled()),
                subscription_pending_requests: RefCell::new(HashMap::new()),
            }),
        }
    }

    pub fn subscribe(&self, plane_id: u32, request_topic: u32, callback: RequestSubscriptionCallback) {
        let mut subscriptions = self.inner.request_subscriptions.borrow_mut();
        let callbacks = subscriptions.entry((plane_id, request_topic))
            .or_insert(Vec::new());
        callbacks.push(callback);
    }

    pub fn handle_request<T: mproto::Owned>(&self, plane_id: u32, topic: u32, source: u64, packet: Packet) {
        let packet_header_len = <TransmitPacket as mproto::BaseLen>::BASE_LEN;

        let Ok(request) =
            mproto::decode_value::<RequestLazy<T>>(&packet.as_ref()[packet_header_len..])
        else {
            return;
        };
        let Ok(request_id) = request.request_id() else { return; };
        let Ok(source_worker) = request.worker() else { return; };

        if let Some(callbacks) = self.inner.request_subscriptions.borrow_mut().get_mut(&(plane_id, topic)) {
            for callback in callbacks {
                let pending_request = self.subscription_start_request(plane_id, source, source_worker, request_id);
                callback(packet.clone(), pending_request);
            }
        }
    }

    pub fn handle_response<T: mproto::Owned>(&self, plane_id: u32, local_addr: u64, local_worker: u16, packet: Packet) {
        let packet_header_len = <TransmitPacket as mproto::BaseLen>::BASE_LEN;
        let Ok(response) =
            mproto::decode_value::<ResponseLazy<T>>(&packet.as_ref()[packet_header_len..])
        else {
            return;
        };
        let Ok(requester) = response.requester() else { return; };
        let Ok(requester_worker) = response.requester_worker() else { return; };
        let Ok(request_id) = response.request_id() else { return; };

        if requester == local_addr && requester_worker == local_worker {
            // Local
            self.client_finish_request(request_id, packet.clone());
        }

        self.subscription_finish_request(plane_id, requester, requester_worker, request_id, packet);
    }

    pub fn client_start_request(&self) -> PendingRequest {
        let request_id = self.inner.pending_requests.borrow_mut().insert(PendingRequestState {
            response: Cell::new(None),
            waker: Cell::new(None),
        }) as u32;

        PendingRequest { state: self.inner.clone(), request_id }
    }

    fn client_finish_request(&self, request_id: u32, response_packet: Packet) {
        let local_pending_requests = self.inner.pending_requests.borrow();
        let Some(pending_request) = local_pending_requests.get(request_id as usize) else {
            // TODO metric - RequestTracker client received response with unknown request_id
            return;
        };
        pending_request.response.set(Some(response_packet));
        if let Some(waker) = pending_request.waker.take() {
            waker.wake();
        }
    }

    fn subscription_start_request(
        &self,
        plane_id: u32,
        requester: u64,
        requester_worker: u16,
        request_id: u32,
    ) -> PendingRequestSubscription {
        let mut pending_requests = self.inner.subscription_pending_requests.borrow_mut();
        let pending_request = pending_requests.entry((plane_id, requester, requester_worker, request_id))
            .or_insert(refpool::PoolRef::new(
                &mut *self.inner.subscription_pool.borrow_mut(),
                PendingRequestSubscriptionState {
                    response: RefCell::new(None),
                    pending_count: Cell::new(0),
                    waiters: localq::WaiterQueue::new(),
                },
            ));
        pending_request.pending_count.set(pending_request.pending_count.get() + 1);
        drop(pending_requests);

        PendingRequestSubscription {
            state: self.inner.clone(),
            plane_id,
            requester,
            requester_worker,
            request_id,
        }
    }

    fn subscription_finish_request(
        &self,
        plane_id: u32,
        requester: u64,
        requester_worker: u16,
        request_id: u32,
        response_packet: Packet,
    ) {
        let subscription_pending_requests = self.inner.subscription_pending_requests.borrow();
        let Some(pending_request) =
            subscription_pending_requests.get(&(plane_id, requester, requester_worker, request_id))
        else {
            return;
        };
        *pending_request.response.borrow_mut() = Some(response_packet);
        pending_request.waiters.notify_all();
    }
}

struct RequestTrackerInner {
    pending_requests: RefCell<slab::Slab<PendingRequestState>>,

    // Map request (plane ID, topic ID) to their response callbacks
    request_subscriptions: RefCell<HashMap<(u32, u32), Vec<RequestSubscriptionCallback>>>,

    subscription_pool: RefCell<refpool::Pool<PendingRequestSubscriptionState>>,
    // Map (plane_id, requester endpoint, requester worker_id, request_id) -> pending request state
    subscription_pending_requests: RefCell<HashMap<(u32, u64, u16, u32), refpool::PoolRef<PendingRequestSubscriptionState>>>,
}

struct PendingRequestState {
    response: Cell<Option<Packet>>,
    waker: Cell<Option<Waker>>,
}

pub struct PendingRequest {
    state: Rc<RequestTrackerInner>,
    request_id: u32,
}

impl PendingRequest {
    pub fn request_id(&self) -> u32 { self.request_id }
}

impl Future for PendingRequest {
    type Output = Packet;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let state = &self.state.pending_requests.borrow()[self.request_id as usize];

        if let Some(response_packet) = state.response.take() {
            Poll::Ready(response_packet)
        } else {
            state.waker.set(Some(cx.waker().clone()));
            Poll::Pending
        }
    }
}

impl Drop for PendingRequest {
    fn drop(&mut self) {
        self.state.pending_requests.borrow_mut().remove(self.request_id as usize);
    }
}

struct PendingRequestSubscriptionState {
    response: RefCell<Option<Packet>>,
    pending_count: Cell<u32>,
    waiters: localq::WaiterQueue,
}

pub struct PendingRequestSubscription {
    state: Rc<RequestTrackerInner>,
    plane_id: u32,
    requester: u64,
    requester_worker: u16,
    request_id: u32,
}

impl PendingRequestSubscription {
    pub async fn wait(self) -> Packet {
        let pending_request_key = (self.plane_id, self.requester, self.requester_worker, self.request_id);
        let pending_request_state = self.state.subscription_pending_requests.borrow_mut()[&pending_request_key].clone();

        pending_request_state.waiters
            .wait_for(|| pending_request_state.response.borrow().clone())
            .await
    }
}

impl Drop for PendingRequestSubscription {
    fn drop(&mut self) {
        use std::collections::hash_map::Entry;

        let pending_request_key = (self.plane_id, self.requester, self.requester_worker, self.request_id);
        let mut pending_requests = self.state.subscription_pending_requests.borrow_mut();
        let Entry::Occupied(mut state_entry) = pending_requests.entry(pending_request_key) else {
            panic!("std_modrpc::RequestTracker missing pending subscription state");
        };

        let pending_count = state_entry.get().pending_count.get();
        state_entry.get_mut().pending_count.set(pending_count - 1);
        if pending_count == 1 {
            // This was the last subscription response waiter to be dropped for the request_id.
            state_entry.remove();
        }
    }
}

pub fn get_request_tracker(setup: &modrpc::RoleSetup) -> RequestTracker {
    setup.worker_context().with_local_fn((), || RequestTracker::new(), |x| x.clone())
}
