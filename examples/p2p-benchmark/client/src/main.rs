use std::sync::Arc;

use modrpc_executor::ModrpcExecutor;

fn main() {
    let task_count = 10000;
    let worker_count = 3;
    let iter_count = 30000000 / task_count / worker_count;

    let mut ex = modrpc_executor::TokioExecutor::new();
    let _guard = ex.tokio_runtime().enter();

    let buffer_pool = modrpc::HeapBufferPool::new(65536, 8, 8);
    let mut rt = modrpc::RuntimeBuilder::new_with_local(ex.spawner());
    let main_group = rt.new_worker_group(worker_count as u16);
    let (rt, _rt_shutdown) = rt.start::<modrpc_executor::TokioExecutor>();

    let start_tasks = Arc::new(no_std_async::semaphore::Semaphore::new(0));
    let finished_tasks = Arc::new(no_std_async::semaphore::Semaphore::new(0));

    ex.run_until(async move {
        // Pin worker threads
        rt.run_on_all_workers(move |worker_cx| {
            core_affinity::set_for_current(core_affinity::CoreId {
                id: worker_cx.worker_id().0 as usize,
            });
        })
        .await;

        let stream = tokio::net::TcpStream::connect("127.0.0.1:9090").await
            .expect("tcp stream connect");
        stream.set_nodelay(true).unwrap();

        println!("Connected to server.");

        let (_endpoint, _transport, _p2p_benchmark_client) =
            modrpc::tcp_connect_builder::<p2p_benchmark_modrpc::P2pBenchmarkClientRole, _>(
                &rt,
                buffer_pool.clone(),
                modrpc::WorkerId::local(),
                p2p_benchmark_modrpc::P2pBenchmarkClientConfig { },
                stream,
                {
                    let start_tasks = start_tasks.clone();
                    let finished_tasks = finished_tasks.clone();
                    async move |start_role| {
                        start_role
                            .on_worker_group(main_group, move |cx| {
                                start_p2p_benchmark_client(
                                    cx,
                                    task_count,
                                    iter_count,
                                    start_tasks.clone(),
                                    finished_tasks.clone(),
                                );
                            })
                            .await
                            // Note we run the transport on the local worker, so the client role
                            // needs to be setup on it for the responses arriving on the transport
                            // to be relayed to the requesting threads.
                            .local(|_cx| { })
                    }
                }
            )
            .await
            .unwrap();

        println!("Waiting for {} total tasks", task_count * worker_count);
        start_tasks.release(task_count * worker_count);
        let now = std::time::Instant::now();
        finished_tasks.acquire(task_count * worker_count).await;

        let elapsed = now.elapsed();
        let ns_per_iter = elapsed.as_nanos() / (worker_count * task_count * iter_count) as u128;
        println!("Total time: {:?}", elapsed);
        println!("ns per iter: {}", ns_per_iter);
        println!("Requests/second: {}", 1_000_000_000 / ns_per_iter);
    });
}

fn start_p2p_benchmark_client(
    cx: modrpc::RoleWorkerContext<p2p_benchmark_modrpc::P2pBenchmarkClientRole>,
    task_count: usize,
    iter_count: usize,
    start_tasks: Arc<no_std_async::semaphore::Semaphore>,
    finished_tasks: Arc<no_std_async::semaphore::Semaphore>,
) {
    for t in 0..task_count {
        let start_tasks = start_tasks.clone();
        let finished_tasks = finished_tasks.clone();
        let p2p_benchmark_client = cx.hooks.clone();
        cx.role_spawner().spawn(async move {
            start_tasks.acquire(1).await;

            for i in 0..iter_count {
                let req = (t as u64 * iter_count as u64) + i as u64;
                let resp = p2p_benchmark_client.test_request.call(req).await;
                assert_eq!(resp, Ok(req as u64 * 2 + 42));
            }

            finished_tasks.release(1);
        });
    }
}
