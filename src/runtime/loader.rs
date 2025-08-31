use deno_ast::{MediaType, ParseParams};
use deno_core::{ModuleCodeString, ModuleName, ModuleSpecifier, SourceMapData};
use deno_error::JsErrorBox;

pub fn transpile(
    mod_name: ModuleName,
    code: ModuleCodeString,
) -> Result<(ModuleCodeString, Option<SourceMapData>), JsErrorBox> {
    // TODO(robherley): naive, clean up later
    let media_type = match &mod_name {
        m if m.ends_with(".ts") => MediaType::TypeScript,
        m if m.ends_with(".js") => MediaType::JavaScript,
        m => panic!("unknown media type for module: {}", m),
    };

    if media_type == MediaType::JavaScript {
        return Ok((code, None));
    }

    let specifier = ModuleSpecifier::parse(&mod_name)
        .map_err(|e| deno_error::JsErrorBox::generic(format!("bad mod specifier: {}", e)))?;

    let parsed = deno_ast::parse_module(ParseParams {
        specifier,
        text: code.into(),
        media_type,
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
    })
    .map_err(|e| deno_error::JsErrorBox::generic(format!("parse error: {}", e)))?;

    let result = parsed
        .transpile(
            &Default::default(),
            &Default::default(),
            &Default::default(),
        )
        .map_err(|e| deno_error::JsErrorBox::generic(format!("Transpile error: {}", e)))?
        .into_source();

    let src_map = result.source_map.map(|map| map.into_bytes().into());

    Ok((result.text.into(), src_map))
}
