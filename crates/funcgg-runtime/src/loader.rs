use std::borrow::Cow;
use std::time::Duration;

use deno_ast::{MediaType, ParseParams};
use deno_core::{
    ModuleCodeString, ModuleLoadResponse, ModuleName, ModuleSource, ModuleSourceCode,
    ModuleSpecifier, ModuleType, SourceMapData, error::ModuleLoaderError, futures::FutureExt,
};
use deno_error::JsErrorBox;

pub struct ModuleLoader {
    http_client: reqwest::Client,
}

impl ModuleLoader {
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .user_agent("func.gg/module_loader")
            .build()
            .expect("Unable to build HTTP client");

        Self { http_client }
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
        specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
    ) -> ModuleLoadResponse {
        if is_dyn_import {
            return ModuleLoadResponse::Sync(Err(ModuleLoaderError::generic(
                "dynamic module loading is not supported",
            )));
        }

        if specifier.scheme() != "https" {
            return ModuleLoadResponse::Sync(Err(ModuleLoaderError::generic(
                "only modules with an 'https' scheme are supported",
            )));
        }

        let specifier = specifier.clone();
        let http_client = self.http_client.clone();
        ModuleLoadResponse::Async(
            async move {
                let res = http_client
                    .get(specifier.clone())
                    .query(&[("target", "deno")]) // tells services to explicitly redirect
                    .send()
                    .await
                    .map_err(|err| ModuleLoaderError::generic(err.to_string()))?;

                let original_specifier = {
                    let mut url = specifier.clone();
                    url.set_query(None);
                    url
                };

                let found_specifier = {
                    let mut url = res.url().clone();
                    url.set_query(None);
                    url
                };

                let content_type = res
                    .headers()
                    .get("content-type")
                    .map(|ct| ct.to_str().unwrap_or_default())
                    .unwrap_or_default()
                    .split(';')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_lowercase();
                let module_type = match content_type.as_ref() {
                    "application/javascript"
                    | "text/javascript"
                    | "application/ecmascript"
                    | "text/ecmascript" => ModuleType::JavaScript,
                    "application/wasm" => ModuleType::Wasm,
                    "application/json" | "text/json" => ModuleType::Json,
                    "text/plain" | "application/octet-stream" => ModuleType::Text,
                    s => ModuleType::Other(Cow::Owned(s.to_string())),
                };

                if !res.status().is_success() {
                    return Err(ModuleLoaderError::generic("failed to load module"));
                }

                // TODO: probably use bytes
                let src_text = res
                    .text()
                    .await
                    .map_err(|err| ModuleLoaderError::generic(err.to_string()))?;

                Ok(ModuleSource::new_with_redirect(
                    module_type,
                    ModuleSourceCode::String(src_text.into()),
                    &original_specifier,
                    &found_specifier,
                    None,
                ))
            }
            .boxed_local(),
        )
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
