//! Minimal pure-Rust Win32 `.res` writer used by build scripts (S-SUP-05: no downloads, no
//! external tools). Writes RT_MANIFEST, RT_ICON/RT_GROUP_ICON and RT_VERSION resources.
#![allow(dead_code)]

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
const RT_MANIFEST: u16 = 24;

fn pad4(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(4) {
        v.push(0);
    }
}

fn utf16z(s: &str) -> Vec<u8> {
    s.encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// A `.res` file under construction.
pub struct Res {
    out: Vec<u8>,
    next_icon_id: u16,
}

impl Res {
    /// Start a `.res` file with the mandatory empty first entry.
    pub fn new() -> Self {
        let mut out = Vec::new();
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&32u32.to_le_bytes());
        out.extend_from_slice(&[0xff, 0xff, 0, 0, 0xff, 0xff, 0, 0]);
        out.extend_from_slice(&[0u8; 16]);
        Self {
            out,
            next_icon_id: 1,
        }
    }

    fn entry(&mut self, rtype: u16, id: u16, lang: u16, data: &[u8]) {
        let o = &mut self.out;
        o.extend_from_slice(&u32::try_from(data.len()).unwrap_or(0).to_le_bytes());
        o.extend_from_slice(&32u32.to_le_bytes());
        o.extend_from_slice(&[0xff, 0xff]);
        o.extend_from_slice(&rtype.to_le_bytes());
        o.extend_from_slice(&[0xff, 0xff]);
        o.extend_from_slice(&id.to_le_bytes());
        o.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
        o.extend_from_slice(&0x1030u16.to_le_bytes()); // MOVEABLE | PURE | DISCARDABLE
        o.extend_from_slice(&lang.to_le_bytes());
        o.extend_from_slice(&0u32.to_le_bytes()); // Version
        o.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
        o.extend_from_slice(data);
        pad4(o);
    }

    /// Add the application manifest (RT_MANIFEST id 1).
    pub fn manifest(&mut self, xml: &str) {
        self.entry(RT_MANIFEST, 1, 0x0409, xml.as_bytes());
    }

    /// Add an `.ico` file as icon group `group_id` (RT_ICON images + RT_GROUP_ICON directory).
    pub fn icon(&mut self, group_id: u16, ico: &[u8]) -> Result<(), String> {
        let rd16 = |o: usize| {
            ico.get(o..o + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .ok_or("ico truncated")
        };
        let rd32 = |o: usize| {
            ico.get(o..o + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .ok_or("ico truncated")
        };
        if rd16(0)? != 0 || rd16(2)? != 1 {
            return Err("not an .ico file".into());
        }
        let count = rd16(4)?;
        let mut group = Vec::new();
        group.extend_from_slice(&0u16.to_le_bytes());
        group.extend_from_slice(&1u16.to_le_bytes());
        group.extend_from_slice(&count.to_le_bytes());
        for i in 0..usize::from(count) {
            let e = 6 + 16 * i;
            let head = ico.get(e..e + 8).ok_or("ico truncated")?.to_vec();
            let size = rd32(e + 8)? as usize;
            let off = rd32(e + 12)? as usize;
            let data = ico
                .get(off..off + size)
                .ok_or("ico image out of range")?
                .to_vec();
            let id = self.next_icon_id;
            self.next_icon_id += 1;
            self.entry(RT_ICON, id, 0x0409, &data);
            group.extend_from_slice(&head); // width, height, colors, reserved, planes, bitcount
            group.extend_from_slice(&u32::try_from(size).unwrap_or(0).to_le_bytes());
            group.extend_from_slice(&id.to_le_bytes());
        }
        self.entry(RT_GROUP_ICON, group_id, 0x0409, &group);
        Ok(())
    }

    /// Add a VS_VERSIONINFO resource (RT_VERSION id 1).
    pub fn version(&mut self, version: (u16, u16, u16), strings: &[(&str, &str)]) {
        let (ma, mi, pa) = version;
        let ms = (u32::from(ma) << 16) | u32::from(mi);
        let ls = u32::from(pa) << 16;
        let mut fixed = Vec::new();
        for v in [
            0xFEEF_04BDu32,
            0x0001_0000,
            ms,
            ls,
            ms,
            ls,
            0x3F,
            0,
            0x0004_0004,
            1,
            0,
            0,
            0,
        ] {
            fixed.extend_from_slice(&v.to_le_bytes());
        }
        let strings: Vec<Vec<u8>> = strings
            .iter()
            .map(|(k, v)| {
                let value = utf16z(v);
                block(
                    k,
                    &value,
                    u16::try_from(value.len() / 2).unwrap_or(0),
                    1,
                    &[],
                )
            })
            .collect();
        let table = block("040904b0", &[], 0, 1, &strings);
        let sfi = block("StringFileInfo", &[], 0, 1, &[table]);
        let translation = [0x09u8, 0x04, 0xb0, 0x04];
        let var = block("Translation", &translation, 4, 0, &[]);
        let vfi = block("VarFileInfo", &[], 0, 1, &[var]);
        let root = block("VS_VERSION_INFO", &fixed, 52, 0, &[sfi, vfi]);
        self.entry(RT_VERSION, 1, 0x0409, &root);
    }

    /// Finish and return the bytes.
    pub fn finish(self) -> Vec<u8> {
        self.out
    }
}

/// One VS_VERSIONINFO-style block: header, key, value, children (each DWORD-aligned).
fn block(key: &str, value: &[u8], value_len: u16, wtype: u16, children: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![0u8; 6];
    b.extend_from_slice(&utf16z(key));
    pad4(&mut b);
    b.extend_from_slice(value);
    for c in children {
        pad4(&mut b);
        b.extend_from_slice(c);
    }
    let len = u16::try_from(b.len()).unwrap_or(u16::MAX);
    b[0..2].copy_from_slice(&len.to_le_bytes());
    b[2..4].copy_from_slice(&value_len.to_le_bytes());
    b[4..6].copy_from_slice(&wtype.to_le_bytes());
    b
}

/// The application manifest (prompt §4.6): PerMonitorV2, longPathAware, asInvoker, Windows 10/11,
/// optional SegmentHeap.
pub fn manifest_xml(name: &str, version: &str, segment_heap: bool) -> String {
    let heap = if segment_heap {
        "\n      <heapType xmlns=\"http://schemas.microsoft.com/SMI/2020/WindowsSettings\">SegmentHeap</heapType>"
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="{name}" version="{version}.0" processorArchitecture="amd64"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security><requestedPrivileges><requestedExecutionLevel level="asInvoker" uiAccess="false"/></requestedPrivileges></security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>{heap}
    </windowsSettings>
  </application>
</assembly>
"#
    )
}
