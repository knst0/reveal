use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use super::{Error, Outcome};
use crate::formats::FORMATS;

const APP_NAME: &str = "Reveal";
const PROG_PREFIX: &str = "Reveal";
const CAPABILITIES: &str = r"Software\Reveal\Capabilities";

type Hkey = isize;
const HKEY_CURRENT_USER: Hkey = -2147483647;
const KEY_WRITE: u32 = 0x20006;
const REG_SZ: u32 = 1;

unsafe extern "system" {
    fn RegCreateKeyExW(
        key: Hkey,
        sub_key: *const u16,
        reserved: u32,
        class: *mut u16,
        options: u32,
        desired: u32,
        security: *const core::ffi::c_void,
        result: *mut Hkey,
        disposition: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        key: Hkey,
        name: *const u16,
        reserved: u32,
        ty: u32,
        data: *const u8,
        len: u32,
    ) -> i32;
    fn RegCloseKey(key: Hkey) -> i32;
    fn SHChangeNotify(
        event: i32,
        flags: u32,
        item1: *const core::ffi::c_void,
        item2: *const core::ffi::c_void,
    );
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value).encode_wide().chain(std::iter::once(0)).collect()
}

fn set_value(path: &str, name: Option<&str>, value: &str) -> Result<(), Error> {
    let sub = wide(path);
    let mut key: Hkey = 0;
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_WRITE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        return Err(Error::Io(io::Error::from_raw_os_error(status)));
    }
    let data = wide(value);
    let len = std::mem::size_of_val(data.as_slice());
    let bytes = unsafe { std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), len) };
    let name_w = name.map(wide);
    let status = unsafe {
        RegSetValueExW(
            key,
            name_w.as_ref().map_or(std::ptr::null(), |n| n.as_ptr()),
            0,
            REG_SZ,
            bytes.as_ptr(),
            bytes.len() as u32,
        )
    };
    unsafe { RegCloseKey(key) };
    if status != 0 {
        return Err(Error::Io(io::Error::from_raw_os_error(status)));
    }
    Ok(())
}

fn exe() -> Result<PathBuf, Error> {
    std::env::current_exe().map_err(Error::Io)
}

pub fn register() -> Result<Outcome, Error> {
    let exe = exe()?;
    let exe = exe.to_string_lossy().to_string();
    let open_command = format!("\"{exe}\" \"%1\"");
    let icon = format!("\"{exe}\",0");

    set_value(CAPABILITIES, Some("ApplicationName"), APP_NAME)?;
    set_value(CAPABILITIES, Some("ApplicationDescription"), "A fast image viewer")?;

    let mut registered = 0;
    for format in FORMATS {
        let prog_id = format!("{PROG_PREFIX}.{}", format.extension);

        set_value(&format!(r"Software\Classes\{prog_id}"), None, format.description)?;
        set_value(&format!(r"Software\Classes\{prog_id}\DefaultIcon"), None, &icon)?;
        set_value(&format!(r"Software\Classes\{prog_id}\shell\open\command"), None, &open_command)?;

        set_value(
            &format!(r"Software\Classes\.{}", format.extension),
            Some("Content Type"),
            format.mime,
        )?;
        set_value(
            &format!(r"Software\Classes\.{}\OpenWithProgids", format.extension),
            Some(&prog_id),
            "",
        )?;

        set_value(
            &format!(r"{CAPABILITIES}\FileAssociations"),
            Some(&format!(".{}", format.extension)),
            &prog_id,
        )?;

        set_value(
            r"Software\Classes\Applications\reveal.exe\SupportedTypes",
            Some(&format!(".{}", format.extension)),
            "",
        )?;
        registered += 1;
    }

    set_value(r"Software\Classes\Applications\reveal.exe\shell\open\command", None, &open_command)?;
    set_value(r"Software\Classes\Applications\reveal.exe", Some("FriendlyAppName"), APP_NAME)?;

    set_value(r"Software\RegisteredApplications", Some(APP_NAME), CAPABILITIES)?;

    unsafe { SHChangeNotify(0x08000000, 0x0000, std::ptr::null(), std::ptr::null()) };

    Ok(Outcome {
        registered,
        needs_user_action: Some(
            "Reveal is now offered for these formats. Windows requires you to confirm the default in Settings \u{203a} Apps \u{203a} Default apps."
                .to_owned(),
        ),
    })
}
