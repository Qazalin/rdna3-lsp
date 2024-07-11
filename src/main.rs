use std::error::Error;

use lsp_types::OneOf;
use lsp_types::{
    CompletionOptions, HoverProviderCapability, InitializeParams, ServerCapabilities,
    WorkDoneProgressOptions,
};

use lsp_server::{Connection, Message};
mod resolver;

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        definition_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(true),
            trigger_characters: Some({
                let mut tc: Vec<_> = ('a'..='z').map(|c| c.to_string()).collect();
                tc.push("_".to_string());
                tc
            }),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
            completion_item: None,
        }),
        ..Default::default()
    })
    .unwrap();
    let initialization_params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    Ok(())
}
fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                let ret = resolver::resolve(req)?;
                connection.sender.send(Message::Response(ret))?;
                continue;
            }
            _ => {}
        }
    }
    Ok(())
}
