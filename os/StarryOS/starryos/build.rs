fn main() {
    println!("cargo:rerun-if-changed=ext_linker.ld");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let ext_linker = format!("{out_dir}/ext_linker.ld");

    std::fs::write(&ext_linker, include_str!("ext_linker.ld")).unwrap();
    println!("cargo:rustc-link-arg-bin=starryos=-T{ext_linker}");

    generate_kallsyms_data(&out_dir);
}

fn generate_kallsyms_data(out_dir: &str) {
    let target_dir = std::path::PathBuf::from(out_dir)
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.join("bin/starryos"))
        .unwrap_or_else(|| std::path::PathBuf::from("target/debug/starryos"));

    let kallsyms_rs = std::path::Path::new(out_dir).join("kallsyms_data.rs");

    if !target_dir.exists() {
        emit_empty_kallsyms(&kallsyms_rs);
        return;
    }

    let nm_output = match std::process::Command::new("nm")
        .arg("-n")
        .arg(&target_dir)
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => {
            emit_empty_kallsyms(&kallsyms_rs);
            return;
        }
    };

    let nm_str = String::from_utf8_lossy(&nm_output.stdout);
    let mut filtered = String::new();
    for line in nm_str.lines() {
        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        if parts.len() < 3 {
            continue;
        }
        let sym_type = parts[1].chars().next().unwrap_or('?');
        if matches!(sym_type, 'T' | 't' | 'D' | 'd' | 'B' | 'b' | 'R' | 'r') {
            let name = parts[2];
            if name.starts_with(".L") || name == "$x" || name == "$d" {
                continue;
            }
            filtered.push_str(line);
            filtered.push('\n');
        }
    }

    let escaped = escaped_rust_string(&filtered);
    let code = format!(
        "/// Auto-generated kernel symbol table.\n#[allow(dead_code)]\npub const KALLSYMS_DATA: \
         &str = {escaped};\n"
    );
    std::fs::write(&kallsyms_rs, code).unwrap();
}

fn emit_empty_kallsyms(path: &std::path::Path) {
    let code = "/// No kernel symbol data available (first build or binary not \
                found).\n#[allow(dead_code)]\npub const KALLSYMS_DATA: &str = \"\";\n";
    std::fs::write(path, code).unwrap();
}

fn escaped_rust_string(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() + 2);
    escaped.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                write!(escaped, "\\u{{{:04x}}}", c as u32).unwrap();
            }
            c => escaped.push(c),
        }
    }
    escaped.push('"');
    escaped
}
