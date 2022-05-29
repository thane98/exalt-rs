use anyhow::{bail, Result};
use encoding_rs::SHIFT_JIS;

pub fn encode_shift_jis(text: &str) -> Result<Vec<u8>> {
    let (bytes, _, errors) = SHIFT_JIS.encode(text);
    if errors {
        bail!("Failed to encode string '{}' as SHIFT-JIS.", text);
    }
    Ok(bytes.into())
}
