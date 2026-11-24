//! La app falsa ("candado") que ocupa el nombre de paquete de una app
//! bloqueada. No tiene código ni icono: solo un manifiesto con ese paquete,
//! versionCode enorme y la firma de Cerrojo. Mientras esté instalada, ninguna
//! tienda (ni un APK suelto) puede instalar la app real, porque Android no
//! deja instalar encima un paquete con otra firma.
//!
//! Se genera aquí mismo, sin SDK de Android: se parchea el nombre de paquete
//! en un manifiesto binario de plantilla (hecho una vez con aapt2), se arma el
//! zip y se firma con el esquema v2 (minSdk 24, así que no hace falta v1)
//! con la clave propia de este PC (ver keys.rs).

use anyhow::{Result, bail};
use rsa::pkcs1v15::SigningKey;
use rsa::signature::{SignatureEncoding, Signer};
use sha2::{Digest, Sha256};

const MANIFEST: &[u8] = include_bytes!("../assets/manifest-plantilla.axml");
const ARSC: &[u8] = include_bytes!("../assets/plantilla.arsc");
const TEMPLATE_PKG: &str = "com.cerrojo.plantilla";

// El ayudante del celular: filtro web y vigía (android/, compilado con
// android/build.sh).
const FILTER_MANIFEST: &[u8] = include_bytes!("../assets/filtro/AndroidManifest.xml");
const FILTER_ARSC: &[u8] = include_bytes!("../assets/filtro/resources.arsc");
const FILTER_DEX: &[u8] = include_bytes!("../assets/filtro/classes.dex");
pub const FILTER_PKG: &str = "dev.cerrojo.filtro";
/// El versionCode de android/build.sh.
pub const FILTER_VERSION: u32 = 19;
// FILTER_RES: los recursos sueltos (res/xml/vigia.xml…), ver build.rs.
include!(concat!(env!("OUT_DIR"), "/filtro_res.rs"));

/// Lo que lleva el versionName de nuestros candados (también los hechos a
/// mano con ~/bloqueo-reels, que decían solo "bloqueado").
pub const MARK: &str = "bloqueado";

pub fn build(package: &str) -> Result<Vec<u8>> {
    if !valid_package(package) {
        bail!("nombre de paquete no válido: {package}");
    }
    let manifest = patch_manifest(MANIFEST, TEMPLATE_PKG, package)?;
    let unsigned = zip(&[("AndroidManifest.xml", &manifest), ("resources.arsc", ARSC)]);
    sign_v2(unsigned)
}

/// El ayudante firmado con la clave de este PC.
pub fn build_filter() -> Result<Vec<u8>> {
    let mut files: Vec<(&str, &[u8])> =
        vec![("AndroidManifest.xml", FILTER_MANIFEST), ("classes.dex", FILTER_DEX), ("resources.arsc", FILTER_ARSC)];
    files.extend_from_slice(FILTER_RES);
    sign_v2(zip(&files))
}

pub fn valid_package(p: &str) -> bool {
    let parts: Vec<&str> = p.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|s| {
            let mut c = s.chars();
            matches!(c.next(), Some(ch) if ch.is_ascii_alphabetic())
                && c.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        })
}

// ---- Manifiesto binario (AXML) ----

fn u16_at(b: &[u8], i: usize) -> u16 {
    u16::from_le_bytes([b[i], b[i + 1]])
}
fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}

/// Cambia una cadena del string pool (UTF-16, sin estilos: así lo deja aapt2
/// para esta plantilla) y rehace los tamaños.
fn patch_manifest(axml: &[u8], from: &str, to: &str) -> Result<Vec<u8>> {
    let pool = 8;
    if u16_at(axml, 0) != 3 || u16_at(axml, pool) != 1 {
        bail!("plantilla de manifiesto inesperada");
    }
    let header = u16_at(axml, pool + 2) as usize;
    let size = u32_at(axml, pool + 4) as usize;
    let count = u32_at(axml, pool + 8) as usize;
    let styles = u32_at(axml, pool + 12);
    let flags = u32_at(axml, pool + 16);
    let start = u32_at(axml, pool + 20) as usize;
    if styles != 0 || flags & 0x100 != 0 {
        bail!("plantilla con estilos o UTF-8");
    }
    let mut strings = Vec::with_capacity(count);
    for i in 0..count {
        let off = pool + start + u32_at(axml, pool + header + 4 * i) as usize;
        let len = u16_at(axml, off) as usize;
        let units: Vec<u16> = (0..len).map(|k| u16_at(axml, off + 2 + 2 * k)).collect();
        strings.push(String::from_utf16(&units)?);
    }
    let Some(idx) = strings.iter().position(|s| s == from) else {
        bail!("la plantilla no tiene el paquete");
    };
    strings[idx] = to.to_string();

    let mut data = Vec::new();
    let mut offsets = Vec::new();
    for s in &strings {
        offsets.push(data.len() as u32);
        let units: Vec<u16> = s.encode_utf16().collect();
        data.extend_from_slice(&(units.len() as u16).to_le_bytes());
        for u in units {
            data.extend_from_slice(&u.to_le_bytes());
        }
        data.extend_from_slice(&[0, 0]);
    }
    while data.len() % 4 != 0 {
        data.push(0);
    }
    let new_start = header + 4 * count;
    let new_size = new_start + data.len();
    let mut chunk = axml[pool..pool + header].to_vec();
    chunk[4..8].copy_from_slice(&(new_size as u32).to_le_bytes());
    chunk[20..24].copy_from_slice(&(new_start as u32).to_le_bytes());
    for o in offsets {
        chunk.extend_from_slice(&o.to_le_bytes());
    }
    chunk.extend_from_slice(&data);

    let mut out = axml[..pool].to_vec();
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&axml[pool + size..]);
    let total = out.len() as u32;
    out[4..8].copy_from_slice(&total.to_le_bytes());
    Ok(out)
}

