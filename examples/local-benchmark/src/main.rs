use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use modrpc_executor::ModrpcExecutor;

fn main() {
    //let probius_flusher = probius::init_tcp_sink("modrpc-local-benchmark", "127.0.0.1:9876");

    let mut ex = modrpc_executor::TokioExecutor::new();
    let _guard = ex.tokio_runtime().enter();

    let mut rt = modrpc::RuntimeBuilder::new_no_local();
    let main_group = rt.new_worker_group(2);
    let client_group = rt.new_worker_group(3);
    let (rt, rt_shutdown) = rt
        .start::<modrpc_executor::TokioExecutor>();

    ex.run_until(async {
        // Pin worker threads
        rt.run_on_all_workers(move |worker_cx| {
            core_affinity::set_for_current(core_affinity::CoreId {
                id: worker_cx.worker_id().0 as usize,
            });
        })
        .await;

        /*rt.run_on_all_workers(move |worker_cx| {
            worker_cx.spawn_interval_loop(
                std::time::Duration::from_millis(5),
                {
                    let probius_flusher = probius_flusher.clone();
                    move || probius_flusher.flush()
                },
            );
        })
        .await;*/

        // Use separate buffer pools for client and server to avoid one being able to starve the
        // other of buffers.
        let server_transport = rt.add_transport(modrpc::LocalTransport {
            buffer_size: 8192,
            buffer_pool_batches: 8,
            buffer_pool_batch_size: 8,
        })
        .await;
        let client_transport = rt.add_transport(modrpc::LocalTransport {
            buffer_size: 8192,
            buffer_pool_batches: 8,
            buffer_pool_batch_size: 8,
        })
        .await;

        rt.start_role(modrpc::RoleConfig {
            plane_id: 0,
            endpoint_addr: modrpc::EndpointAddr { endpoint: 0 },
            transport: server_transport,
            topic_channels: modrpc::TopicChannels::SingleChannel { channel_id: 0 },
            config: foo_modrpc::FooClientConfig { },
            init: foo_modrpc::FooInitState {
                fooness: std_modrpc::PropertyInitState { value: 42 },
            },
        })
        .on_worker_group(main_group, start_foo_service).await
        .on_worker_group(client_group, |cx: modrpc::RoleWorkerContext<foo_modrpc::FooClientRole>| {
            cx.stubs.foo_the_bar.build_proxied(cx.setup);
        })
        .await;
        
        let iter_count = 30000000;
        let task_count = 100;

        // Miri config:
        //let iter_count = 30;
        //let task_count = 2;

        let worker_count = client_group.worker_count() as usize;
        let start_tasks = Arc::new(no_std_async::semaphore::Semaphore::new(0));
        let finished_tasks = Arc::new(no_std_async::semaphore::Semaphore::new(0));

        let start_foo_plane = rt.start_role::<foo_modrpc::FooServerRole>(modrpc::RoleConfig {
            plane_id: 0,
            endpoint_addr: modrpc::EndpointAddr { endpoint: 0 },
            transport: client_transport,
            topic_channels: modrpc::TopicChannels::SingleChannel { channel_id: 0 },
            config: foo_modrpc::FooServerConfig { },
            init: foo_modrpc::FooInitState {
                fooness: std_modrpc::PropertyInitState { value: 42 },
            },
        })
        .on_worker_group(main_group, move |_cx| { }).await
        .on_worker_group(client_group, {
            let start_tasks = start_tasks.clone();
            let finished_tasks = finished_tasks.clone();
            move |cx| {
                println!("Setting up client {:?}", cx.setup.worker_id());

                for t in 0..task_count {
                    let foo_client = cx.hooks.clone();
                    let start_tasks = start_tasks.clone();
                    let finished_tasks = finished_tasks.clone();
                    let worker_id = cx.setup.worker_id();
                    cx.setup.role_spawner().spawn(async move {
                        let task_request_count =
                            iter_count / task_count as u128 / worker_count as u128;

                        start_tasks.acquire(1).await;

                        let mut worst_latency = Duration::from_secs(0);
                        let mut sum_latency = Duration::from_secs(0);

                        for i in 0..task_request_count {
                            let start = Instant::now();
                            let req = (t as u32 * task_request_count as u32) + i as u32;
                            let resp = foo_client.foo_the_bar.call(req).await;
                            assert_eq!(resp, Ok(req as u64 * 2 + 42));
                            let latency = start.elapsed();
                            sum_latency += latency;
                            worst_latency = std::cmp::max(worst_latency, latency);
                        }

                        finished_tasks.release(1);

                        println!(
                            "Finished task worker={:?} task={} avg_latency={}ns worst_latency={}us",
                            worker_id, t,
                            worst_latency.as_micros(),
                            sum_latency.as_nanos() / task_request_count as u128,
                        );
                    });
                }
                println!("Client {:?} setup finished!", cx.setup.worker_id());
            }
        })
        .await;
        let foo_shutdown_signal = start_foo_plane.role_shutdown_signal.clone();

        println!("Waiting for {} total tasks", task_count * worker_count);
        assert_eq!(iter_count as usize % (task_count * worker_count), 0);
        start_tasks.release(task_count * worker_count);
        let now = Instant::now();
        finished_tasks.acquire(task_count * worker_count).await;

        if iter_count >= 1000 {
            let elapsed = now.elapsed();
            let ps_per_iter = elapsed.as_nanos() / (iter_count / 1000);
            println!("Total time: {:?}", elapsed);
            println!("ns per iter: {}", ps_per_iter / 1000);
            println!("Requests/second: {}", 1000000000000 / ps_per_iter);
        }

        foo_shutdown_signal.notify();

        // TODO This needs to also wait for local worker's tasks to complete.
        rt_shutdown.shutdown().await;
    });
}

fn start_foo_service(
    cx: modrpc::RoleWorkerContext<foo_modrpc::FooClientRole>,
) {
    let mut foo_the_bar = FooTheBarHandler {
        worker_id: cx.setup.worker_id(),
        x: 0,
    };
    cx.stubs.foo_the_bar.build_replier(
        cx.setup,
        async move |cx, r| foo_the_bar.call_replier(cx, r).await,
    );
}

pub struct FooTheBarHandler {
    pub worker_id: u16,
    pub x: u64,
}

impl FooTheBarHandler {
    async fn call_replier(
        &mut self,
        mut cx: std_modrpc::RequestContext<'_, Result<u64, String>>,
        request: u32,
    ) {
        self.x = self.x.wrapping_add(request as u64);
        cx.reply.send_ok(request as u64 * 2 + 42).await;
    }
}
