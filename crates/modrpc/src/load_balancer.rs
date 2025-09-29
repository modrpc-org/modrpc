use core::{cell::Cell, mem::MaybeUninit};
use std::rc::Rc;

use crate::{
    Packet, RoleSetup, SendPacket,
    packet_processor::PACKET_PROCESSOR_SOURCE_NEW,
    worker::{get_global_queue_receiver, get_global_queue_sender},
};

pub struct LoadBalancerConfig {
    pub rx_burst_size: usize,
    pub tx_burst_size: usize,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
struct LoadBalancerKey {
    plane_id: u32,
    topic: u32,
}

pub fn spawn_load_balancer(
    setup: &RoleSetup,
    config: LoadBalancerConfig,
    plane_id: u32,
    topic: u32,
    object_path: String,
    local_queue_tx: localq::mpsc::Sender<Packet>,
) {
    let is_flushing_overflow = Rc::new(Cell::new(false));

    let key = LoadBalancerKey { plane_id, topic };
    let capacity = 64;
    let shared_tx = get_global_queue_sender(setup, key, capacity, config.tx_burst_size);
    let shared_rx = get_global_queue_receiver(setup, key, capacity);

    setup.worker_context().route_to_local_queue(
        &object_path,
        PACKET_PROCESSOR_SOURCE_NEW,
        plane_id,
        topic,
        {
            let is_flushing_overflow = is_flushing_overflow.clone();
            let local_queue_tx = local_queue_tx.clone();
            move |_source, packet| {
                probius::trace_branch(|| {
                    if !is_flushing_overflow.get() {
                        match local_queue_tx.try_send(packet.clone()) {
                            Ok(_) => {
                                probius::trace_metric("local", 1);
                                None
                            }
                            Err(localq::mpsc::TrySendError::Full(_packet))
                            | Err(localq::mpsc::TrySendError::Shutdown(_packet)) => {
                                probius::trace_metric("start_batch", 1);
                                Some(shared_tx.clone())
                            }
                        }
                    } else {
                        probius::trace_metric("append_batch", 1);
                        Some(shared_tx.clone())
                    }
                })
            }
        },
    );

    // Spawn task to pull packets from shared mpmc queue and put them in the local queue.
    let rx_burst_size = config.rx_burst_size;
    setup.worker_context().spawn_traced(
        &object_path,
        core::time::Duration::from_millis(1000),
        async move |tracer| {
            let mut overflow: Vec<MaybeUninit<SendPacket>> =
                (0..rx_burst_size).map(|_| MaybeUninit::uninit()).collect();

            'recv_loop: loop {
                is_flushing_overflow.set(false);

                let mut m = 0;
                let Ok(n) = shared_rx
                    .recv(rx_burst_size, |r| {
                        let dst = (&mut overflow[..r.len()]).as_mut_ptr();
                        m = r.len();
                        unsafe {
                            r.read_to_ptr(dst as *mut SendPacket);
                        }
                    })
                    .await
                else {
                    break;
                };
                assert_eq!(n, m);

                tracer.trace(|| probius::trace_metric("batch_size", n as i64));

                if n > 0 {
                    is_flushing_overflow.set(true);

                    for i in 0..n {
                        let Ok(()) = local_queue_tx
                            .send(unsafe { overflow[i].assume_init_read().receive() })
                            .await
                        else {
                            break 'recv_loop;
                        };
                    }
                }
            }
        },
    );
}

pub fn proxy_load_balancer(setup: &RoleSetup, plane_id: u32, topic: u32, object_path: String) {
    let key = LoadBalancerKey { plane_id, topic };
    // TODO configurable
    let capacity = 64;
    let tx_burst_size = 32;

    let sender = get_global_queue_sender(setup, key, capacity, tx_burst_size);
    setup.worker_context().add_local_queue(
        &object_path,
        PACKET_PROCESSOR_SOURCE_NEW,
        plane_id,
        topic,
        sender,
    );
}
