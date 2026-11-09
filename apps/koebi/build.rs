//! Embeds the application's icon in the Windows executable: Explorer, shortcuts and pinned task
//! bar buttons read it from there (running windows get theirs from `src/app_icon.rs`).
//!
//! `platform/windows/koebi.ico` (drawn from `crates/koebi-icons/icons/app/icon.svg` and kept up
//! to date by a test of `src/app_icon.rs`) becomes a compiled resource file, the format `rc.exe`
//! writes, which the MSVC linker takes as an input. Other targets need nothing here.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a build script talks to Cargo on stdout and reports its errors on stderr"
)]

use std::path::PathBuf;
use std::{env, fs, process};

/// Resource types.
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
/// Movable, discardable (and pure, for the group): what `rc.exe` gives icons.
const ICON_FLAGS: u16 = 0x1010;
const GROUP_FLAGS: u16 = 0x1030;
/// English (United States), `rc.exe`'s default.
const LANGUAGE: u16 = 0x0409;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=../../platform/windows/koebi.ico");
    let windows = env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows");
    let msvc = env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|target_env| target_env == "msvc");
    if !windows || !msvc {
        return;
    }
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by Cargo"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("set by Cargo"));
    let ico = manifest.join("../../platform/windows/koebi.ico");
    let resources = fs::read(&ico)
        .map_err(|error| format!("{}: {error}", ico.display()))
        .and_then(|bytes| compile(&bytes));
    match resources {
        Ok(res) => {
            let path = out.join("koebi.res");
            if let Err(error) = fs::write(&path, res) {
                eprintln!("error: cannot write {}: {error}", path.display());
                process::exit(1);
            }
            println!("cargo::rustc-link-arg-bins={}", path.display());
        }
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(1);
        }
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, String> {
    bytes
        .get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| "koebi.ico is truncated".to_owned())
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| "koebi.ico is truncated".to_owned())
}

/// One resource: its header (numeric type and name) and its data, padded to four bytes.
fn resource(
    out: &mut Vec<u8>,
    kind: u16,
    name: u16,
    flags: u16,
    data: &[u8],
) -> Result<(), String> {
    let size = u32::try_from(data.len()).map_err(|_| "an icon is too large".to_owned())?;
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&32_u32.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(&[0xFF, 0xFF]);
    out.extend_from_slice(&name.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes()); // data version
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&LANGUAGE.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes()); // version
    out.extend_from_slice(&0_u32.to_le_bytes()); // characteristics
    out.extend_from_slice(data);
    out.resize(out.len().next_multiple_of(4), 0);
    Ok(())
}

/// The resource file of the icons of an ICO file: one icon resource per image, and the group
/// that lists them, the executable's icon.
fn compile(ico: &[u8]) -> Result<Vec<u8>, String> {
    if u16_at(ico, 0)? != 0 || u16_at(ico, 2)? != 1 {
        return Err("koebi.ico is not an icon file".to_owned());
    }
    let count = u16_at(ico, 4)?;
    // A resource file starts with an empty resource.
    let mut res = vec![0_u8; 32];
    res[4] = 32;
    res[8..10].copy_from_slice(&[0xFF, 0xFF]);
    res[12..14].copy_from_slice(&[0xFF, 0xFF]);
    let mut group = Vec::new();
    group.extend_from_slice(&[0, 0, 1, 0]);
    group.extend_from_slice(&count.to_le_bytes());
    for index in 0..count {
        let entry = 6 + 16 * usize::from(index);
        let header = ico
            .get(entry..entry + 8)
            .ok_or_else(|| "koebi.ico is truncated".to_owned())?;
        let size = u32_at(ico, entry + 8)?;
        let offset = u32_at(ico, entry + 12)? as usize;
        let image = ico
            .get(offset..offset + size as usize)
            .ok_or_else(|| "koebi.ico is truncated".to_owned())?;
        let id = index + 1;
        resource(&mut res, RT_ICON, id, ICON_FLAGS, image)?;
        // The group's entry: the ICO entry with the resource's number for its offset.
        group.extend_from_slice(header);
        group.extend_from_slice(&size.to_le_bytes());
        group.extend_from_slice(&id.to_le_bytes());
    }
    resource(&mut res, RT_GROUP_ICON, 1, GROUP_FLAGS, &group)?;
    Ok(res)
}
