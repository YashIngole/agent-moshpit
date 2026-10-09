//! The office's own address: `agent-moshpit://desk/<id>` brings the office to the
//! front with that desk's terminal open. A notification opens it when it is
//! clicked: Windows starts the app with the address, and a second launch hands it
//! to the office already running (see the single-instance plugin in `lib.rs`).
//!
//! A named instance has an address of its own, `agent-moshpit-<name>://`, so a test
//! never sends a click to the office the user has open.

/// The scheme this office answers to.
pub fn scheme(instance: Option<&str>) -> String {
    match instance {
        Some(name) => format!("agent-moshpit-{}", name.to_ascii_lowercase()),
        None => "agent-moshpit".to_string(),
    }
}

/// The address that opens a desk's terminal. Only Windows notifications carry one so far.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn desk(scheme: &str, id: &str) -> String {
    format!("{scheme}://desk/{id}")
}

/// The address that only brings the office to the front.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn office(scheme: &str) -> String {
    format!("{scheme}://open")
}

/// What a launch was asked to open, from its words: `Some(Some(id))` a desk,
/// `Some(None)` the office, `None` when it was given no address of this office's.
pub fn asked(args: &[String], scheme: &str) -> Option<Option<String>> {
    let prefix = format!("{scheme}://");
    let address = args.iter().find_map(|arg| arg.get(..prefix.len()).filter(|p| p.eq_ignore_ascii_case(&prefix)).map(|_| &arg[prefix.len()..]))?;
    let path = address.trim_end_matches('/');
    // A desk's id is what the engine makes: hex digits. Anything else opens only the office.
    let id = path.strip_prefix("desk/").filter(|id| !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_hexdigit()));
    Some(id.map(str::to_string))
}

/// Tell Windows that this program opens the scheme, for this user only. Written each
/// time the office starts, so it follows the program if it moves. False if it could not.
#[cfg(windows)]
pub fn register(scheme: &str, exe: &std::path::Path, instance: Option<&str>) -> bool {
    let exe = exe.to_string_lossy();
    let named = instance.map(|name| format!(" --instance {name}")).unwrap_or_default();
    let base = format!(r"Software\Classes\{scheme}");
    set(&base, None, "URL:Agent Moshpit")
        && set(&base, Some("URL Protocol"), "")
        && set(&format!(r"{base}\DefaultIcon"), None, &format!("\"{exe}\",0"))
        && set(&format!(r"{base}\shell\open\command"), None, &format!("\"{exe}\"{named} \"%1\""))
}

#[cfg(windows)]
fn set(path: &str, name: Option<&str>, value: &str) -> bool {
    use windows_sys::Win32::System::Registry::{RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ};
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (path, name, data) = (wide(path), name.map(wide), wide(value));
    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        let made = RegCreateKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, std::ptr::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, std::ptr::null(), &mut key, std::ptr::null_mut());
        if made != 0 {
            return false;
        }
        let name = name.as_ref().map_or(std::ptr::null(), |n| n.as_ptr());
        let written = RegSetValueExW(key, name, 0, REG_SZ, data.as_ptr().cast(), (data.len() * 2) as u32);
        RegCloseKey(key);
        written == 0
    }
}

#[cfg(not(windows))]
pub fn register(_scheme: &str, _exe: &std::path::Path, _instance: Option<&str>) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_office_has_an_address_of_its_own() {
        assert_eq!(scheme(None), "agent-moshpit");
        assert_eq!(scheme(Some("e2e")), "agent-moshpit-e2e");
        assert_eq!(desk("agent-moshpit", "0a1b2c3d4e5f6071"), "agent-moshpit://desk/0a1b2c3d4e5f6071");
        assert_eq!(office("agent-moshpit-ux"), "agent-moshpit-ux://open");
    }

    #[test]
    fn a_launch_says_which_desk_to_open() {
        let words = |list: &[&str]| list.iter().map(|w| w.to_string()).collect::<Vec<_>>();
        let exe = r"C:\Program Files\Agent Moshpit\agent-moshpit.exe";
        assert_eq!(asked(&words(&[exe, "agent-moshpit://desk/0a1b2c3d4e5f6071"]), "agent-moshpit"), Some(Some("0a1b2c3d4e5f6071".into())));
        // Windows may add a slash, or change the case of the scheme.
        assert_eq!(asked(&words(&[exe, "Agent-Moshpit://desk/0a1b/"]), "agent-moshpit"), Some(Some("0a1b".into())));
        assert_eq!(asked(&words(&[exe, "agent-moshpit://open"]), "agent-moshpit"), Some(None));
        // Not a desk id: only the office is brought forward.
        assert_eq!(asked(&words(&[exe, "agent-moshpit://desk/..%2F..%2Fcalc"]), "agent-moshpit"), Some(None));
        // Another office's address, or none at all, asks for nothing here.
        assert_eq!(asked(&words(&[exe, "agent-moshpit-e2e://desk/0a1b"]), "agent-moshpit"), None);
        assert_eq!(asked(&words(&[exe, "--hidden"]), "agent-moshpit"), None);
    }
}
