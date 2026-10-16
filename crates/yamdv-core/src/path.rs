//! Path validation (S-FILE-04) and display form (R-OPEN-12). The platform layer normalizes with
//! `GetFullPathNameW` first and passes the result here.

/// Why a path was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathError {
    /// Empty path.
    Empty,
    /// Embedded NUL.
    Nul,
    /// Device namespace (`\\.\`, `\\?\GLOBALROOT`, `\??\`).
    DeviceNamespace,
    /// Reserved device name (`CON`, `NUL`, `COM1`, …) in any component.
    ReservedName,
    /// Alternate data stream (`:` beyond the drive letter).
    AlternateStream,
}

const RESERVED: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];

fn is_reserved(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches([' ', '.']);
    let u = stem.to_ascii_uppercase();
    if RESERVED.contains(&u.as_str()) {
        return true;
    }
    // COM1–9 / LPT1–9, including the superscript digits Windows also treats as devices.
    let b = u.as_bytes();
    (u.starts_with("COM") || u.starts_with("LPT"))
        && (b.len() == 4 && b.get(3).is_some_and(|d| (b'1'..=b'9').contains(d))
            || matches!(u.get(3..), Some("¹" | "²" | "³")))
}

/// Strips `\\?\` / `\\?\UNC\` for display (R-OPEN-12).
pub fn display_form(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path.to_string()
    }
}

/// Internal long-path form: `C:\…` → `\\?\C:\…`, `\\server\share\…` → `\\?\UNC\server\share\…`.
pub fn long_form(path: &str) -> String {
    if path.starts_with(r"\\?\") {
        path.to_string()
    } else if let Some(rest) = path.strip_prefix(r"\\") {
        format!(r"\\?\UNC\{rest}")
    } else {
        format!(r"\\?\{path}")
    }
}

/// Validates a full (already normalized) path. User-opened UNC paths are allowed.
pub fn validate(path: &str) -> Result<(), PathError> {
    if path.is_empty() {
        return Err(PathError::Empty);
    }
    if path.contains('\0') {
        return Err(PathError::Nul);
    }
    let p = path.replace('/', "\\");
    let upper = p.to_ascii_uppercase();
    if upper.starts_with(r"\\.\")
        || upper.starts_with(r"\??\")
        || upper.starts_with(r"\\?\GLOBALROOT")
    {
        return Err(PathError::DeviceNamespace);
    }
    let body = display_form(&p);
    // Skip the drive designator `C:` when checking for stream colons.
    let after_drive = match body.as_bytes() {
        [d, b':', ..] if d.is_ascii_alphabetic() => body.get(2..).unwrap_or(""),
        _ => body.as_str(),
    };
    if after_drive.contains(':') {
        return Err(PathError::AlternateStream);
    }
    if after_drive.split('\\').any(is_reserved) {
        return Err(PathError::ReservedName);
    }
    Ok(())
}

/// Lower-case extension (ASCII, R-I18N-04) of a path, without the dot.
pub fn extension(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or("");
    name.rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default()
}

/// File name component.
pub fn file_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s_file_04_rejects() {
        assert_eq!(validate(""), Err(PathError::Empty));
        assert_eq!(validate("C:\\a\0b.md"), Err(PathError::Nul));
        assert_eq!(
            validate(r"\\.\PhysicalDrive0"),
            Err(PathError::DeviceNamespace)
        );
        assert_eq!(
            validate(r"\\?\GLOBALROOT\Device\X"),
            Err(PathError::DeviceNamespace)
        );
        assert_eq!(validate(r"\??\C:\x"), Err(PathError::DeviceNamespace));
        assert_eq!(validate(r"C:\docs\con.md"), Err(PathError::ReservedName));
        assert_eq!(validate(r"C:\docs\Lpt9"), Err(PathError::ReservedName));
        assert_eq!(validate(r"C:\docs\COM¹.txt"), Err(PathError::ReservedName));
        assert_eq!(
            validate(r"C:\docs\a.md:secret"),
            Err(PathError::AlternateStream)
        );
        assert_eq!(
            validate(r"\\?\C:\docs\a.md:x"),
            Err(PathError::AlternateStream)
        );
    }

    #[test]
    fn s_file_04_accepts() {
        assert_eq!(validate(r"C:\docs\readme.md"), Ok(()));
        assert_eq!(validate(r"C:\docs\console.md"), Ok(()));
        assert_eq!(validate(r"C:\docs\COM0.md"), Ok(()));
        assert_eq!(validate(r"\\server\share\a.md"), Ok(()));
        assert_eq!(validate(r"\\?\UNC\server\share\a.md"), Ok(()));
        assert_eq!(validate(r"C:/x/y.md"), Ok(()));
    }

    #[test]
    fn r_open_12_forms() {
        assert_eq!(display_form(r"\\?\C:\a"), r"C:\a");
        assert_eq!(display_form(r"\\?\UNC\s\sh\a"), r"\\s\sh\a");
        assert_eq!(long_form(r"C:\a"), r"\\?\C:\a");
        assert_eq!(long_form(r"\\s\sh\a"), r"\\?\UNC\s\sh\a");
        assert_eq!(extension(r"C:\x\Readme.MD"), "md");
        assert_eq!(file_name(r"C:\x\Readme.MD"), "Readme.MD");
    }
}
