# modrpc

*Experimental, not ready for production*

Modrpc is a framework that aims to empower everyone to build software that communicates efficiently. At a high level, it is similar to RPC frameworks such as gRPC, Thrift, and Cap'n Proto. Modrpc does a few things differently, though:

- Modular - interfaces all the way down. Define re-usable interface building blocks, implement once in Rust, use everywhere. Modrpc's standard primitives (e.g. `std.Request`, `std.ByteStream`) are just another interface.
- Single portable core runtime implementation in Rust - without this, composable interfaces would not be possible.
- Performant - messages are automatically batched at various parts of the system to massively improve throughput without compromising latency.
- Lightweight - on a modern x86 CPU the overhead per request at a server can be as low as 200ns amortized and 1500ns without batching.

A quick example of modrpc's IDL from the [standard library](https://github.com/modrpc-org/modrpc/blob/main/proto/std.modrpc):
```
interface Request<Req, Resp> @(Client, Server) {
    // Requests are sent from clients and received by servers as well as other clients.
    events @(Client) -> @(Client, Server) {
        private request: Request<Req>,
    }
    // Responses are sent from servers to clients.
    events @(Server) -> @(Client) {
        private response: Response<Resp>,
    }
    // Server implementations must provide a callback to handle requests.
    impl @(Server) {
        handler: async Req -> Resp,
    }
    // Client handles have a `call` method to initiate a request and await the response.
    methods @(Client) {
        call: async Req -> Resp,
    }
}

struct Request<T> {
    request_id: u32,
    worker: u16,
    payload: T,
}

struct Response<T> {
    request_id: u32,
    requester: u64,
    requester_worker: u16,
    payload: T,
}
```

The client role is implemented [here](https://github.com/modrpc-org/modrpc/blob/main/std-modrpc/rust/src/role_impls/request_client.rs)
The server role is implemented [here](https://github.com/modrpc-org/modrpc/blob/main/std-modrpc/rust/src/role_impls/request_server.rs)
The "getting started" tutorial below demonstrates usage of the `Request` primitive.

## Getting started

See the modrpc book: https://modrpc-org.github.io/book/getting-started.html

## Sample applications

Currently there is only one: [the chat example](https://github.com/modrpc-org/chat-modrpc-example)

## Status

Modrpc is still very experimental - consider it a research project. While I would love for you to play with it, do not use it to build mission-critical things.

Currently Rust is the only supported host language for applications. The intention is to eventually add support for at least TypeScript and Python. There was a previous working POC of TypeScript integration via the WebAssembly Component Model [see here](https://github.com/modrpc-org/modrpc/blob/main/crates/modrpc-codegen/src/codegen/wasm/mod.rs), but it has since bitrotted. For now I want to focus on polishing the Rust side of things. When I come back to it, I'm considering just using raw WebAssembly (no component model) and/or something like UniFFI.

## License

Apache 2.0
