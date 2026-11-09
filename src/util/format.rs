use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr;

pub fn permissions(mode: u32) -> String {
    let rwx = |mode, r, w, x| {
        format!(
            "{}{}{}",
            if mode & r != 0 { 'r' } else { '-' },
            if mode & w != 0 { 'w' } else { '-' },
            if mode & x != 0 { 'x' } else { '-' }
        )
    };

    format!(
        "{}{}{}",
        rwx(mode, 0o400, 0o200, 0o100),
        rwx(mode, 0o040, 0o020, 0o010),
        rwx(mode, 0o004, 0o002, 0o001)
    )
}

pub fn user_name(uid: u32) -> String {
    let mut buffer = vec![0u8; 1024];
    loop {
        let mut entry = MaybeUninit::<libc::passwd>::uninit();
        let mut result = ptr::null_mut();

        let status = unsafe {
            libc::getpwuid_r(
                uid,
                entry.as_mut_ptr(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &raw mut result,
            )
        };

        if status == libc::ERANGE && self::grow_buffer(&mut buffer) {
            continue;
        }

        if status == 0 && !result.is_null() {
            let name = unsafe { (*result).pw_name };
            if !name.is_null()
                && let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str()
            {
                return name.to_owned();
            }
        }

        return uid.to_string();
    }
}

pub fn group_name(gid: u32) -> String {
    let mut buffer = vec![0u8; 1024];
    loop {
        let mut entry = MaybeUninit::<libc::group>::uninit();
        let mut result = ptr::null_mut();

        let status = unsafe {
            libc::getgrgid_r(
                gid,
                entry.as_mut_ptr(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &raw mut result,
            )
        };

        if status == libc::ERANGE && self::grow_buffer(&mut buffer) {
            continue;
        }

        if status == 0 && !result.is_null() {
            let name = unsafe { (*result).gr_name };
            if !name.is_null()
                && let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str()
            {
                return name.to_owned();
            }
        }

        return gid.to_string();
    }
}

pub fn size(bytes: u64) -> String {
    if bytes < 1024 {
        bytes.to_string()
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0).replace(".0", "")
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0)).replace(".0", "")
    } else {
        format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0)).replace(".0", "")
    }
}

fn grow_buffer(buffer: &mut Vec<u8>) -> bool {
    let Some(size) = buffer.len().checked_mul(2) else {
        return false;
    };

    if buffer.try_reserve_exact(buffer.len()).is_err() {
        return false;
    }

    buffer.resize(size, 0);

    true
}
