pub fn sanitize(text: &str, mut filter: impl FnMut(char) -> bool) -> String {
    text.chars().filter(|&ch| !ch.is_control() || filter(ch)).collect()
}
