//! Small made-up files the fake Jira serves for the demo attachments, so Quick Look and inline images have
//! something to show.

/// The bytes of a demo attachment by its MIME type: a small picture, a one-page PDF or a zip with one note.
pub fn demo_file(mime_type: &str) -> Vec<u8> {
    match mime_type {
        "image/png" => png(),
        "application/pdf" => pdf("Demo release notes"),
        "application/zip" => zip("notes.txt", b"Made-up log lines for the demo.\n"),
        _ => b"Demo file\n".to_vec(),
    }
}

pub const PNG_WIDTH: u32 = 240;
pub const PNG_HEIGHT: u32 = 120;

/// The color of one pixel of the demo picture: a white card on a pale blue ground, with a blue dot and two
/// grey lines of "text", like a small screen mockup.
fn pixel(x: u32, y: u32) -> [u8; 3] {
    let in_card = (20..220).contains(&x) && (20..100).contains(&y);
    let (dx, dy) = (x.abs_diff(60), y.abs_diff(60));
    if dx * dx + dy * dy <= 22 * 22 {
        [64, 120, 230]
    } else if (100..200).contains(&x) && (44..54).contains(&y)
        || (100..170).contains(&x) && (64..72).contains(&y)
    {
        [180, 195, 225]
    } else if in_card {
        [255, 255, 255]
    } else {
        [220, 230, 250]
    }
}

/// The demo picture as a `PNG_WIDTH`×`PNG_HEIGHT` RGB PNG, its pixel data in uncompressed zlib blocks.
fn png() -> Vec<u8> {
    let mut raw = Vec::new();
    for y in 0..PNG_HEIGHT {
        raw.push(0); // no filter
        for x in 0..PNG_WIDTH {
            raw.extend_from_slice(&pixel(x, y));
        }
    }
    let mut zlib = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = raw.chunks(u16::MAX as usize).collect();
    for (index, block) in blocks.iter().enumerate() {
        zlib.push(u8::from(index + 1 == blocks.len()));
        zlib.extend_from_slice(&(block.len() as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());
    let mut header = Vec::new();
    header.extend_from_slice(&PNG_WIDTH.to_be_bytes());
    header.extend_from_slice(&PNG_HEIGHT.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit RGB, no interlace
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    for (kind, data) in [(b"IHDR", header), (b"IDAT", zlib), (b"IEND", Vec::new())] {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut chunk = kind.to_vec();
        chunk.extend_from_slice(&data);
        out.extend_from_slice(&chunk);
        out.extend_from_slice(&crc32(&chunk).to_be_bytes());
    }
    out
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

/// A one-page PDF with `text`, its cross-reference table at the right offsets.
fn pdf(text: &str) -> Vec<u8> {
    let content = format!("BT /F1 24 Tf 72 720 Td ({text}) Tj ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_string(),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
    }
    let xref = out.len();
    out.push_str(&format!(
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    ));
    for offset in offsets {
        out.push_str(&format!("{offset:010} 00000 n \n"));
    }
    out.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    ));
    out.into_bytes()
}

/// A zip archive holding one stored (uncompressed) file.
fn zip(name: &str, data: &[u8]) -> Vec<u8> {
    let crc = crc32(data);
    let size = data.len() as u32;
    let name_len = name.len() as u16;
    let mut out = Vec::new();
    let entry = |out: &mut Vec<u8>, central: bool| {
        out.extend_from_slice(if central {
            b"PK\x01\x02"
        } else {
            b"PK\x03\x04"
        });
        if central {
            out.extend_from_slice(&20u16.to_le_bytes());
        }
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // stored
        out.extend_from_slice(&0u32.to_le_bytes()); // time and date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra length
        if central {
            out.extend_from_slice(&[0; 10]); // comment length, disk, attributes
            out.extend_from_slice(&0u32.to_le_bytes()); // local header offset
        }
        out.extend_from_slice(name.as_bytes());
    };
    entry(&mut out, false);
    out.extend_from_slice(data);
    let central_at = out.len() as u32;
    entry(&mut out, true);
    let central_size = out.len() as u32 - central_at;
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&[0; 4]); // disk numbers
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&central_size.to_le_bytes());
    out.extend_from_slice(&central_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
