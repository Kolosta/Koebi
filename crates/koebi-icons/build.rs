//! Compiles every icon of `icons/` with `koebi-svg` and generates the crate's tables.
//!
//! The icons are checked here, so that a file the renderer would draw differently from its SVG
//! never reaches the application: the build fails on the first file outside the subset, naming
//! its line, and on a file that breaks the conventions of `docs/development/icons.md`:
//!
//! - `icons/<category>/<name>.svg`, one level of folders, names in lowercase kebab-case;
//! - a square view box (an icon fills a square; a drawing against one side keeps the space on
//!   the other).
//!
//! It writes `icons.rs` (a module per category, a constant per icon, the names, roles and
//! sources) and `icons.bin` (the set packed for the GPU, little-endian `f32`) to `OUT_DIR`.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a build script talks to Cargo on stdout and reports its errors on stderr"
)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs, process};

use koebi_svg::Role;

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("set by Cargo"));
    let root = manifest.join("icons");
    // A directory is scanned whole: any file added, changed or removed reruns the script.
    println!("cargo::rerun-if-changed=icons");
    match build(&root) {
        Ok((code, data)) => {
            write(&out.join("icons.rs"), code.as_bytes());
            write(&out.join("icons.bin"), &data);
        }
        Err(message) => {
            eprintln!("error: {message}");
            process::exit(1);
        }
    }
}

fn write(path: &Path, bytes: &[u8]) {
    if let Err(error) = fs::write(path, bytes) {
        eprintln!("error: cannot write {}: {error}", path.display());
        process::exit(1);
    }
}

/// An icon file and what it compiles to.
struct Entry {
    category: String,
    name: String,
    icon: koebi_svg::Icon,
}

fn build(root: &Path) -> Result<(String, Vec<u8>), String> {
    let mut entries = Vec::new();
    for category in sorted(root)? {
        let category_name = file_name(&category);
        if !category.is_dir() {
            return Err(format!(
                "icons/{category_name}: icons go in a category folder, icons/<category>/<name>.svg"
            ));
        }
        check_name(&category_name, &format!("icons/{category_name}"))?;
        for file in sorted(&category)? {
            let file_name = file_name(&file);
            let shown = format!("icons/{category_name}/{file_name}");
            let Some(name) = file_name.strip_suffix(".svg") else {
                return Err(format!("{shown}: only .svg files belong in icons/"));
            };
            if file.is_dir() {
                return Err(format!("{shown}: categories hold icons, not folders"));
            }
            check_name(name, &shown)?;
            let text = fs::read_to_string(&file).map_err(|error| format!("{shown}: {error}"))?;
            let icon = koebi_svg::compile(&text)
                .map_err(|error| format!("{shown}:{}: {}", error.line.max(1), error.message))?;
            let [width, height] = icon.view_size;
            if (width - height).abs() > width.max(height) * 1e-6 {
                return Err(format!(
                    "{shown}: the view box is {width} × {height}: icons are square"
                ));
            }
            entries.push(Entry {
                category: category_name.clone(),
                name: name.to_owned(),
                icon,
            });
        }
    }
    if entries.is_empty() {
        return Err("icons/ holds no icon".to_owned());
    }
    let icons: Vec<&koebi_svg::Icon> = entries.iter().map(|entry| &entry.icon).collect();
    let packed = koebi_svg::pack::pack(&icons);
    let data = packed
        .data
        .iter()
        .flatten()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    Ok((code(&entries), data))
}

/// The entries of `directory`, sorted by name: the icon numbers do not depend on the file
/// system.
fn sorted(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = fs::read_dir(directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{}: {error}", directory.display()))?;
    paths.sort();
    Ok(paths)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Lowercase kebab-case: words of `a-z` and `0-9` joined by single hyphens, starting with a
/// letter.
fn check_name(name: &str, shown: &str) -> Result<(), String> {
    let words_ok = name.split('-').all(|word| {
        !word.is_empty()
            && word
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    });
    let starts_with_letter = name
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_lowercase());
    if words_ok && starts_with_letter {
        Ok(())
    } else {
        Err(format!(
            "{shown}: names are lowercase kebab-case (`chevron-down`), starting with a letter"
        ))
    }
}

/// `chevron-down` as `CHEVRON_DOWN`, `actions` as `actions`.
fn constant(name: &str) -> String {
    name.replace('-', "_").to_ascii_uppercase()
}

fn module(name: &str) -> String {
    name.replace('-', "_")
}

fn role(role: Role) -> &'static str {
    match role {
        Role::Primary => "Role::Primary",
        Role::Secondary => "Role::Secondary",
        Role::Tertiary => "Role::Tertiary",
        Role::Accent => "Role::Accent",
    }
}

fn code(entries: &[Entry]) -> String {
    let mut code = String::from("// Generated by build.rs from icons/: do not edit.\n\n");
    let count = entries.len();
    let _ = writeln!(
        code,
        "/// How many icons the set holds.\npub const COUNT: usize = {count};\n"
    );
    let mut index = 0;
    while index < entries.len() {
        let category = &entries[index].category;
        let _ = writeln!(
            code,
            "/// The icons of `icons/{category}/`.\npub mod {} {{\n    use crate::IconId;\n",
            module(category)
        );
        while index < entries.len() && entries[index].category == *category {
            let entry = &entries[index];
            let _ = writeln!(
                code,
                "    /// `{category}/{}.svg`.\n    pub const {}: IconId = IconId({index});",
                entry.name,
                constant(&entry.name)
            );
            index += 1;
        }
        code.push_str("}\n\n");
    }
    let _ = writeln!(code, "pub(crate) static ALL: [IconId; COUNT] = [");
    for index in 0..entries.len() {
        let _ = writeln!(code, "    IconId({index}),");
    }
    code.push_str(
        "];

",
    );
    let _ = writeln!(code, "pub(crate) static NAMES: [&str; COUNT] = [");
    for entry in entries {
        let _ = writeln!(code, "    \"{}/{}\",", entry.category, entry.name);
    }
    code.push_str("];\n\n");
    let _ = writeln!(code, "pub(crate) static ROLES: [&[Role]; COUNT] = [");
    for entry in entries {
        let roles: Vec<&str> = entry
            .icon
            .zones
            .iter()
            .map(|zone| role(zone.role))
            .collect();
        let _ = writeln!(code, "    &[{}],", roles.join(", "));
    }
    code.push_str("];\n\n");
    let _ = writeln!(code, "pub(crate) static SOURCES: [&str; COUNT] = [");
    for entry in entries {
        let _ = writeln!(
            code,
            "    include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/icons/{}/{}.svg\")),",
            entry.category, entry.name
        );
    }
    code.push_str("];\n");
    code
}
