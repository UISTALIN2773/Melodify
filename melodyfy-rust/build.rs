use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/logo.ico");
    println!("cargo:rerun-if-changed=build.rs");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let ico_path = PathBuf::from("assets/logo.ico");
    let ico_bytes = match fs::read(&ico_path) {
        Ok(b) => b,
        Err(_) => return,
    };

    if ico_bytes.len() < 6 {
        return;
    }

    let count = u16::from_le_bytes([ico_bytes[4], ico_bytes[5]]) as usize;
    if count == 0 || ico_bytes.len() < 6 + count * 16 {
        return;
    }

    let mut res = Vec::new();
    // Standard 32-byte empty Win32 .res header
    res.extend_from_slice(&[
        0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ]);

    let mut group_data = Vec::new();
    group_data.extend_from_slice(&0u16.to_le_bytes()); // idReserved
    group_data.extend_from_slice(&1u16.to_le_bytes()); // idType = 1 (ICON)
    group_data.extend_from_slice(&(count as u16).to_le_bytes());

    for i in 0..count {
        let dir_off = 6 + i * 16;
        let width = ico_bytes[dir_off];
        let height = ico_bytes[dir_off + 1];
        let color_count = ico_bytes[dir_off + 2];
        let reserved = ico_bytes[dir_off + 3];
        let planes = u16::from_le_bytes([ico_bytes[dir_off + 4], ico_bytes[dir_off + 5]]);
        let bit_count = u16::from_le_bytes([ico_bytes[dir_off + 6], ico_bytes[dir_off + 7]]);
        let bytes_in_res = u32::from_le_bytes([
            ico_bytes[dir_off + 8],
            ico_bytes[dir_off + 9],
            ico_bytes[dir_off + 10],
            ico_bytes[dir_off + 11],
        ]);
        let img_offset = u32::from_le_bytes([
            ico_bytes[dir_off + 12],
            ico_bytes[dir_off + 13],
            ico_bytes[dir_off + 14],
            ico_bytes[dir_off + 15],
        ]) as usize;

        let end = img_offset.saturating_add(bytes_in_res as usize);
        if end > ico_bytes.len() {
            return;
        }

        let icon_id = (i + 1) as u16;
        append_res_record(&mut res, 3, icon_id, &ico_bytes[img_offset..end]);

        // GRPICONDIRENTRY (14 bytes)
        group_data.push(width);
        group_data.push(height);
        group_data.push(color_count);
        group_data.push(reserved);
        group_data.extend_from_slice(&planes.to_le_bytes());
        group_data.extend_from_slice(&bit_count.to_le_bytes());
        group_data.extend_from_slice(&bytes_in_res.to_le_bytes());
        group_data.extend_from_slice(&icon_id.to_le_bytes());
    }

    // RT_GROUP_ICON = 14, Name = 1
    append_res_record(&mut res, 14, 1, &group_data);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let res_path = out_dir.join("melodyfy_icon.res");
    if fs::write(&res_path, &res).is_ok() {
        println!("cargo:rustc-link-arg-bins={}", res_path.display());
    }
}

fn append_res_record(out: &mut Vec<u8>, res_type: u16, res_name: u16, data: &[u8]) {
    let data_size = data.len() as u32;
    let header_size: u32 = 32;

    out.extend_from_slice(&data_size.to_le_bytes());
    out.extend_from_slice(&header_size.to_le_bytes());
    // Type (ordinal)
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_type.to_le_bytes());
    // Name (ordinal)
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&res_name.to_le_bytes());
    // DataVersion, MemoryFlags, LanguageId, Version, Characteristics
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0x1030u16.to_le_bytes()); // MOVEABLE | PURE | DISCARDABLE
    out.extend_from_slice(&0x0409u16.to_le_bytes()); // EN-US
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());

    out.extend_from_slice(data);
    while out.len() % 4 != 0 {
        out.push(0);
    }
}
