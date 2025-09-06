use deno_ast::{MediaType, ParseParams};
use deno_core::{
    ModuleCodeString, ModuleLoadResponse, ModuleName, ModuleSpecifier, SourceMapData,
    error::ModuleLoaderError,
};
use deno_error::JsErrorBox;

pub struct ModuleLoader;

impl ModuleLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl deno_core::ModuleLoader for ModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Result<ModuleSpecifier, deno_core::error::ModuleLoaderError> {
        // https://html.spec.whatwg.org/multipage/webappapis.html#resolve-a-module-specifier
        deno_core::resolve_import(specifier, referrer).map_err(JsErrorBox::from_err)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dynamic: bool,
        _requested_module_type: deno_core::RequestedModuleType,
    ) -> deno_core::ModuleLoadResponse {
        log::error!("attempting to load module: {}", module_specifier);
        ModuleLoadResponse::Sync(Err(ModuleLoaderError::generic(
            "module loading is not supported",
        )))
    }
}

pub fn transpile(
    mod_name: ModuleName,
    code: ModuleCodeString,
) -> Result<(ModuleCodeString, Option<SourceMapData>), JsErrorBox> {
    let media_type = MediaType::from_filename(&mod_name);

    // alt: see #is_jsx and #is_typed
    let needs_transpilation = match media_type {
        MediaType::JavaScript => false,
        MediaType::Jsx => true,
        MediaType::TypeScript => true,
        MediaType::Tsx => true,
        _ => {
            return Result::Err(JsErrorBox::generic(format!(
                "media type '{}' not supported",
                media_type
            )));
        }
    };

    if !needs_transpilation {
        return Ok((code, None));
    }

    let specifier = ModuleSpecifier::parse(&mod_name).map_err(JsErrorBox::from_err)?;

    let parsed = deno_ast::parse_module(ParseParams {
        specifier,
        text: code.into(),
        media_type,
        capture_tokens: false,
        scope_analysis: false,
        maybe_syntax: None,
    })
    .map_err(JsErrorBox::from_err)?;

    let result = parsed
        .transpile(
            &Default::default(),
            &Default::default(),
            &Default::default(),
        )
        .map_err(JsErrorBox::from_err)?
        .into_source();

    Ok((
        result.text.into(),
        result.source_map.map(|srcmap| srcmap.into_bytes().into()),
    ))
}
