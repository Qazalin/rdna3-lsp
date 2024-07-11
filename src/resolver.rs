use std::{error::Error, fs, process::Command};

use lsp_types::request::{Completion, HoverRequest};
use lsp_types::{
    CompletionItem, CompletionResponse, Hover, HoverContents, TextDocumentPositionParams,
};

use lsp_server::{ExtractError, Request, RequestId, Response};
use serde::{de::DeserializeOwned, Deserialize};

#[derive(Debug, Deserialize)]
pub struct InstructionSpec {
    pub desc: String,
    pub code: String,
    pub notes: String,
}

fn read_text(params: &TextDocumentPositionParams) -> Result<String, Box<dyn Error + Sync + Send>> {
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
    Ok(val.to_string())
}

fn jq<T>(arg: String) -> Result<Option<T>, Box<dyn Error + Sync + Send>>
where
    T: DeserializeOwned,
{
    let ref_path = "/Users/qazal/code/rdna3-lsp/ref.json";
    let output = Command::new("jq").arg(arg).arg(ref_path).output()?;
    Ok(serde_json::from_str(&String::from_utf8_lossy(&output.stdout)).unwrap())
}

pub fn resolve(req: Request) -> Result<Response, Box<dyn Error + Sync + Send>> {
    match req.method.as_str() {
        "textDocument/completion" => {
            let (id, params) = cast::<Completion>(req)?;
            let ret = match params.context.unwrap().trigger_character {
                Some(_) => {
                    let text = read_text(&params.text_document_position)?;
                    let completions = match jq::<Vec<String>>(format!(
                        r#". | to_entries | map(select(.key | startswith("{text}")).key)"#,
                    ))? {
                        Some(matches) => matches
                            .iter()
                            .map(|m| CompletionItem {
                                label: m.to_string(),
                                ..Default::default()
                            })
                            .collect(),
                        None => vec![],
                    };
                    let ret = CompletionResponse::Array(completions);
                    Some(ret)
                }
                None => None,
            };
            let result = Some(serde_json::to_value(ret).unwrap());
            Ok(Response {
                id,
                result,
                error: None,
            })
        }
        "completionItem/resolve" => Ok(Response {
            id: req.id,
            result: None,
            error: None,
        }),
        "textDocument/hover" => {
            let (id, params) = cast::<HoverRequest>(req)?;
            eprintln!("{params:?}");
            let text = read_text(&params.text_document_position_params)?;
            let value = jq::<InstructionSpec>(format!(".{}", text))?;
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
    use lsp_types::*;
    use serde_json::{json, Value};
    use std::{fs::File, io::Write};

    static FP: &'static str = "/private/tmp/test.s";
    fn _seed_file(seed: &str) {
        File::create(FP)
            .unwrap()
            .write_all(seed.as_bytes())
            .unwrap();
    }

    fn _helper_test_hover(seed: &str, line: usize, character: usize) -> Response {
        _seed_file(seed);
        resolve(Request {
            id: 1.into(),
            method: "textDocument/hover".into(),
            params: json!({
                "textDocument": {
                    "uri": format!("file://{FP}")
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

    fn _helper_test_complete(
        seed: &str,
        line: usize,
        character: usize,
    ) -> Option<Vec<CompletionItem>> {
        _seed_file(seed);

        let res = resolve(Request {
            id: 1.into(),
            method: "textDocument/completion".into(),
            params: json!({
                "context": {
                    "triggerKind": 1,
                    "triggerCharacter": "u"
                },
                "textDocument": {
                    "uri": format!("file://{FP}")
                },
                "position": {
                    "line": line,
                    "character": character
                },
                "workDoneToken": null
            }),
        });
        serde_json::from_value(res.unwrap().result.unwrap_or(Value::default())).unwrap()
    }

    #[test]
    fn test_autocomplete() {
        let ret = _helper_test_complete("s_add", 0, 0)
            .unwrap()
            .iter()
            .map(|x| x.label.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            ret,
            vec!["s_add_u32", "s_add_i32", "s_addc_u32", "s_addk_i32"]
        );
    }
}
