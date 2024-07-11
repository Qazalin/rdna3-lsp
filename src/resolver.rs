use std::{error::Error, fs, process::Command};

use lsp_types::request::HoverRequest;
use lsp_types::{Hover, HoverContents};

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
        "textDocument/hover" => {
            let (id, params) = cast::<HoverRequest>(req)?;
            eprintln!("{params:?}");
            let params = params.text_document_position_params;
            let fp = params.text_document.uri.path().to_string();
            let content = fs::read_to_string(&fp)?;
            let line = content.split("\n").nth(params.position.line as usize);
            let words = line.unwrap().split_whitespace();
            let character_idx = match params.position.character == 0 {
                true => 0,
                false => params.position.character as usize - 1,
            };
            let mut val = "";
            let mut start = 0;
            for w in words {
                start += w.len();
                if start >= character_idx {
                    val = w;
                    break;
                }
            }
            let ref_path = "/Users/qazal/code/rdna3-lsp/ref.json";
            let output = Command::new("jq")
                .arg(format!(".{}", val))
                .arg(ref_path)
                .output()?;
            let value: Option<InstructionSpec> =
                serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
            let result = match value {
                Some(v) => {
                    let value = format!(
                        "{}\n```\n{}\n```\n{}",
                        v.desc,
                        v.code,
                        if v.notes.len() != 0 {
                            format!("*Notes*\n{}", v.notes)
                        } else {
                            "".to_string()
                        }
                    );
                    let ret = Hover {
                        contents: HoverContents::Markup(lsp_types::MarkupContent {
                            kind: lsp_types::MarkupKind::Markdown,
                            value,
                        }),
                        range: None,
                    };
                    Some(serde_json::to_value(ret).unwrap())
                }
                None => None,
            };

            Ok(Response {
                id,
                result,
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

#[cfg(test)]
mod test_resolver {
    use super::*;
    use serde_json::json;
    use std::{fs::File, io::Write};

    fn _helper_test_hover(seed: &str, line: usize, character: usize) -> Response {
        let fp = "/private/tmp/test.s";
        File::create(fp)
            .unwrap()
            .write_all(seed.as_bytes())
            .unwrap();
        resolve(Request {
            id: 1.into(),
            method: "textDocument/hover".into(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{fp}")
                },
                "position": {
                    "line": line,
                    "character": character
                },
                "workDoneToken": null
            }),
        })
        .unwrap()
    }

    #[test]
    fn test_hover_instruction() {
        assert!(_helper_test_hover("s_add_u32", 0, 0).result.is_some());
        assert!(_helper_test_hover("s_add_u32", 0, 2).result.is_some());
        assert!(_helper_test_hover("s_add_f32", 0, 0).result.is_none());
        assert!(_helper_test_hover("  s_add_u32", 0, 2).result.is_some());
        assert!(_helper_test_hover("  s_add_u32", 0, 10).result.is_some());
    }
}
