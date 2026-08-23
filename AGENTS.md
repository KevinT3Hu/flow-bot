# AGENTS.md

## Overview

`flow-bot` is a Rust SDK for OneBot 11 that simplifies bot creation, modeled after axum's handler/extractor design. Runtime is tokio. All four OneBot 11 communication types are supported behind one `ConnectionConfig` enum (`src/base/transport/`): forward WebSocket and HTTP use `tokio-tungstenite`/`reqwest` as clients; reverse WebSocket and HTTP POST run embedded servers on `axum`. There is no README; the primary documentation is the `//!` doc comments in `src/lib.rs` (with `no_run` doctests).

## Layout

- Workspace: root crate + `flow-bot-macros/` (proc-macro crate providing `#[flow_service]`, `#[flow_filter]`, `#[group_message]`, `#[private_message]`).
- `src/base/` — core: `FlowBot` run loop + dispatcher (`bot.rs`, `DispatchMode` ordered-by-default), `FlowBotBuilder` (`with_state`/`with_handler`/`with_service`), `BotContext` shared state (`context.rs`, dashmap), `Handler` trait + tuple impls + type erasure (`handler.rs`), `HandlerControl`/`HandlerError` (`control.rs`), middleware (`middleware.rs`).
- `src/base/transport/` — the four connection types: `ConnectionConfig` + validation + `ApiTransport` trait (`mod.rs`), forward WS client with reconnection (`forward_ws.rs`), reverse WS server (`reverse_ws.rs`), HTTP API client (`http.rs`), HTTP POST webhook server with `X-Signature` verification (`http_post.rs`), shared WS session plumbing — echo-keyed pending requests, frame routing (`ws.rs`).
- `src/api/` — OneBot API calls, the `ApiExt` trait implemented by `BotContext`, and retcode-checked response parsing (`parse_api_response`). Generic escapes: `send_message`/`MessageTarget` (`send_msg`), `call_action` (raw action + params, covers the `_async`/`_rate_limited` suffixes), `handle_quick_operation`/`QuickOperation`, `reply`.
- `src/event/` — `BotEvent`/`Event`/`TypedEvent` plus message/notice/request/meta-event types; `notify` sub-events are typed (`NotifyEvent`: poke/lucky_king/honor + untyped fallback) alongside the de-facto `essence` notice; all event types are `Serialize` (quick-op contexts re-serialize the parsed event).
- `src/extract/` — axum-style `FromEvent` extractors: messages, segments, filters, events; `command.rs` is feature-gated.
- `src/message/` — message segments, `IntoMessage` (`message_ext.rs`), and the CQ-code codec (`cq.rs`). `Message` is a newtype: it deserializes the CQ string, segment array, and single-segment forms, always serializes to the array form, and `Display`s as the CQ string. `Segment::Unknown` confines unrecognized segment types; 0/1 flags are the `Flag` type (sent as integers, parsed leniently).
- `src/extensions/turso.rs` — Turso/libSQL persistence (feature-gated).
- `src/error.rs` — single `FlowError` enum.
- `examples/` — `simple.rs`, `filters.rs` (forward WS), `reverse_ws.rs`, `webhook.rs` (HTTP POST).
- `tests/` — integration tests against a fake OneBot implementation (`common/mod.rs` harness) plus builder/validation unit tests.

## Commands

- Toolchain is **nightly** (enforced by `rust-toolchain.toml`). The `command` and `turso` features require it (`adt_const_params` / `unsized_const_params`).
- `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`.
- Run examples: `cargo run --example simple`.
- Features: `default = ["macros", "tls"]`; optional `command` (clap derive), `turso` (both imply `nightly`), and `tls` (rustls for `wss://`/`https://` — build-time validation rejects those schemes without it).

## Conventions

- Errors: `thiserror`-derived `FlowError` in `src/error.rs`. Handlers return `Result<HandlerControl, HandlerError>`; call `HandlerError::skip()` to pass an event to the next handler.
- Traits use `#[async_trait]` (`Handler`, `Service`, `FromEvent`, `ErasedHandler`); all public types are `Send + Sync + 'static`.
- Serde enums use `#[serde(rename_all = "snake_case")]`.
- No `unsafe` anywhere (src and macros) — keep it that way.
- Logging uses `tracing` (currently only in `src/base/bot.rs`); do not use `println!` in library code.

## Gotchas

- `Handler` is implemented for functions via `macro_rules!` tuple expansion (`all_tuples!`/`impl_handler!` in `src/base/handler.rs`). An extractor returning `None` skips the handler rather than erroring. (Impls start at one extractor — zero-argument handlers don't compile yet.)
- Outbound API calls are transport-agnostic: `Context` holds an `Arc<dyn ApiTransport>` slot; WS transports install a `WsSession` (echo-keyed oneshot pending map + writer channel), HTTP transports a reqwest-based caller. `Service::init` runs exactly once per bot regardless of reconnections.
- Events flow through a bounded `mpsc` queue of **parsed** `BotEvent`s into the dispatcher (`DispatchMode::Ordered` by default, `Concurrent` opt-in); a full queue applies backpressure to the connection. Transports parse via `bot::parse_event`.
- Quick operations: the HTTP-POST webhook registers a per-event slot in `Context` keyed by the `Arc<Event>` allocation address *before* enqueueing, then answers the POST with the slot's `QuickOperation` (or 204). The dispatcher must call `Context::finish_event` after each event — it wakes the waiting webhook response. `ApiExt::handle_quick_operation` fills a pending slot when there is one, else falls back to the hidden `.handle_quick_operation` API action. Handlers must pass the **same** `BotEvent` they received (an `Arc` clone), not a cloned `Event`, or the slot lookup misses.
- Graceful shutdown uses `tokio::sync::Notify` + an `AtomicBool` (`BotShared::wait_shutdown` registers the `notified()` future before checking the flag — keep that ordering); `run()` returns `Ok(())` on shutdown. Forward-WS reconnection uses exponential backoff (`ReconnectionStrategy::Infinite` is the default); server transports don't reconnect (the implementation retries per spec).
- Keep `flow-bot` and `flow-bot-macros` crate versions in lockstep (path + version dependency).
- Read the `src/lib.rs` module docs before changing the public handler/extractor API — they are the de facto design spec.
