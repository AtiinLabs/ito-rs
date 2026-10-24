use crate::host;

/// Stores a plain setting value. Hosts keep it with the plugin's settings, which backups include;
/// use `set_secret` for credentials.
pub fn set(key: &str, value: &str) {
    let key_bytes = key.as_bytes();
    let val_bytes = value.as_bytes();
    unsafe {
        host::set(
            key_bytes.as_ptr() as i32,
            key_bytes.len() as i32,
            val_bytes.as_ptr() as i32,
            val_bytes.len() as i32,
        );
    }
}

/// Stores a value the plugin obtained itself, such as an access token, where the host keeps
/// credentials: never in backups or logs. `get` reads it back. It never replaces a secret the
/// user entered for the same key; `remove` deletes it.
pub fn set_secret(key: &str, value: &str) {
    let key_bytes = key.as_bytes();
    let val_bytes = value.as_bytes();
    unsafe {
        host::set_secret(
            key_bytes.as_ptr() as i32,
            key_bytes.len() as i32,
            val_bytes.as_ptr() as i32,
            val_bytes.len() as i32,
        );
    }
}

pub fn get(key: &str) -> crate::Result<Option<String>> {
    let key_bytes = key.as_bytes();
    let packed = unsafe {
        host::get(key_bytes.as_ptr() as i32, key_bytes.len() as i32)
    };
    
    unsafe { from_packed_option_ptr(packed) }
}

/// Removes a value the plugin stored with `set` or `set_secret`. Secrets the user entered are not
/// affected.
pub fn remove(key: &str) {
    let key_bytes = key.as_bytes();
    unsafe {
        host::remove(key_bytes.as_ptr() as i32, key_bytes.len() as i32);
    }
}

unsafe fn from_packed_option_ptr<T: serde::de::DeserializeOwned>(packed: i64) -> crate::Result<Option<T>> {
    if packed == 0 {
        return Ok(None);
    }
    let ptr = (packed >> 32) as i32;
    let len = (packed & 0xFFFFFFFF) as i32;
    
    let slice = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let mut res = None;
    if !slice.is_empty()
        && slice[0] == 1 {
            res = Some(postcard::from_bytes(&slice[1..]).map_err(crate::Error::Postcard)?);
        }
    
    unsafe { crate::ffi_alloc::dealloc(ptr as *mut u8, len as usize) };
    Ok(res)
}
