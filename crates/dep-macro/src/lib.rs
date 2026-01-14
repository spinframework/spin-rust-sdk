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

            let mut wasm = read_wasm(&deppath).with_context(|| format!("failed to read Wasm from {}", deppath.display()))?;
            importize(&mut wasm, None, None).with_context(|| format!("failed to importize Wasm from {}", deppath.display()))?;

            let itfs = extract_imports(&wasm);

            if itfs.is_empty() {
                continue;
            }

            let ns = &itfs[0].0.namespace;
            
            let resolve = wasm.resolve().clone();

            let mut opts = wit_bindgen_rust::Opts::default();
            opts.generate_all = true;
            opts.runtime_path = Some("::spin_sdk::wit_bindgen::rt".to_string());

            let ts = wit_bindgen_macro_expand(opts, ns, resolve, &itfs)?;
            let ts = proc_macro2::TokenStream::from(ts);
            
            let mod_name = quote::format_ident!("{}_dep", identifierify(ns));

            let ts2 = quote! {
                mod #mod_name {
                    #ts
                }
            };
            tokstm.extend([ts2]);
        }
    }

    Ok(tokstm)
}

use wit_parser::Resolve;
use wit_parser::PackageId;

// Adapted from wit-bindgen macro source
fn wit_bindgen_macro_parse_source(
    mut resolve: Resolve,
    source: &str,
) -> anyhow::Result<(Resolve, Vec<PackageId>, Vec<PathBuf>)> {
    let mut pkgs = Vec::new();
    pkgs.push(resolve.push_group(wit_parser::UnresolvedPackageGroup::parse("macro-input", source)?)?);
    Ok((resolve, pkgs, vec![]))
}


// Adapted from wit-bindgen macro source
fn wit_bindgen_macro_expand(opts: wit_bindgen_rust::Opts, ns: &str, resolve: Resolve, itfs: &[(wit_parser::PackageName, String)]) -> anyhow::Result<TokenStream> {
    use wit_bindgen_core::WorldGenerator;

    let import_clauses = itfs.iter().map(|(p, i)| format!("import {};", qname(p, i))).collect::<Vec<_>>();
    let joined_import_clauses = import_clauses.join("\n");
    let inline = format!(r#"
        package test:test-{ns};
        world spork {{
            {joined_import_clauses}
        }}
    "#);

    let (resolve, pkgs, self_files) = wit_bindgen_macro_parse_source(resolve, &inline)?;

    let world_pre = format!("test:test-{ns}/spork");

    let world = resolve
        .select_world(&pkgs, Some(world_pre.as_str()))?;

    let mut files = Default::default();
    let mut generator = opts.build();
    generator
        .generate(&resolve, world, &mut files)?;
    let (_, src) = files.iter().next().unwrap();
    let src = std::str::from_utf8(src).unwrap().to_string();

    let mut contents = src.parse::<TokenStream>().unwrap();

    // Include a dummy `include_bytes!` for any files we read so rustc knows that
    // we depend on the contents of those files.
    for file in self_files.iter() {
        contents.extend(
            format!(
                "const _: &[u8] = include_bytes!(r#\"{}\"#);\n",
                file.display()
            )
            .parse::<TokenStream>()
            .unwrap(),
        );
    }

    Ok(contents)
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

fn extract_imports(wasm: &DecodedWasm) -> Vec<(wit_parser::PackageName, String)> {
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
