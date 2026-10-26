//! Minimal readers for Unreal Engine 5 pak (v10/v11) and IoStore (.utoc) directory
//! indexes. We only need the list of contained asset paths for conflict detection.

use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const PAK_MAGIC: u32 = 0x5A6F_12E1;
const NONE: u32 = u32::MAX;

struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cur<'a> {
    fn new(b: &'a [u8]) -> Self {
        Cur { b, p: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.p + n > self.b.len() {
            bail!("Index abgeschnitten");
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into()?))
    }
    fn fstring(&mut self) -> Result<String> {
        let n = self.i32()?;
        if n == 0 {
            return Ok(String::new());
        }
        if n > 0 {
            let s = self.take(n as usize)?;
            Ok(String::from_utf8_lossy(&s[..s.len() - 1]).into_owned())
        } else {
            let chars = (-n) as usize;
            let s = self.take(chars * 2)?;
            let units: Vec<u16> = s[..s.len() - 2]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&units))
        }
    }
}

fn read_at(f: &mut File, offset: u64, len: usize) -> Result<Vec<u8>> {
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

/// Turns `../../../Stalker2/Content/x` into `Stalker2/Content/x`.
fn clean_mount(mount: &str) -> String {
    let mut m = mount.replace('\\', "/");
    while let Some(rest) = m.strip_prefix("../") {
        m = rest.to_string();
    }
    m.trim_start_matches('/').to_string()
}

pub fn list_pak(path: &Path) -> Result<Vec<String>> {
    let mut f = File::open(path).with_context(|| format!("{} öffnen", path.display()))?;
    let size = f.metadata()?.len();
    let tail_len = size.min(512) as usize;
    let tail = read_at(&mut f, size - tail_len as u64, tail_len)?;
    let pos = tail
        .windows(4)
        .rposition(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]) == PAK_MAGIC)
        .context("Keine gültige .pak-Datei")?;
    let mut c = Cur::new(&tail[pos..]);
    c.u32()?;
    let version = c.u32()?;
    let index_offset = c.u64()?;
    let index_size = c.u64()?;
    let encrypted = pos > 0 && tail[pos - 1] == 1;
    if encrypted {
        bail!("Pak-Index ist verschlüsselt");
    }
    if version < 10 {
        return list_pak_legacy(&mut f, version, index_offset, index_size);
    }
    let index = read_at(&mut f, index_offset, index_size as usize)?;
    let mut c = Cur::new(&index);
    let _mount = c.fstring()?;
    let _entries = c.u32()?;
    let _seed = c.u64()?;
    if c.u32()? != 0 {
        c.take(8 + 8 + 20)?;
    }
    if c.u32()? == 0 {
        return Ok(Vec::new());
    }
    let fdi_offset = c.u64()?;
    let fdi_size = c.u64()?;
    let mount = clean_mount(&_mount);
    let fdi = read_at(&mut f, fdi_offset, fdi_size as usize)?;
    let mut c = Cur::new(&fdi);
    let dirs = c.u32()?;
    let mut out = Vec::new();
    for _ in 0..dirs {
        let dir = c.fstring()?;
        let files = c.u32()?;
        for _ in 0..files {
            let name = c.fstring()?;
            c.u32()?;
            let dir = dir.trim_start_matches('/');
            out.push(format!("{mount}{dir}{name}"));
        }
    }
    Ok(out)
}

/// Pak versions before 10 store `FString path + FPakEntry` pairs; we read the paths and
/// skip the entries using the record layout of the respective version.
fn list_pak_legacy(f: &mut File, version: u32, offset: u64, size: u64) -> Result<Vec<String>> {
    let index = read_at(f, offset, size as usize)?;
    let mut c = Cur::new(&index);
    let mount = clean_mount(&c.fstring()?);
    let count = c.u32()?;
    let mut out = Vec::new();
    for _ in 0..count {
        let name = c.fstring()?;
        // offset, size, uncompressed size
        c.take(24)?;
        let compression = c.u32()?;
        if version == 1 {
            c.take(8)?;
        }
        c.take(20)?; // sha1
        if version >= 3 {
            if compression != 0 {
                let blocks = c.u32()?;
                c.take(blocks as usize * 16)?;
            }
            c.take(1 + 4)?; // flags + block size
        }
        out.push(format!("{mount}{name}"));
    }
    Ok(out)
}

