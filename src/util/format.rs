use std::ffi::CStr;

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
    unsafe {
        let pw = libc::getpwuid(uid);
        if !pw.is_null()
            && let Ok(name) = CStr::from_ptr((*pw).pw_name).to_str()
        {
            return name.to_string();
        }
    }

    uid.to_string()
}

pub fn group_name(gid: u32) -> String {
    unsafe {
        let gr = libc::getgrgid(gid);
        if !gr.is_null()
            && let Ok(name) = CStr::from_ptr((*gr).gr_name).to_str()
        {
            return name.to_string();
        }
    }

    gid.to_string()
}

pub fn size(size: u64) -> String {
    if size < 1024 {
        size.to_string()
    } else if size < 1024 * 1024 {
        format!("{:.1}K", size as f64 / 1024.0).replace(".0", "")
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1}M", size as f64 / 1048576.0).replace(".0", "")
    } else {
        format!("{:.1}G", size as f64 / 1073741824.0).replace(".0", "")
    }
}
