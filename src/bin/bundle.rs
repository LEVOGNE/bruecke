//! bruecke bundler — cargo run --bin bundle
//! Input:  pkg/bruecke_bg.wasm + pkg/bruecke.js  (wasm-pack output)
//!         src/engine.js + src/shell.html          (baked in at compile time)
//! Output: dist/index.html  (self-contained, WASM embedded as base64)
//!
//! Architecture note: bruecke embeds a RustPython VM inside WASM.
//! The user writes app.py; the dev server watches it and streams source
//! to the browser via SSE.  engine.js calls the WASM export `on_run(src)`
//! to compile the Python, then calls `frame(t)` each animation tick.
//! No Rhai scripting — Python is the sole user-facing language.

use std::fs;
use std::io::{BufWriter, Write};

const SHELL:  &str = include_str!("../shell.html");
const ENGINE: &str = include_str!("../engine.js");

fn main() {
    let root = std::env::current_dir().expect("cwd");
    let pkg  = root.join("pkg");
    let dist = root.join("dist");

    let wasm_path = pkg.join("bruecke_bg.wasm");
    let glue_path = pkg.join("bruecke.js");
    for p in [&wasm_path, &glue_path] {
        if !p.exists() {
            eprintln!("ERROR: missing {}", p.display());
            eprintln!("  → run: wasm-pack build --target web --release");
            std::process::exit(1);
        }
    }

    // 1. WASM → base64
    let wasm_bytes = fs::read(&wasm_path).expect("read wasm");
    let wasm_b64   = b64(&wasm_bytes);
    println!("  wasm  : {} bytes → {} chars", wasm_bytes.len(), wasm_b64.len());

    // 2. wasm-bindgen glue: strip ES-module syntax → global script
    let mut glue = fs::read_to_string(&glue_path).expect("read glue");
    if let Some(s) = glue.find("/* @ts-self-types") {
        if let Some(r) = glue[s..].find("*/") {
            glue = format!("{}{}", &glue[..s], glue[s+r+2..].trim_start_matches('\n'));
        }
    }
    glue = glue.replace("export function ", "function ");
    glue = glue.lines()
        .filter(|l| !l.trim_start().starts_with("export {"))
        .collect::<Vec<_>>().join("\n");
    glue = glue.replace(
        "module_or_path = new URL('bruecke_bg.wasm', import.meta.url);",
        "throw new Error('standalone: pass WASM bytes to init()');",
    );
    glue = glue.replace("async function __wbg_init(", "async function init(");
    println!("  glue  : {} chars", glue.len());

    // 3. engine.js: strip trailing main() call
    let mut engine = ENGINE.to_string();
    let t = engine.trim_end();
    if let Some(pos) = t.rfind("\nmain()") {
        engine = t[..pos].trim_end().to_string();
    } else if t.ends_with("main();") {
        engine = t[..t.len()-7].trim_end().to_string();
    }
    println!("  engine: {} chars", engine.len());

    // 4. CSS + body from shell.html
    let css  = between(SHELL, "<style>", "</style>").unwrap_or("");
    let body = between(SHELL, "<body>", "<script").unwrap_or("").trim();

    // 5. write dist/index.html
    fs::create_dir_all(&dist).expect("create dist/");
    let out  = dist.join("index.html");
    let mut w = BufWriter::new(fs::File::create(&out).expect("create output"));
    macro_rules! w { ($($x:tt)*) => { write!(w, $($x)*).unwrap() } }

    w!("<!DOCTYPE html><html lang=\"en\"><head>\n");
    w!("<meta charset=\"UTF-8\">");
    w!("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1.0\">\n");
    w!("<title>bruecke</title><style>{}</style></head><body>\n", css);
    w!("{}\n<script>\n", body);
    w!("const __WASM_B64='{}';\n", wasm_b64);
    w!("function js_now(){{return performance.now();}}\n");
    w!("{}", glue);
    w!("{}", engine);
    w!("\n(async()=>{{const b=Uint8Array.from(atob(__WASM_B64),c=>c.charCodeAt(0));");
    w!("try{{await init({{module_or_path:b}});await main()}}catch(e){{fatal(e)}}}})();\n");
    w!("</script></body></html>\n");
    w.flush().unwrap();

    let kb = fs::metadata(&out).expect("stat").len() as f64 / 1024.0;
    println!("  out   : {} ({:.1} KB)", out.display(), kb);
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let i = s.find(open)? + open.len();
    Some(&s[i..i + s[i..].find(close)?])
}

fn b64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity((data.len() + 2) / 3 * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16
              | (*c.get(1).unwrap_or(&0) as u32) << 8
              | (*c.get(2).unwrap_or(&0) as u32);
        o.push(T[((n>>18)&63) as usize] as char);
        o.push(T[((n>>12)&63) as usize] as char);
        o.push(if c.len()>1 { T[((n>>6)&63) as usize] as char } else { '=' });
        o.push(if c.len()>2 { T[(n&63)    as usize] as char } else { '=' });
    }
    o
}
