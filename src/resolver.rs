use std::{error::Error, fs, process::Command};

use lsp_types::request::{GotoDefinition, HoverRequest};
use lsp_types::{GotoDefinitionResponse, Hover, HoverContents};

use lsp_server::{ExtractError, Request, RequestId, Response};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct InstructionSpec {
    pub desc: String,
    pub code: String,
    pub notes: String,
}

pub fn resolve(req: Request) -> Result<Response, Box<dyn Error + Sync + Send>> {
    match req.method.as_str() {
        "textDocument/completion" => {
            todo!("completion")
        }
        "textDocument/definition" => {
            let (id, params) = cast::<GotoDefinition>(req)?;
            eprintln!("got gotoDefinition request #{id}: {params:?}");
            let result = Some(GotoDefinitionResponse::Array(Vec::new()));
            let result = serde_json::to_value(&result).unwrap();
            Ok(Response {
                id,
                result: Some(result),
                error: None,
            })
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

            Ok(Response {
                id,
                result: Some(serde_json::to_value(&result).unwrap()),
                error: None,
            })
        }
        _ => todo!("{}", req.method),
    }
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}
