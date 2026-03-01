# Flow-Bot Plugin SDK

A thin wrapper SDK for building Flow-Bot WASM plugins with minimal boilerplate.

## Overview

This SDK provides a simple interface for building Flow-Bot plugins:

1. **PluginHandler Trait**: Implement a trait with typed `Event` parameter (no raw bytes!)
2. **Automatic WIT Bindings**: Generated at compile time - no need to reference WIT files
3. **Event Deserialization**: Automatic MessagePack deserialization to typed events
4. **Re-exports**: Convenient access to `flow-bot-onebot11` and `flow-bot-extractor`

**Key Benefit**: You get a clean, typed API without dealing with WIT files, raw bytes, or manual serialization.

## Installation

Add to your plugin's `Cargo.toml`:

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
flow-bot-plugin-sdk = { path = "../../flow-bot-plugin-sdk" }

[profile.release]
opt-level = "s"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
strip = true         # Strip debug symbols
```

## Quick Start

### Minimal Plugin

```rust
use flow_bot_plugin_sdk::*;

// Export plugin - this generates all WIT bindings automatically
export_plugin!(MyPlugin);

// Define your plugin
#[derive(Default)]
struct MyPlugin;

// Implement the PluginHandler trait
impl PluginHandler for MyPlugin {
    fn handle_event(&self, event: Event) -> Result<(), String> {
        // Handle the event - it's already deserialized!
        match event.event {
            TypedEvent::Message(msg) => {
                eprintln!("Got message: {}", msg.raw_message);
            }
            _ => {}
        }
        Ok(())
    }
    
    fn name(&self) -> &str { "my-plugin" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }
    fn description(&self) -> &str { "My awesome plugin" }
}
```

That's it! No WIT file references, no manual deserialization, no Guest trait implementation.

### Responding to Messages

```rust
use flow_bot_plugin_sdk::*;

export_plugin!(PingPlugin);

#[derive(Default)]
struct PingPlugin;

impl PluginHandler for PingPlugin {
    fn handle_event(&self, event: Event) -> Result<(), String> {
        if let TypedEvent::Message(msg) = event.event {
            if msg.raw_message.trim() == "!ping" {
                // Use the auto-generated API
                if let Some(group_id) = msg.group_id {
                    api::send_group_message(group_id, "Pong!".to_string(), None)?;
                } else {
                    api::send_private_message(msg.user_id, "Pong!".to_string(), None)?;
                }
            }
        }
        Ok(())
    }
    
