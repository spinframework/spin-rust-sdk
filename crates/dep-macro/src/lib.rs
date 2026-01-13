use anyhow::Context;
use std::path::{Path, PathBuf};

use wit_component::DecodedWasm;

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn imports_for(input: TokenStream) -> TokenStream {
    let comp_name = syn::parse_macro_input!(input as syn::LitStr).value();

    let tokstm = match imports_for_impl(comp_name) {
        Ok(tokstm) => tokstm,
        Err(msg) => {
            let err = syn::LitStr::new(&msg.to_string(), proc_macro2::Span::call_site());
            quote! { compile_error!(#err); }
        }
    };

    TokenStream::from(tokstm)
}

fn imports_for_impl(comp_name: String) -> anyhow::Result<proc_macro2::TokenStream> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").context("OUT_DIR not set")?);

    let manifest_path = find_spin_toml().context("spin.toml not found")?;
    let manifest_dir = manifest_path.parent().unwrap();
    let s = std::fs::read_to_string(&manifest_path).context("spin.toml is not valid text")?;
    let t: toml::Table = toml::from_str(&s).context("spin.toml is not a valid TOML table")?;
    let c = t.get("component").context("spin.toml does not contain any components")?
        .get(&comp_name).with_context(|| format!("spin.toml does not contain a component named '{comp_name}'"))?;

    let mut tokstm = proc_macro2::TokenStream::new();

    if let Some(deps) = c.get("dependencies").and_then(|d| d.as_table()) {
        for (depname, dep) in deps {
            let Some(deppath) = dep.get("path") else {
                anyhow::bail!("the macro supports only file deps at the moment: {depname} is not a path");
            };
            let Some(deppath) = deppath.as_str() else {
                anyhow::bail!("dep paths must be strings: {depname}'s path is not a string");
            };
            let deppath = manifest_dir.join(deppath);

            let enc_wit_path = out_dir.join(format!("{}.wasm", safeify(depname)));

            let mut wasm = read_wasm(&deppath).with_context(|| format!("failed to read Wasm from {}", deppath.display()))?;
            importize(&mut wasm, None, None).with_context(|| format!("failed to importize Wasm from {}", deppath.display()))?;
            emit_wasm(&wasm, &enc_wit_path).with_context(|| format!("failed to save importized Wasm to {}", enc_wit_path.display()))?;

            let itfs = extract_imports(wasm);

            if itfs.is_empty() {
                continue;
            }

            let ns = &itfs[0].0.namespace;

            let mack = dep_module(&enc_wit_path, &itfs, ns);
            tokstm.extend([mack]);
        }
    }

    Ok(tokstm)
}

fn find_spin_toml() -> Option<PathBuf> {
    fn is_git_root(dir: &Path) -> bool {
        dir.join(".git").is_dir()
    }
    
    let mut current_dir = std::env::current_dir().ok()?;

    loop {
        let candidate = current_dir.join("spin.toml");
        if candidate.exists() {
            return Some(candidate);
        }

        if is_git_root(&current_dir) {
            return None;
        }

        let Some(parent) = current_dir.parent() else {
            return None;
        };

        current_dir = parent.to_owned();
    }
}

fn extract_imports(wasm: DecodedWasm) -> Vec<(wit_parser::PackageName, String)> {
    let mut itfs = vec![];

    for (_pid, pp) in &wasm.resolve().packages {
        for (_w, wid) in &pp.worlds {
            if let Some(world) = wasm.resolve().worlds.get(*wid) {
                for (_wk, witem) in &world.imports {
                    if let wit_parser::WorldItem::Interface { id, .. } = witem {
                        if let Some(itf) = wasm.resolve().interfaces.get(*id) {
                            if let Some(itfp) = itf.package.as_ref() {
                                if let Some(ppp) = wasm.resolve().packages.get(*itfp) {
                                    if let Some(itfname) = itf.name.as_ref() {
                                        itfs.push((ppp.name.clone(), itfname.clone()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    itfs
}

fn read_wasm(path: impl AsRef<Path>) -> anyhow::Result<DecodedWasm> {
    if !path.as_ref().exists() {
        println!("HELP IT DOESN'T EXIST");
    }
    let wasm_bytes = std::fs::read(path.as_ref());
    if wasm_bytes.is_err() {
        println!("{wasm_bytes:?}");
    }
    let wasm_bytes = wasm_bytes?;

    if wasmparser::Parser::is_component(&wasm_bytes) {
        wit_component::decode(&wasm_bytes)
    } else {
        let (wasm, bindgen) = wit_component::metadata::decode(&wasm_bytes)?;
        if wasm.is_none() {
            anyhow::bail!(
                "input is a core wasm module with no `component-type*` \
                    custom sections meaning that there is not WIT information; \
                    is the information not embedded or is this supposed \
                    to be a component?"
            )
        }
        Ok(DecodedWasm::Component(bindgen.resolve, bindgen.world))
    }
}

fn importize(
    decoded: &mut DecodedWasm,
    world: Option<&str>,
    out_world_name: Option<&String>,
) -> anyhow::Result<()> {
    let (resolve, world_id) = match (&mut *decoded, world) {
        (DecodedWasm::Component(resolve, world), None) => (resolve, *world),
        (DecodedWasm::Component(..), Some(_)) => {
            anyhow::bail!(
                "the `--importize-world` flag is not compatible with a \
                    component input, use `--importize` instead"
            );
        }
        (DecodedWasm::WitPackage(resolve, id), world) => {
            let world = resolve.select_world(&[*id], world)?;
            (resolve, world)
        }
    };
    resolve
        .importize(world_id, out_world_name.cloned())
        .context("failed to move world exports to imports")?;
    let resolve = std::mem::take(resolve);
    *decoded = DecodedWasm::Component(resolve, world_id);
    Ok(())
}

fn emit_wasm(decoded: &DecodedWasm, dest: impl AsRef<Path>) -> anyhow::Result<()> {
    let decoded_package = decoded.package();
    let bytes = wit_component::encode(decoded.resolve(), decoded_package)?;
    std::fs::write(dest.as_ref(), bytes)?;
    Ok(())
}

fn dep_module(wasm_path: impl AsRef<Path>, itfs: &[(wit_parser::PackageName, String)], ns: &str) -> proc_macro2::TokenStream {
    let import_clauses = itfs.iter().map(|(p, i)| format!("import {};", qname(p, i))).collect::<Vec<_>>();
    let joined_import_clauses = import_clauses.join("\n");

    let wasm_path_str = wasm_path.as_ref().display().to_string();

    let mod_name = quote::format_ident!("{}_dep", identifierify(ns));

    let inline = format!(r#"
        package test:test-{ns};
        world spork {{
            {joined_import_clauses}
        }}
    "#);

    let inline_lit = syn::LitStr::new(&inline, proc_macro2::Span::call_site());

    let world = format!("test:test-{ns}/spork");
    let world_lit = syn::LitStr::new(&world, proc_macro2::Span::call_site());

    quote! {
        mod #mod_name {
            spin_sdk::wit_bindgen::generate!({
                inline: #inline_lit,
                path: #wasm_path_str,
                world: #world_lit,
                runtime_path: "::spin_sdk::wit_bindgen::rt",
                generate_all
            });
        }
    }
}

fn qname(p: &wit_parser::PackageName, i: &str) -> String {
    match &p.version {
        Some(v) => format!("{}:{}/{}@{}", p.namespace, p.name, i, v),
        None => format!("{}:{}/{}", p.namespace, p.name, i),
    }
}

fn safeify(depname: &str) -> String {
    depname.replace("/", "_").replace("@", "_").replace(":", "_").replace(".", "_")
}

fn identifierify(depname: &str) -> String {
    safeify(depname).replace("-", "_")
}