pub fn list_utoc(path: &Path) -> Result<Vec<String>> {
    let data = std::fs::read(path).with_context(|| format!("{} lesen", path.display()))?;
    if data.len() < 144 || &data[..16] != b"-==--==--==--==-" {
        bail!("Keine gültige .utoc-Datei");
    }
    let version = data[16];
    let mut c = Cur::new(&data[20..]);
    let header_size = c.u32()? as usize;
    let entry_count = c.u32()? as usize;
    let block_count = c.u32()? as usize;
    let block_entry_size = c.u32()? as usize;
    let method_count = c.u32()? as usize;
    let method_len = c.u32()? as usize;
    let _block_size = c.u32()?;
    let dir_index_size = c.u32()? as usize;
    let flags = data[80];
    let seeds = u32::from_le_bytes(data[84..88].try_into()?) as usize;
    let no_hash = u32::from_le_bytes(data[96..100].try_into()?) as usize;

    const ENCRYPTED: u8 = 2;
    const SIGNED: u8 = 4;
    const INDEXED: u8 = 8;
    if flags & INDEXED == 0 || dir_index_size == 0 {
        return Ok(Vec::new());
    }
    if flags & ENCRYPTED != 0 {
        bail!("IoStore-Container ist verschlüsselt");
    }

    let mut p = header_size + entry_count * 12 + entry_count * 10;
    if version >= 4 {
        p += seeds * 4;
    }
    if version >= 5 {
        p += no_hash * 4;
    }
    p += block_count * block_entry_size + method_count * method_len;
    if flags & SIGNED != 0 {
        let hash_size = i32::from_le_bytes(data[p..p + 4].try_into()?) as usize;
        p += 4 + hash_size * 2 + block_count * 20;
    }
    let end = p + dir_index_size;
    if end > data.len() {
        bail!("IoStore-Verzeichnisindex abgeschnitten");
    }
    let mut c = Cur::new(&data[p..end]);
    let mount = clean_mount(&c.fstring()?);
    let n = c.i32()? as usize;
    let mut dirs = Vec::with_capacity(n);
    for _ in 0..n {
        dirs.push((c.u32()?, c.u32()?, c.u32()?, c.u32()?));
    }
    let n = c.i32()? as usize;
    let mut files = Vec::with_capacity(n);
    for _ in 0..n {
        files.push((c.u32()?, c.u32()?, c.u32()?));
    }
    let n = c.i32()? as usize;
    let mut strings = Vec::with_capacity(n);
    for _ in 0..n {
        strings.push(c.fstring()?);
    }

    let name = |i: u32| strings.get(i as usize).cloned().unwrap_or_default();
    let mut out = Vec::new();
    // (dir index, prefix)
    let mut stack = vec![(0u32, mount)];
    let mut guard = 0usize;
    while let Some((d, prefix)) = stack.pop() {
        guard += 1;
        if guard > dirs.len() + 1 {
            bail!("IoStore-Verzeichnisindex fehlerhaft");
        }
        let Some(&(dname, first_child, _next, first_file)) = dirs.get(d as usize) else {
            continue;
        };
        let path = if dname == NONE { prefix } else { format!("{prefix}{}/", name(dname)) };
        let mut fi = first_file;
        let mut steps = 0;
        while fi != NONE && steps <= files.len() {
            let (fname, next, _) = files[fi as usize];
            out.push(format!("{path}{}", name(fname)));
            fi = next;
            steps += 1;
        }
        let mut ci = first_child;
        let mut steps = 0;
        while ci != NONE && steps <= dirs.len() {
            stack.push((ci, path.clone()));
            ci = dirs[ci as usize].2;
            steps += 1;
        }
    }
    Ok(out)
}

/// Lists the assets in a pak-ish file; `.ucas` and `.sig` carry no index.
pub fn list_assets(path: &Path) -> Result<Vec<String>> {
    match crate::util::ext_lower(&path.to_string_lossy()).as_str() {
        "pak" => list_pak(path),
        "utoc" => list_utoc(path),
        _ => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAKS: &str = "C:/Program Files (x86)/Steam/steamapps/common/S.T.A.L.K.E.R. 2 Heart of Chornobyl/Stalker2/Content/Paks";

    #[test]
    fn reads_game_pak_and_utoc() {
        let dir = Path::new(PAKS);
        if !dir.exists() {
            return;
        }
        let pak = list_pak(&dir.join("pakchunk0-Windows.pak")).unwrap();
        assert!(pak.iter().any(|p| p == "Stalker2/Content/GameLite/GameData/Achievements.cfg"));
        let utoc = list_utoc(&dir.join("pakchunk101-Windows.utoc")).unwrap();
        assert_eq!(utoc.len(), 33);
        assert!(utoc.iter().all(|p| p.starts_with("Stalker2/Content/")));
    }
}