    fn name(&self) -> &str { "ping-plugin" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Responds to ping" }
}
```

### Using the Command Extractor

```rust
use flow_bot_plugin_sdk::*;

export_plugin!(CommandPlugin);

#[derive(Default)]
struct CommandPlugin;

impl PluginHandler for CommandPlugin {
    fn handle_event(&self, event: Event) -> Result<(), String> {
        if let TypedEvent::Message(msg) = event.event {
            // Use extractor to parse commands
            if let Some(cmd) = extractor::extract(&msg.raw_message) {
                match cmd.command.as_str() {
                    "hello" => {
                        let response = format!("Hello, {}!", 
                            cmd.args.join(" "));
                        self.send_response(&msg, &response)?;
                    }
                    "info" => {
                        let info = format!(
                            "Plugin: {}\nVersion: {}\nDescription: {}",
                            self.name(),
                            self.version(),
                            self.description()
                        );
                        self.send_response(&msg, &info)?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    fn name(&self) -> &str { "command-plugin" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Command handler" }
}

impl CommandPlugin {
    fn send_response(&self, msg: &Message, text: &str) -> Result<(), String> {
        if let Some(group_id) = msg.group_id {
            api::send_group_message(group_id, text.to_string(), None)?;
        } else {
            api::send_private_message(msg.user_id, text.to_string(), None)?;
        }
        Ok(())
    }
}
```

## API Reference

### PluginHandler Trait

The main trait you implement for your plugin:

```rust
pub trait PluginHandler {
    /// Handle an incoming event
    fn handle_event(&self, event: Event) -> Result<(), String>;
    
    /// Get the plugin name
    fn name(&self) -> &str;
    
    /// Get the plugin version
    fn version(&self) -> &str;
    
    /// Get the plugin description
    fn description(&self) -> &str;
}
```

**Important**: Your plugin struct must implement `Default`. This is used by the SDK to create instances when calling the trait methods.

### export_plugin! Macro

Generates all WIT bindings and exports your plugin:

```rust
export_plugin!(YourPluginStruct);
```

This macro:
- Generates WIT bindings from the SDK's bundled WIT files
- Creates a bridge between WIT's `Guest` trait and your `PluginHandler`
- Handles automatic event deserialization
- Exports `api` and `types` modules for API calls

After calling this macro, you can use:
- `api::*` - All OneBot-11 API functions
- `types::*` - All WIT type definitions

### Event Types

The SDK re-exports all event types from `flow-bot-onebot11`:

#### Event

Top-level event structure:

```rust
pub struct Event {
    pub time: i64,          // Unix timestamp
    pub self_id: i64,       // Bot's user ID
    pub event: TypedEvent,  // The actual event
}
```

#### TypedEvent

Enum of all event types:

```rust
pub enum TypedEvent {
    Message(Box<Message>),      // Message event
    Notice(Notice),             // Notice event
    Request(Request),           // Request event
    MetaEvent(MetaEvent),       // Meta event (heartbeat, lifecycle)
    Unknown(serde_json::Value), // Unknown event type
}
```

#### Message

Message event (see `flow-bot-onebot11` for full details):

```rust
pub struct Message {
    pub message_id: i64,
    pub user_id: i64,
    pub message: String,        // With CQ codes
    pub raw_message: String,    // Plain text
    pub group_id: Option<i64>,  // Present for group messages
    // ... more fields
}
```

### API Functions

After `export_plugin!()`, you get access to the full OneBot-11 API:

```rust
// Send messages
api::send_private_message(user_id, message, auto_escape)?;
api::send_group_message(group_id, message, auto_escape)?;

// Delete messages
api::delete_message(message_id)?;

// Get info
api::get_login_info()?;
api::get_stranger_info(user_id, no_cache)?;
api::get_group_info(group_id, no_cache)?;
api::get_group_member_info(group_id, user_id, no_cache)?;

// Group management
api::set_group_kick(group_id, user_id, reject_add_request)?;
api::set_group_ban(group_id, user_id, duration)?;
api::set_group_admin(group_id, user_id, enable)?;
api::set_group_card(group_id, user_id, card)?;
api::set_group_name(group_id, group_name)?;

// And many more...
```

All API functions return `Result<T, String>` where T is the response type.

### Re-exports

#### onebot11

Full re-export of `flow-bot-onebot11`:

```rust
use flow_bot_plugin_sdk::onebot11;

// Access all types:
// - onebot11::event::*
// - onebot11::message::*
// - onebot11::api::*
```

#### extractor

Full re-export of `flow-bot-extractor`:

```rust
use flow_bot_plugin_sdk::extractor;

if let Some(cmd) = extractor::extract("!ping hello") {
    println!("Command: {}", cmd.command);  // "ping"
    println!("Args: {:?}", cmd.args);       // ["hello"]
}
```

#### Other Re-exports

- `wit_bindgen` - For advanced WIT usage
- `rmp_serde` - For custom MessagePack handling
- `serde` - For serialization support

## Building Your Plugin

### Build Command

```bash
cargo build --target wasm32-wasip2 --release
```

### Output

The WASM file will be at:
```
target/wasm32-wasip2/release/your_plugin_name.wasm
```

Copy this to your Flow-Bot `plugins/` directory for hot-reloading!

## Project Structure

```
my-plugin/
├── Cargo.toml
└── src/
    └── lib.rs
```

**Cargo.toml**:
```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
flow-bot-plugin-sdk = { path = "../../flow-bot-plugin-sdk" }

[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
strip = true
```

**src/lib.rs**:
```rust
use flow_bot_plugin_sdk::*;

export_plugin!(MyPlugin);

#[derive(Default)]
struct MyPlugin;

impl PluginHandler for MyPlugin {
    fn handle_event(&self, event: Event) -> Result<(), String> {
        // Your code here
        Ok(())
    }
    
    fn name(&self) -> &str { "my-plugin" }
    fn version(&self) -> &str { "0.1.0" }
    fn description(&self) -> &str { "Description" }
}
```

## Key Features

### ✨ No WIT File Management

You don't need to copy or reference WIT files. The SDK handles all WIT binding generation automatically at compile time.

### ✨ Typed Events

You receive fully deserialized `Event` objects, not raw bytes. The SDK handles all MessagePack deserialization.

### ✨ Simple Interface

Just implement `PluginHandler` - no need to understand WIT bindings or the `Guest` trait.

### ✨ Automatic API Generation

The `export_plugin!` macro generates all API bindings automatically.

### ✨ Zero Overhead

Despite the convenience, the SDK adds no runtime overhead. All binding generation happens at compile time.

### ✨ Fast Plugin Development

Just implement `PluginHandler`, call `export_plugin!()`, and you're done. No build scripts, no WIT file copying, no manual binding setup.

## Common Patterns

### Pattern Matching Events

```rust
fn handle_event(&self, event: Event) -> Result<(), String> {
    match event.event {
        TypedEvent::Message(msg) => self.handle_message(*msg)?,
        TypedEvent::Notice(notice) => self.handle_notice(notice)?,
        TypedEvent::Request(req) => self.handle_request(req)?,
        TypedEvent::MetaEvent(meta) => self.handle_meta(meta)?,
        TypedEvent::Unknown(_) => {}
    }
    Ok(())
}
```

### Helper Methods

```rust
impl MyPlugin {
    fn reply(&self, msg: &Message, text: &str) -> Result<(), String> {
        if let Some(group_id) = msg.group_id {
            api::send_group_message(group_id, text.to_string(), None)?;
        } else {
            api::send_private_message(msg.user_id, text.to_string(), None)?;
        }
        Ok(())
    }
}
```

### Command Routing

```rust
fn handle_message(&self, msg: Message) -> Result<(), String> {
    if let Some(cmd) = extractor::extract(&msg.raw_message) {
        match cmd.command.as_str() {
            "ping" => self.handle_ping(&msg)?,
            "help" => self.handle_help(&msg)?,
            "info" => self.handle_info(&msg)?,
            _ => {}
        }
    }
    Ok(())
}
```

## Examples

See the [plugin-example](../../examples/plugin-example/) directory for a complete working example that demonstrates:

- Message handling
- Command parsing with extractor
- API usage
- Event type handling

## Troubleshooting

### Plugin Won't Compile

Make sure you have the `wasm32-wasip2` target:
```bash
rustup target add wasm32-wasip2
```

### "Default trait not implemented"

Your plugin struct must derive or implement `Default`:
```rust
#[derive(Default)]
struct MyPlugin;
```

Or:
```rust
impl Default for MyPlugin {
    fn default() -> Self {
        MyPlugin { /* fields */ }
    }
}
```

### API Functions Not Available

Make sure you've called `export_plugin!` before trying to use the API:
```rust
export_plugin!(MyPlugin);  // Must be called first

// Now you can use:
api::send_group_message(/* ... */)?;
```

## License

AGPL-3.0-only

## See Also

- [Flow-Bot Main Docs](../../README.md)
- [Plugin Quick Start](../../PLUGIN_QUICKSTART.md)
- [Runtime Implementation](../../RUNTIME_IMPLEMENTATION.md)
- [flow-bot-onebot11 Docs](../flow-bot-onebot11/)
- [flow-bot-extractor Docs](../flow-bot-extractor/)