// ---- Zip (sin comprimir, datos alineados a 4) ----

fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32fast::hash(data);
        // Relleno en el campo extra para que los datos queden alineados a 4
        // (Android 11+ lo exige para resources.arsc).
        let pad = (4 - (out.len() + 30 + name.len()) % 4) % 4;
        let mut local = Vec::new();
        local.extend_from_slice(&0x04034b50u32.to_le_bytes());
        local.extend_from_slice(&10u16.to_le_bytes()); // versión
        local.extend_from_slice(&0u16.to_le_bytes()); // flags
        local.extend_from_slice(&0u16.to_le_bytes()); // stored
        local.extend_from_slice(&0u16.to_le_bytes()); // hora
        local.extend_from_slice(&0x0021u16.to_le_bytes()); // fecha 1980-01-01
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&(data.len() as u32).to_le_bytes());
        local.extend_from_slice(&(data.len() as u32).to_le_bytes());
        local.extend_from_slice(&(name.len() as u16).to_le_bytes());
        local.extend_from_slice(&(pad as u16).to_le_bytes());
        local.extend_from_slice(name.as_bytes());
        local.extend(std::iter::repeat_n(0u8, pad));
        out.extend_from_slice(&local);
        out.extend_from_slice(data);

        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        central.extend_from_slice(&10u16.to_le_bytes());
        central.extend_from_slice(&10u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0x0021u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // comentario
        central.extend_from_slice(&0u16.to_le_bytes()); // disco
        central.extend_from_slice(&0u16.to_le_bytes()); // attr internos
        central.extend_from_slice(&0u32.to_le_bytes()); // attr externos
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x06054b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

// ---- Firma APK v2 ----

fn lp(v: &[u8]) -> Vec<u8> {
    let mut o = (v.len() as u32).to_le_bytes().to_vec();
    o.extend_from_slice(v);
    o
}

fn chunked_digest(sections: &[&[u8]]) -> Vec<u8> {
    const CHUNK: usize = 1 << 20;
    let mut digests = Vec::new();
    let mut count = 0u32;
    for s in sections {
        for c in s.chunks(CHUNK) {
            let mut h = Sha256::new();
            h.update([0xa5]);
            h.update((c.len() as u32).to_le_bytes());
            h.update(c);
            digests.extend_from_slice(&h.finalize());
            count += 1;
        }
    }
    let mut h = Sha256::new();
    h.update([0x5a]);
    h.update(count.to_le_bytes());
    h.update(&digests);
    h.finalize().to_vec()
}

fn sign_v2(apk: Vec<u8>) -> Result<Vec<u8>> {
    let eocd_pos = apk.len() - 22;
    let cd_offset = u32_at(&apk, eocd_pos + 16) as usize;
    let contents = &apk[..cd_offset];
    let central = &apk[cd_offset..eocd_pos];
    let eocd = &apk[eocd_pos..];
    // Para el resumen, el EOCD apunta donde empezará el bloque de firma, que
    // es justo donde hoy empieza el directorio central.
    let digest = chunked_digest(&[contents, central, eocd]);

    const RSA_PKCS1_SHA256: u32 = 0x0103;
    let mut d = RSA_PKCS1_SHA256.to_le_bytes().to_vec();
    d.extend_from_slice(&lp(&digest));
    let digests = lp(&lp(&d));
    let keys = crate::keys::load()?;
    let certs = lp(&lp(&keys.cert));
    let attrs = lp(&[]);
    let signed_data = [digests, certs, attrs].concat();

    let sig = SigningKey::<Sha256>::new(keys.key).sign(&signed_data).to_vec();
    let mut s = RSA_PKCS1_SHA256.to_le_bytes().to_vec();
    s.extend_from_slice(&lp(&sig));
    let signatures = lp(&lp(&s));

    let signer = [lp(&signed_data), signatures, lp(&keys.public)].concat();
    let value = lp(&lp(&signer));

    let mut pair = ((value.len() + 4) as u64).to_le_bytes().to_vec();
    pair.extend_from_slice(&0x7109871au32.to_le_bytes());
    pair.extend_from_slice(&value);
    let block_size = (pair.len() + 8 + 16) as u64;
    let mut block = block_size.to_le_bytes().to_vec();
    block.extend_from_slice(&pair);
    block.extend_from_slice(&block_size.to_le_bytes());
    block.extend_from_slice(b"APK Sig Block 42");

    let mut out = contents.to_vec();
    out.extend_from_slice(&block);
    out.extend_from_slice(central);
    let mut eocd = eocd.to_vec();
    let new_cd = (cd_offset + block.len()) as u32;
    eocd[16..20].copy_from_slice(&new_cd.to_le_bytes());
    out.extend_from_slice(&eocd);
    Ok(out)
}
