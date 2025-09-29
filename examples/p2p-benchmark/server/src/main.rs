use modrpc_executor::ModrpcExecutor;

fn main() {
    let mut ex = modrpc_executor::TokioExecutor::new();
    let _guard = ex.tokio_runtime().enter();

    let buffer_pool = modrpc::HeapBufferPool::new(65536, 8, 8);
    let rt = modrpc::RuntimeBuilder::new_with_local(ex.spawner());
    let (rt, _rt_shutdown) = rt.start::<modrpc_executor::TokioExecutor>();

    ex.run_until(async move {
        // Pin worker threads
        rt.run_on_all_workers(move |worker_cx| {
            core_affinity::set_for_current(core_affinity::CoreId {
                id: worker_cx.worker_id().0 as usize + 4,
            });
        })
        .await;

        let tcp_server = modrpc::TcpServer::new();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:9090").await
            .expect("tcp listener");

        loop {
            println!("Accepting dataplane stream");
            let (stream, client_addr) = match listener.accept().await {
                Ok(s) => s,
                Err(e) => {
                    println!("Failed to accept client: {}", e);
                    continue;
                }
            };
            stream.set_nodelay(true).unwrap();

            let _ = tcp_server.accept_local::<p2p_benchmark_modrpc::P2pBenchmarkServerRole>(
                &rt,
                buffer_pool.clone(),
                stream,
                start_p2p_benchmark,
                p2p_benchmark_modrpc::P2pBenchmarkServerConfig { },
                p2p_benchmark_modrpc::P2pBenchmarkInitState { },
            )
            .await
            .unwrap();

            println!("Accepted client {}", client_addr);
        }
    });
}

fn start_p2p_benchmark(
    cx: modrpc::RoleWorkerContext<p2p_benchmark_modrpc::P2pBenchmarkServerRole>,
) {
    cx.stubs.test_request.build_replier(cx.setup, async move |mut cx, request: u64| {
        cx.reply.send_ok(request * 2 + 42).await;
    });
}
