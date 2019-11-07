pub fn escape(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 4);

    for byte in bytes {
        output.push_str(&format!(r#"\{:02x}"#, byte));
    }

    output
}
