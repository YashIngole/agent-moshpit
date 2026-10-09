//! Windows notifications that can be clicked. The notification plugin's own
//! cannot say what a click opens, so the toast is made here: a click opens the
//! office's address for that desk (see `link.rs`), which Windows hands to the app.
//! While the app is running, the click is also heard directly.

use windows::core::HSTRING;
use windows::Data::Xml::Dom::XmlDocument;
use windows::Foundation::TypedEventHandler;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

/// The name Windows shows notifications under for a program that was not installed
/// with a Start menu shortcut of its own (a build being worked on, a named test office).
const POWERSHELL: &str = "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";

/// The app id to show notifications under: the installed app's own, else PowerShell's,
/// as the notification plugin decides it.
pub fn app_id(identifier: &str, named: bool) -> String {
    let exe = std::env::current_exe().ok();
    let dir = exe.as_deref().and_then(std::path::Path::parent).map(|d| d.to_string_lossy().to_lowercase()).unwrap_or_default();
    let built_here = dir.ends_with(r"\target\debug") || dir.ends_with(r"\target\release");
    if built_here || named {
        POWERSHELL.to_string()
    } else {
        identifier.to_string()
    }
}

/// What a toast says, and what clicking it opens: written as Windows reads it.
pub fn xml(title: &str, body: &str, opens: Option<&str>) -> String {
    let launch = opens.map(|address| format!(" launch=\"{}\" activationType=\"protocol\"", escape(address))).unwrap_or_default();
    format!(
        "<toast{launch}><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        escape(title),
        escape(body)
    )
}

fn short(text: &str) -> String {
    text.chars().take(64).collect()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Show a toast. `opens` is the address a click opens, when the scheme is known to
/// Windows; `clicked` is called when the click is heard in this process.
///
/// `group` is the office it is from and `tag` what it is about (a desk): a newer toast
/// about the same desk takes the place of the older one in the notification centre,
/// so it never fills with what is no longer true.
pub fn show(app_id: &str, group: &str, tag: &str, title: &str, body: &str, opens: Option<&str>, clicked: impl Fn() + Send + 'static) -> windows::core::Result<()> {
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml(title, body, opens)))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    // Windows takes at most 64 characters for each; a desk's id and an address's scheme are well under.
    toast.SetGroup(&HSTRING::from(short(group)))?;
    toast.SetTag(&HSTRING::from(short(tag)))?;
    toast.Activated(&TypedEventHandler::<ToastNotification, windows::core::IInspectable>::new(move |_, _| {
        clicked();
        Ok(())
    }))?;
    // Windows holds the toast, and with it the handler, for as long as it is shown or kept.
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?.Show(&toast)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toast_opens_its_desk_and_says_only_what_it_was_given() {
        let xml = xml("Fix <the> total needs you", "Asked: \"rm -rf dist\" & go?", Some("agent-moshpit://desk/0a1b"));
        assert!(xml.starts_with("<toast launch=\"agent-moshpit://desk/0a1b\" activationType=\"protocol\">"), "{xml}");
        assert!(xml.contains("<text>Fix &lt;the&gt; total needs you</text>"));
        assert!(xml.contains("<text>Asked: &quot;rm -rf dist&quot; &amp; go?</text>"));
        // With nowhere to go, a click only does what Windows does by itself.
        assert!(super::xml("a", "b", None).starts_with("<toast><visual>"));
        // It is a document Windows can read.
        let doc = XmlDocument::new().unwrap();
        doc.LoadXml(&HSTRING::from(xml)).unwrap();
    }
}
