# eyeqwst

eyeqwst is a [Quaddle](/QWD/Quaddle) client library for Rust. It should work both with Tokio and on WASM.

## usage

(note: this example is untested)

``` toml
# Cargo.toml
[dependencies]
quaddlecl = { git = "https://codeberg.org/Makefile_dot_in/eyeqwst" }
```

``` rust
use quaddlecl::client::Client;
use quaddlecl::client::gateway::GatewayEvent;
use anyhow::Result;
use futures::TryStreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = Client::new()?;
    let (_session_id, _user) = client.login("meow", "the_meower")?;
    
    while let Some(event) = client.gateway_mut().try_next()?.await {
        match event {
            GatewayEvent::MessageCreate { message } => {
                if &*message.content == "hello" {
                    client.http()
                          .create_message(message.channel, "haii :3")
                          .await?;
                }
            },
            _ => {}
        }
    }
}
```
