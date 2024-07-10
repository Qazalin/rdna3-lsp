use std::{error::Error, fs, process::Command};

use lsp_types::request::{GotoDefinition, HoverRequest};
use lsp_types::OneOf;
use lsp_types::{
    GotoDefinitionResponse, Hover, HoverContents, HoverProviderCapability, InitializeParams,
    ServerCapabilities,
};

use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct InstructionSpec {
    desc: String,
    code: String,
    notes: String,
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    // Note that  we must have our logging only write out to stderr.
    eprintln!("starting generic LSP server");

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        definition_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
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

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}
fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();
    eprintln!("starting example main loop");
    for msg in &connection.receiver {
        eprintln!("got msg: {msg:?}");
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                eprintln!("got request: {req:?}");
                match req.method.as_str() {
                    "textDocument/completion" => {
                        todo!("completion")
                    }
                    "textDocument/definition" => {
                        let (id, params) = cast::<GotoDefinition>(req)?;
                        eprintln!("got gotoDefinition request #{id}: {params:?}");
                        let result = Some(GotoDefinitionResponse::Array(Vec::new()));
                        let result = serde_json::to_value(&result).unwrap();
                        let ret = Response {
                            id,
                            result: Some(result),
                            error: None,
                        };
                        connection.sender.send(Message::Response(ret))?;
                        continue;
                    }
                    "textDocument/hover" => {
                        let (id, params) = cast::<HoverRequest>(req)?;
                        let params = params.text_document_position_params;
                        let fp = params.text_document.uri.path().to_string();
                        let content = fs::read_to_string(&fp)?;
                        let line = content.split("\n").nth(params.position.line as usize);
                        let mut character = "";
                        let mut base = 0;
                        for (i, x) in line.unwrap().split_whitespace().enumerate() {
                            if i + base >= params.position.character as usize {
                                character = x;
                                break;
                            }
                            base += x.len()
                        }
                        let ref_path = "/Users/qazal/code/rdna3-lsp/ref.json";
                        let output = Command::new("jq")
                            .arg(format!(".{}", character))
                            .arg(ref_path)
                            .output()?;
                        let value: InstructionSpec =
                            serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
                        let value = format!(
                            "{}\n```\n{}\n```\n{}",
                            value.desc,
                            value.code,
                            if value.notes.len() != 0 {
                                format!("*Notes*\n{}", value.notes)
                            } else {
                                "".to_string()
                            }
                        );
                        let result = Some(Hover {
                            contents: HoverContents::Markup(lsp_types::MarkupContent {
                                kind: lsp_types::MarkupKind::Markdown,
                                value,
                            }),
                            range: None,
                        });

                        let ret = Response {
                            id,
                            result: Some(serde_json::to_value(&result).unwrap()),
                            error: None,
                        };
                        connection.sender.send(Message::Response(ret))?;
                        continue;
                    }
                    _ => todo!("{}", req.method),
                }
            }
            Message::Response(resp) => {
                eprintln!("got response: {resp:?}");
            }
            Message::Notification(not) => {
                eprintln!("got notification: {not:?}");
            }
        }
    }
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}
